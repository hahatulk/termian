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
    "Не все пароли и ключи из профиля старого клиента удалось прочитать. Сначала нажмите «Получить», чтобы взять данные из облака.";
pub const NOT_CONFIGURED: &str = "Сначала настройте своё облако";
const BACKUPS_KEPT: usize = 10;

/// What the sync dialog shows before anything is changed, as the legacy check.
#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    Unconfigured,
    /// The cloud has no snapshot yet.
    Empty,
    Synced,
    /// Something to send, something to receive, or both.
    Changes(Vec<sync::Change>),
    Conflicts(Vec<ConflictInfo>),
}

impl Preview {
    /// The headline of the legacy status panel.
    pub fn headline(&self) -> &'static str {
        match self {
            Preview::Unconfigured => "Облако не настроено",
            Preview::Empty => "Снимок в облаке пуст",
            Preview::Synced => "Данные клиента и сервера совпадают",
            Preview::Changes(changes) => match (changes.iter().any(|c| c.upload), changes.iter().any(|c| !c.upload)) {
                (true, false) => "Есть новые локальные изменения → требуется отправка в облако",
                (false, true) => "В облаке появились новые данные → требуется получение на клиент",
                _ => "На обеих сторонах есть изменения — возможен конфликт",
            },
            Preview::Conflicts(_) => "На обеих сторонах есть изменения — возможен конфликт",
        }
    }
}

/// The legacy `sync-change-groups.cjs` grouping: by direction and action,
/// then by record type, keeping every record.
#[derive(Debug, PartialEq)]
pub struct ChangeGroup {
    /// `На устройстве`, `В облаке` or `Обе стороны`.
    pub direction: &'static str,
    /// `Добавлено`, `Изменено`, `Удалено` or `Конфликт`.
    pub action: &'static str,
    pub total: usize,
    pub categories: Vec<ChangeCategory>,
}

#[derive(Debug, PartialEq)]
pub struct ChangeCategory {
    /// Type name in the right plural form, as `SSH-ключа`.
    pub label: &'static str,
    /// `«name»` per record, with what changed when known.
    pub records: Vec<(String, String)>,
}

impl ChangeGroup {
    /// `Изменено · 2 записи`.
    pub fn title(&self) -> String {
        format!("{} · {}", self.action, count_label(self.total))
    }
}

/// Russian plural form index: 1 запись, 2 записи, 5 записей.
fn plural(n: usize) -> usize {
    let (mod100, mod10) = (n % 100, n % 10);
    if (11..=14).contains(&mod100) {
        2
    } else if mod10 == 1 {
        0
    } else if (2..=4).contains(&mod10) {
        1
    } else {
        2
    }
}

pub fn count_label(n: usize) -> String {
    format!("{n} {}", ["запись", "записи", "записей"][plural(n)])
}

/// `5 записей · 2 группы`, the legacy overview line.
pub fn overview(groups: &[ChangeGroup]) -> String {
    let total: usize = groups.iter().map(|g| g.total).sum();
    format!("{} · {} {}", count_label(total), groups.len(), ["группа", "группы", "групп"][plural(groups.len())])
}

fn collection_label(kind: &str, n: usize) -> &'static str {
    let forms: [&'static str; 3] = match kind {
        "groups" => ["Группа", "Группы", "Групп"],
        "hosts" => ["Хост", "Хоста", "Хостов"],
        "identities" => ["Идентификатор", "Идентификатора", "Идентификаторов"],
        "sshKeys" => ["SSH-ключ", "SSH-ключа", "SSH-ключей"],
        "snippets" => ["Сниппет", "Сниппета", "Сниппетов"],
        "snippetPackages" => ["Пакет сниппетов", "Пакета сниппетов", "Пакетов сниппетов"],
        "portForwardingRules" => ["Проброс портов", "Правила проброса портов", "Правил проброса портов"],
        "knownHosts" => ["Известный хост", "Известных хоста", "Известных хостов"],
        "sshConfigs" => ["SSH-конфигурация", "SSH-конфигурации", "SSH-конфигураций"],
        "telnetConfigs" => ["Telnet-конфигурация", "Telnet-конфигурации", "Telnet-конфигураций"],
        "proxies" => ["Прокси", "Прокси", "Прокси"],
        "workspaceTemplates" => ["Шаблон рабочей области", "Шаблона рабочей области", "Шаблонов рабочей области"],
        "settings" => ["Настройки", "Настройки", "Настройки"],
        _ => ["Запись", "Записи", "Записей"],
    };
    forms[plural(n)]
}

