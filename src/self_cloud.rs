//! «Своё облако» actions, as in legacy/src/bridge/self-cloud.js: Sync (merge
//! both sides), Upload (replace the cloud snapshot) and Download (replace the
//! local data after an encrypted backup). Conflicts are reported, never
//! resolved without the user's choice.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::cloud::{self, Condition};
use crate::portable::{self, VaultDevice};
use crate::sync::{self, ConflictInfo, Prefer};
use crate::vault::Vault;

/// Legacy-profile data has no passwords or keys; sending it would erase them in the cloud.
pub const SECRETS_MISSING: &str =
    "Данные прочитаны из профиля старого клиента без паролей и ключей. Сначала нажмите «Получить», чтобы взять данные из облака.";
pub const NOT_CONFIGURED: &str = "Сначала настройте своё облако";
const BACKUPS_KEPT: usize = 10;

#[derive(Debug, PartialEq)]
pub enum Report {
    /// Done; the text says what moved.
    Done(String),
    /// The same records changed on both sides: the user picks a side.
    Conflicts(Vec<ConflictInfo>),
}

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// `<data>/termian-native/self-cloud-backups`.
pub fn default_backup_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("termian-native").join("self-cloud-backups")
}

/// Encrypted copy of the local data before an import, keeping the last ten.
fn backup(dir: &Path, snapshot: &Value, passphrase: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%3fZ");
    std::fs::write(dir.join(format!("{stamp}.json")), cloud::seal(snapshot, passphrase)?).map_err(|e| e.to_string())?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
    files.sort();
    files.reverse();
    for old in files.into_iter().skip(BACKUPS_KEPT) {
        let _ = std::fs::remove_file(old);
    }
    Ok(())
}

fn configured(vault: &Vault) -> Result<(cloud::CloudConfig, cloud::Cloud), String> {
    let config = vault.cloud.clone().ok_or(NOT_CONFIGURED)?;
    let client = config.validate()?;
    Ok((config, client))
}

/// Sync: merge, retrying twice when another device uploaded meanwhile.
pub fn sync(vault: &mut Vault, prefer: Option<Prefer>, backups: &Path) -> Result<Report, String> {
    let (config, client) = configured(vault)?;
    if vault.secrets_missing {
        return Err(SECRETS_MISSING.into());
    }
    let server = sync::server_key(&config.url);
    let mut attempt = 0;
    loop {
        let saved = vault.sync_state.clone();
        let passphrase = config.passphrase.clone();
        let result = {
            let mut device = VaultDevice { vault: &mut *vault, now: now(), backup: Box::new(move |s| backup(backups, s, &passphrase)) };
            sync::sync_once(saved, &server, prefer, &mut device, &client, &now())
        };
        match result {
            Ok(outcome) if !outcome.conflicts.is_empty() => {
                // New ID matches are kept so the next attempt starts from them.
                if outcome.state.is_some() {
                    vault.sync_state = outcome.state;
                }
                return Ok(Report::Conflicts(outcome.conflicts));
            }
            Ok(outcome) => {
                vault.sync_state = outcome.state;
                let mut parts = Vec::new();
                if outcome.from_local > 0 {
                    parts.push(format!("отправлено изменений: {}", outcome.from_local));
                }
                if outcome.from_remote > 0 {
                    parts.push(format!("получено изменений: {}", outcome.from_remote));
                }
                let text = if parts.is_empty() { "Данные уже синхронизированы".to_owned() } else { format!("Синхронизировано: {}", parts.join(", ")) };
                return Ok(Report::Done(text));
            }
            Err(error) if attempt < 2 && error.contains(cloud::REVISION_CHANGED) => attempt += 1,
            Err(error) => return Err(config.redact(&error)),
        }
    }
}

/// «Отправить»: the cloud snapshot is replaced with this device's data.
pub fn upload(vault: &mut Vault) -> Result<Report, String> {
    let (config, client) = configured(vault)?;
    if vault.secrets_missing {
        return Err(SECRETS_MISSING.into());
    }
    let mut ids = vault.sync_state.as_ref().filter(|s| s.server == sync::server_key(&config.url)).map(|s| s.ids.clone()).unwrap_or_default();
    let exported = sync::validate_snapshot(&portable::export(vault, &now()))?;
    let snapshot = sync::validate_snapshot(&sync::to_sync_form(&exported, &mut ids))?;
    client.upload(&snapshot, Condition::Any).map_err(|e| config.redact(&e))?;
    vault.sync_state = Some(sync::State { server: sync::server_key(&config.url), local: sync::digests_of(Some(&snapshot)), remote: sync::digests_of(Some(&snapshot)), ids });
    Ok(Report::Done("Данные загружены".into()))
}