fn group(items: impl IntoIterator<Item = (&'static str, &'static str, String, String, String)>) -> Vec<ChangeGroup> {
    let mut groups: Vec<(ChangeGroup, Vec<(String, Vec<(String, String)>)>)> = Vec::new();
    for (direction, action, kind, label, detail) in items {
        let index = match groups.iter().position(|(g, _)| g.direction == direction && g.action == action) {
            Some(i) => i,
            None => {
                groups.push((ChangeGroup { direction, action, total: 0, categories: Vec::new() }, Vec::new()));
                groups.len() - 1
            }
        };
        let (group, kinds) = &mut groups[index];
        group.total += 1;
        let label = if label.is_empty() { "ранее сохранённая запись".to_owned() } else { label };
        let detail = detail.trim().trim_start_matches('(').trim_end_matches(')').to_owned();
        match kinds.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, records)) => records.push((label, detail)),
            None => kinds.push((kind, vec![(label, detail)])),
        }
    }
    groups
        .into_iter()
        .map(|(mut group, kinds)| {
            group.categories = kinds.into_iter().map(|(kind, records)| ChangeCategory { label: collection_label(&kind, records.len()), records }).collect();
            group
        })
        .collect()
}

pub fn group_changes(changes: &[sync::Change]) -> Vec<ChangeGroup> {
    group(changes.iter().map(|c| {
        let action = match c.action {
            "added" => "Добавлено",
            "removed" => "Удалено",
            _ => "Изменено",
        };
        (if c.upload { "На устройстве" } else { "В облаке" }, action, c.kind.clone(), c.label.clone(), String::new())
    }))
}

pub fn group_conflicts(conflicts: &[ConflictInfo]) -> Vec<ChangeGroup> {
    group(conflicts.iter().map(|c| ("Обе стороны", "Конфликт", c.kind.clone(), c.label.clone(), c.detail.clone())))
}

impl Preview {
    pub fn groups(&self) -> Vec<ChangeGroup> {
        match self {
            Preview::Changes(changes) => group_changes(changes),
            Preview::Conflicts(conflicts) => group_conflicts(conflicts),
            _ => Vec::new(),
        }
    }

    /// Records to send and to receive, for the header badges.
    pub fn counts(&self) -> (usize, usize) {
        match self {
            Preview::Changes(changes) => (changes.iter().filter(|c| c.upload).count(), changes.iter().filter(|c| !c.upload).count()),
            _ => (0, 0),
        }
    }
}

/// Wrong token or passphrase: automatic checks pause so they do not extend a
/// server lockout (legacy `isCredentialError`).
pub fn is_credential_error(message: &str) -> bool {
    let message = message.to_lowercase();
    ["токен доступа", "отклонил авторизацию", "неверных попыток авторизации", "неверный пароль шифрования", "неверных паролей"].iter().any(|p| message.contains(p))
}

/// The sync state behind the header buttons.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Checking,
    Syncing,
    Checked(Preview),
    Error { message: String, credentials: bool },
}

impl Default for Status {
    fn default() -> Self {
        Status::Checked(Preview::Unconfigured)
    }
}

pub const TONE_OK: u32 = 0x3dcc94;
pub const TONE_ERROR: u32 = 0xe66b6b;
pub const TONE_WARNING: u32 = 0xf2b34b;
const TONE_PENDING: u32 = 0xffbd59;
const TONE_IDLE: u32 = 0x8d91a5;

/// The legacy header: the sync icon keeps the last settled state while a
/// background check runs, so it does not flicker.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Header {
    pub status: Status,
    settled: Status,
    /// Unix seconds of the last successful sync or replace.
    pub synced_at: Option<i64>,
}

impl Header {
    pub fn set(&mut self, status: Status) {
        if !matches!(status, Status::Checking | Status::Syncing) {
            self.settled = status.clone();
        }
        self.status = status;
    }

    /// Forgets the state after new settings, keeping the last sync time.
    pub fn reset(&mut self) {
        *self = Header { synced_at: self.synced_at, ..Header::default() };
    }

    pub fn syncing(&self) -> bool {
        self.status == Status::Syncing
    }

    /// Automatic checks stay off after a credential error until the user acts.
    pub fn paused(&self) -> bool {
        matches!(self.status, Status::Error { credentials: true, .. })
    }

    /// Tooltip of the sync button; `time` formats `synced_at`.
    pub fn sync_title(&self, time: impl Fn(i64) -> String) -> String {
        match &self.settled {
            _ if self.syncing() => "Синхронизация…".into(),
            Status::Checked(Preview::Unconfigured) => "Синхронизация — облако не настроено".into(),
            Status::Checked(Preview::Synced | Preview::Empty) => {
                format!("Синхронизировано{}", self.synced_at.map(|t| format!(" · {}", time(t))).unwrap_or_default())
            }
            Status::Error { message, .. } => format!("Ошибка синхронизации: {message}"),
            Status::Checked(Preview::Conflicts(conflicts)) => format!("Конфликты синхронизации: {}", conflicts.len()),
            _ => "Есть что синхронизировать".into(),
        }
    }

    /// `✓` or `!` on the sync button, with its colour; none while idle or syncing.
    pub fn sync_badge(&self) -> Option<(&'static str, u32)> {
        match &self.settled {
            _ if self.syncing() => None,
            Status::Checked(Preview::Unconfigured) => None,
            Status::Checked(Preview::Synced | Preview::Empty) => Some(("✓", TONE_OK)),
            Status::Error { .. } => Some(("!", TONE_ERROR)),
            _ => Some(("!", TONE_WARNING)),
        }
    }

    /// Count and colour of the upload (`true`) or download dot.
    pub fn indicator(&self, upload: bool) -> (usize, u32) {
        let (up, down) = match &self.status {
            Status::Checked(preview) => preview.counts(),
            _ => (0, 0),
        };
        let count = if upload { up } else { down };
        let tone = match &self.status {
            Status::Error { .. } => TONE_ERROR,
            Status::Checked(Preview::Conflicts(_)) => TONE_WARNING,
            _ if count > 0 => TONE_PENDING,
            Status::Checked(Preview::Synced | Preview::Empty) => TONE_OK,
            _ => TONE_IDLE,
        };
        (count, tone)
    }
}

/// Compares this device with the cloud without changing either side.
pub fn preview(vault: &Vault) -> Result<Preview, String> {
    let Some(config) = vault.cloud.clone().filter(|c| c.validate().is_ok()) else { return Ok(Preview::Unconfigured) };
    let client = config.validate()?;
    let Some(remote) = client.fetch().map_err(|e| config.redact(&e))? else { return Ok(Preview::Empty) };
    let remote = sync::validate_snapshot(&remote.snapshot)?;
    let server = sync::server_key(&config.url);
    let saved = vault.sync_state.clone().filter(|s| s.server == server);
    let mut ids = saved.as_ref().map(|s| s.ids.clone()).unwrap_or_default();
    let exported = sync::validate_snapshot(&portable::export(vault, &now()))?;
    sync::match_ids(&exported, Some(&remote), &mut ids);
    let local = sync::to_sync_form(&exported, &mut ids);
    let base = saved.map(|s| sync::Base { local: s.local, remote: s.remote });
    let merged = sync::merge_snapshots(&local, Some(&remote), base.as_ref(), None, &now());
    Ok(if !merged.conflicts.is_empty() {
        Preview::Conflicts(merged.conflicts)
    } else if merged.changes.is_empty() {
        Preview::Synced
    } else {
        Preview::Changes(merged.changes)
    })
}

/// What the settings form sends: secrets are write-only, an empty field keeps the saved one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConfigChanges {
    pub url: String,
    pub token: String,
    pub passphrase: String,
    /// Empty turns Basic auth off.
    pub basic_user: String,
    pub basic_password: String,
}

/// Saves the settings the way the legacy form does; the result must validate.
pub fn save_config(vault: &mut Vault, changes: ConfigChanges) -> Result<(), String> {
    let saved = vault.cloud.clone().unwrap_or(cloud::CloudConfig { auto_sync: true, ..cloud::CloudConfig::default() });
    let keep = |new: String, old: String| if new.is_empty() { old } else { new };
    let basic_user = changes.basic_user.trim().to_owned();
    let config = cloud::CloudConfig {
        url: changes.url.trim().to_owned(),
        token: keep(changes.token, saved.token.clone()),
        passphrase: keep(changes.passphrase, saved.passphrase.clone()),
        basic_password: if basic_user.is_empty() { String::new() } else { keep(changes.basic_password, saved.basic_password.clone()) },
        basic_user,
        auto_sync: saved.auto_sync,
    };
    config.validate()?;
    vault.cloud = Some(config);
    Ok(())
}

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

/// Dot colour of the Settings → Account status line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Idle,
    Ok,
    Busy,
    Error,
}