/// «Получить»: the local data is replaced with the cloud snapshot, after a backup.
pub fn download(vault: &mut Vault, backups: &Path) -> Result<Report, String> {
    let (config, client) = configured(vault)?;
    let remote = client.fetch().map_err(|e| config.redact(&e))?.ok_or("В облаке ещё нет данных")?;
    let snapshot = sync::validate_snapshot(&remote.snapshot)?;
    backup(backups, &portable::export(vault, &now()), &config.passphrase)?;
    let pairs = portable::import(vault, &snapshot)?;
    let mut ids = sync::ids_from_import(&pairs);
    let exported = sync::validate_snapshot(&portable::export(vault, &now()))?;
    sync::match_ids(&exported, Some(&snapshot), &mut ids);
    let local = sync::to_sync_form(&exported, &mut ids);
    vault.sync_state = Some(sync::State { server: sync::server_key(&config.url), local: sync::digests_of(Some(&local)), remote: sync::digests_of(Some(&snapshot)), ids });
    Ok(Report::Done("Данные загружены и применены".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Host;

    fn host(id: u64, label: &str) -> Host {
        Host { id, label: label.into(), address: format!("{label}.lan"), os: None, group: None, username: None, tags: vec![], port: None, password: None, key: None, identity: None }
    }

    fn vault(url: &str, hosts: &[&str]) -> Vault {
        let mut vault = Vault { cloud: Some(crate::cloud::tests::config(url)), ..Vault::default() };
        for (i, label) in hosts.iter().enumerate() {
            vault.hosts.push(host(i as u64 + 1, label));
        }
        vault
    }

    fn labels(vault: &Vault) -> Vec<String> {
        let mut l: Vec<String> = vault.hosts.iter().map(|h| h.label.clone()).collect();
        l.sort();
        l
    }

    #[test]
    fn sync_merges_and_reports_what_moved() {
        let (url, _) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let (mut a, mut b) = (vault(&url, &["web"]), vault(&url, &["db"]));
        assert_eq!(sync(&mut a, None, backups.path()).unwrap(), Report::Done("Синхронизировано: отправлено изменений: 2".into()));
        assert!(matches!(sync(&mut b, None, backups.path()).unwrap(), Report::Done(text) if text.contains("получено изменений: 2")));
        assert_eq!(labels(&b), ["db", "web"]);
        assert_eq!(std::fs::read_dir(backups.path()).unwrap().count(), 1, "a backup before the import");
        sync(&mut a, None, backups.path()).unwrap();
        assert_eq!(sync(&mut a, None, backups.path()).unwrap(), Report::Done("Данные уже синхронизированы".into()));
    }

    #[test]
    fn conflicts_wait_for_the_users_choice() {
        let (url, _) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let (mut a, mut b) = (vault(&url, &["web"]), vault(&url, &[]));
        sync(&mut a, None, backups.path()).unwrap();
        sync(&mut b, None, backups.path()).unwrap();
        a.hosts[0].address = "a.lan".into();
        b.hosts[0].address = "b.lan".into();
        sync(&mut a, None, backups.path()).unwrap();
        let Report::Conflicts(conflicts) = sync(&mut b, None, backups.path()).unwrap() else { panic!("expected a conflict") };
        assert_eq!(conflicts[0].label, "web");
        assert_eq!(b.hosts[0].address, "b.lan", "nothing applied");
        sync(&mut b, Some(Prefer::Remote), backups.path()).unwrap();
        assert_eq!(b.hosts[0].address, "a.lan");
    }

    #[test]
    fn legacy_profile_data_without_secrets_is_never_sent() {
        let (url, stored) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let mut full = vault(&url, &["web"]);
        full.hosts[0].password = Some("secret".into());
        upload(&mut full).unwrap();
        let mut legacy = Vault { secrets_missing: true, ..vault(&url, &["web"]) };
        assert_eq!(sync(&mut legacy, None, backups.path()).unwrap_err(), SECRETS_MISSING);
        assert_eq!(upload(&mut legacy).unwrap_err(), SECRETS_MISSING);
        let before = stored.lock().unwrap().clone();
        download(&mut legacy, backups.path()).unwrap();
        assert_eq!(stored.lock().unwrap().clone(), before, "the cloud is untouched");
        assert_eq!(legacy.hosts[0].password.as_deref(), Some("secret"));
        assert!(!legacy.secrets_missing);
        assert!(sync(&mut legacy, None, backups.path()).is_ok(), "sync is allowed after a download");
    }

    #[test]
    fn upload_replaces_and_backups_are_capped() {
        let (url, _) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let mut a = vault(&url, &["web"]);
        assert!(matches!(download(&mut a, backups.path()), Err(e) if e == "В облаке ещё нет данных"));
        upload(&mut a).unwrap();
        let mut b = vault(&url, &["other"]);
        for _ in 0..12 {
            download(&mut b, backups.path()).unwrap();
        }
        assert_eq!(labels(&b), ["web"]);
        assert_eq!(std::fs::read_dir(backups.path()).unwrap().count(), BACKUPS_KEPT);
        assert_eq!(sync(&mut Vault::default(), None, backups.path()).unwrap_err(), NOT_CONFIGURED);
    }
}