impl Tone {
    pub fn color(self) -> u32 {
        match self {
            Tone::Idle => 0x9ea5b3,
            Tone::Ok => 0x00cc74,
            Tone::Busy => 0xf0bc6b,
            Tone::Error => 0xec706f,
        }
    }
}

/// The server answered: «Получить данные с сервера» is offered only then.
pub fn connected(status: &Status) -> bool {
    matches!(status, Status::Checked(preview) if *preview != Preview::Unconfigured)
}

/// `host:port` of the configured address, as the legacy settings show it.
pub fn server_host(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| u.port().map(|p| format!("{h}:{p}")).unwrap_or_else(|| h.to_owned())))
        .unwrap_or_else(|| url.to_owned())
}

/// Settings → Account → «Своё облако» status line (legacy `Hs`).
pub fn settings_status(status: &Status, url: Option<&str>) -> (String, Tone) {
    let Some(url) = url else { return ("Сервер не настроен".into(), Tone::Idle) };
    let host = server_host(url);
    let summary = |preview: &Preview| match preview {
        Preview::Synced => "данные совпадают".to_owned(),
        Preview::Empty => "снимок пуст".to_owned(),
        Preview::Conflicts(_) => "изменения есть на обеих сторонах".to_owned(),
        Preview::Changes(_) => match preview.counts() {
            (up, 0) => format!("есть локальные изменения: {up}"),
            (0, down) => format!("на сервере новые данные: {down}"),
            _ => "изменения есть на обеих сторонах".to_owned(),
        },
        Preview::Unconfigured => String::new(),
    };
    match status {
        Status::Checking | Status::Syncing => (format!("Проверка подключения к {host}…"), Tone::Busy),
        Status::Checked(Preview::Unconfigured) => ("Сервер не настроен".into(), Tone::Idle),
        Status::Checked(preview) => (format!("Подключено к {host} · {}", summary(preview)), Tone::Ok),
        Status::Error { .. } => (format!("Нет подключения к {host}"), Tone::Error),
    }
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
    fn the_preview_says_which_way_changes_go_without_moving_them() {
        let (url, stored) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        assert_eq!(preview(&Vault::default()).unwrap(), Preview::Unconfigured);
        let (mut a, mut b) = (vault(&url, &["web"]), vault(&url, &[]));
        assert_eq!(preview(&a).unwrap(), Preview::Empty);
        sync(&mut a, None, backups.path()).unwrap();
        sync(&mut b, None, backups.path()).unwrap();
        assert_eq!(preview(&a).unwrap(), Preview::Synced);
        a.hosts[0].label = "web-1".into();
        let before = stored.lock().unwrap().clone();
        let Preview::Changes(changes) = preview(&a).unwrap() else { panic!("expected changes") };
        assert_eq!(stored.lock().unwrap().clone(), before, "the preview sends nothing");
        assert_eq!(changes[0].text(), "Хост «web-1»: изменено");
        assert!(changes.iter().all(|c| c.upload));
        assert_eq!(Preview::Changes(changes).headline(), "Есть новые локальные изменения → требуется отправка в облако");
        sync(&mut a, None, backups.path()).unwrap();
        let Preview::Changes(incoming) = preview(&b).unwrap() else { panic!("expected changes") };
        assert!(incoming.iter().all(|c| !c.upload));
        b.hosts[0].label = "web-b".into();
        assert!(matches!(preview(&b).unwrap(), Preview::Conflicts(_)));
    }

    #[test]
    fn changes_are_grouped_as_in_the_legacy_dialog() {
        let change = |upload, kind: &str, label: &str, action| sync::Change { upload, kind: kind.into(), label: label.into(), action };
        let groups = group_changes(&[
            change(true, "hosts", "web", "modified"),
            change(true, "hosts", "db", "modified"),
            change(true, "sshKeys", "", "modified"),
            change(false, "hosts", "new", "added"),
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!((groups[0].direction, groups[0].title()), ("На устройстве", "Изменено · 3 записи".to_owned()));
        assert_eq!(groups[0].categories[0].label, "Хоста");
        assert_eq!(groups[0].categories[1].records[0].0, "ранее сохранённая запись");
        assert_eq!((groups[1].direction, groups[1].categories[0].label), ("В облаке", "Хост"));
        assert_eq!(overview(&groups), "4 записи · 2 группы");
        assert_eq!(count_label(11), "11 записей");
        assert_eq!(count_label(21), "21 запись");
        let conflict = ConflictInfo { kind: "hosts".into(), label: "web".into(), detail: "(адрес)".into(), text: String::new() };
        let groups = group_conflicts(&[conflict]);
        assert_eq!((groups[0].direction, groups[0].action, groups[0].categories[0].records[0].1.as_str()), ("Обе стороны", "Конфликт", "адрес"));
    }

    #[test]
    fn the_header_keeps_the_settled_state_while_checking() {
        let mut header = Header::default();
        assert_eq!((header.sync_title(|_| String::new()), header.sync_badge()), ("Синхронизация — облако не настроено".into(), None));
        header.synced_at = Some(1);
        header.set(Status::Checked(Preview::Synced));
        header.set(Status::Checking);
        assert_eq!(header.sync_title(|_| "10:00".into()), "Синхронизировано · 10:00");
        assert_eq!(header.sync_badge(), Some(("✓", TONE_OK)));
        header.set(Status::Syncing);
        assert_eq!((header.sync_title(|_| String::new()).as_str(), header.sync_badge()), ("Синхронизация…", None));
        let change = sync::Change { upload: true, kind: "hosts".into(), label: "web".into(), action: "modified" };
        header.set(Status::Checked(Preview::Changes(vec![change])));
        assert_eq!((header.indicator(true).0, header.indicator(false).0), (1, 0));
        assert_eq!(header.sync_badge(), Some(("!", TONE_WARNING)));
        let message = "Загрузка из облака: сервер отклонил авторизацию (HTTP 401). Проверьте токен";
        assert!(is_credential_error(message) && is_credential_error(crate::cloud::WRONG_PASSPHRASE));
        assert!(!is_credential_error(crate::cloud::REVISION_CHANGED));
        header.set(Status::Error { message: message.into(), credentials: true });
        assert!(header.paused());
        assert_eq!(header.indicator(true), (0, TONE_ERROR));
    }

    #[test]
    fn settings_keep_saved_secrets_when_their_fields_are_empty() {
        let mut vault = Vault::default();
        let changes = ConfigChanges { url: "https://sync.example".into(), token: "tok".into(), passphrase: "long enough passphrase".into(), ..ConfigChanges::default() };
        save_config(&mut vault, changes.clone()).unwrap();
        save_config(&mut vault, ConfigChanges { url: "https://other.example".into(), ..ConfigChanges::default() }).unwrap();
        let saved = vault.cloud.clone().unwrap();
        assert_eq!((saved.url.as_str(), saved.token.as_str(), saved.passphrase.as_str()), ("https://other.example", "tok", "long enough passphrase"));
        assert!(save_config(&mut vault, ConfigChanges { url: "http://public.example".into(), ..ConfigChanges::default() }).is_err());
        assert_eq!(vault.cloud.unwrap().url, "https://other.example", "a rejected change saves nothing");
        let mut fresh = Vault::default();
        assert_eq!(save_config(&mut fresh, ConfigChanges { url: "https://a.example".into(), ..ConfigChanges::default() }).unwrap_err(), "Недопустимый токен");
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

    #[test]
    fn settings_status_reads_like_the_legacy_account_tab() {
        let url = Some("https://sync.example:8443/base");
        let change = |upload| crate::sync::Change { upload, kind: "hosts".into(), label: "web".into(), action: "modified" };
        assert_eq!(settings_status(&Status::default(), None), ("Сервер не настроен".to_owned(), Tone::Idle));
        assert_eq!(settings_status(&Status::Checking, url).0, "Проверка подключения к sync.example:8443…");
        assert_eq!(settings_status(&Status::Checked(Preview::Synced), url), ("Подключено к sync.example:8443 · данные совпадают".to_owned(), Tone::Ok));
        assert!(settings_status(&Status::Checked(Preview::Changes(vec![change(true), change(true)])), url).0.ends_with("есть локальные изменения: 2"));
        assert!(settings_status(&Status::Checked(Preview::Changes(vec![change(false)])), url).0.ends_with("на сервере новые данные: 1"));
        assert!(settings_status(&Status::Checked(Preview::Changes(vec![change(false), change(true)])), url).0.ends_with("обеих сторонах"));
        let error = Status::Error { message: "x".into(), credentials: false };
        assert_eq!(settings_status(&error, url), ("Нет подключения к sync.example:8443".to_owned(), Tone::Error));
        assert!(connected(&Status::Checked(Preview::Empty)) && !connected(&error) && !connected(&Status::Checking));
        assert_eq!(server_host("https://h.example/x"), "h.example");
    }
}
