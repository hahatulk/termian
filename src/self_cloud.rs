//! «Своё облако» actions, as in legacy/src/bridge/self-cloud.js: Sync (merge
//! both sides), Upload (replace the cloud snapshot) and Download (replace the
//! local data after an encrypted backup). Conflicts are reported, never
//! resolved without the user's choice.

use std::path::Path;

use crate::store::CloudBackups;

use crate::cloud::{self, Condition};
use crate::portable::{self, VaultDevice};
use crate::sync::{self, ConflictInfo, ConflictPolicy};
use crate::vault::Vault;

/// Legacy-profile data has no passwords or keys; sending it would erase them in the cloud.
pub const SECRETS_MISSING: &str =
    "Не все пароли и ключи из профиля старого клиента удалось прочитать. Сначала нажмите «Получить», чтобы взять данные из облака.";
pub const NOT_CONFIGURED: &str = "Сначала настройте своё облако";

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
            _ if self.syncing() => crate::language::text("Синхронизация…").into(),
            Status::Checked(Preview::Unconfigured) => crate::language::text("Синхронизация — облако не настроено").into(),
            Status::Checked(Preview::Synced | Preview::Empty) => {
                crate::ui_format!("Sync complete{}", "Синхронизировано{}", self.synced_at.map(|t| format!(" · {}", time(t))).unwrap_or_default())
            }
            Status::Error { message, .. } => crate::ui_format!("Sync error: {}", "Ошибка синхронизации: {}", crate::language::text(message)),
            Status::Checked(Preview::Conflicts(conflicts)) => crate::ui_format!("Sync conflicts: {}", "Конфликты синхронизации: {}", conflicts.len()),
            _ => crate::language::text("Есть что синхронизировать").into(),
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
}

#[derive(Clone, Debug)]
pub struct UploadPreview {
    server: String,
    condition: Condition,
}

/// Compares this device with the cloud without changing either side.
pub fn checked_preview(vault: &Vault) -> Result<(Preview, Option<UploadPreview>), String> {
    let Some(config) = vault.cloud.clone().filter(|c| c.validate().is_ok()) else { return Ok((Preview::Unconfigured, None)) };
    let client = config.validate()?;
    let (remote, can_create) = client.fetch_for_sync().map_err(|e| config.redact(&e))?;
    if remote.is_none() && !can_create && !config.workspace.is_nil() {
        return Err(crate::language::text("Update the sync server to use more than one Vault.").into());
    }
    let ticket = |condition| UploadPreview { server: config.sync_key(), condition };
    let Some(remote) = remote else {
        return Ok((Preview::Empty, can_create.then(|| ticket(Condition::Create))));
    };
    let ticket = remote.revision.map(|revision| ticket(Condition::Revision(revision)));
    let remote = sync::validate_snapshot(&remote.snapshot)?;
    let server = config.sync_key();
    let saved = vault.sync_state.clone().filter(|s| s.server == server);
    let mut ids = saved.as_ref().map(|s| s.ids.clone()).unwrap_or_default();
    let exported = sync::validate_snapshot(&portable::export(vault, &now()))?;
    sync::match_ids(&exported, Some(&remote), &mut ids);
    let local = sync::to_sync_form(&exported, &mut ids);
    let base = saved.map(|s| sync::Base { local: s.local, remote: s.remote });
    let merged = sync::merge_snapshots(&local, Some(&remote), base.as_ref(), ConflictPolicy::Preview, &now());
    Ok((if !merged.conflicts.is_empty() {
        Preview::Conflicts(merged.conflicts)
    } else if merged.changes.is_empty() {
        Preview::Synced
    } else {
        Preview::Changes(merged.changes)
    }, ticket))
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
    vault.cloud = Some(candidate_config(vault, changes)?);
    Ok(())
}

pub fn candidate_config(vault: &Vault, changes: ConfigChanges) -> Result<cloud::CloudConfig, String> {
    let saved = vault.cloud.clone().unwrap_or(cloud::CloudConfig { auto_sync: true, ..cloud::CloudConfig::default() });
    let keep = |new: String, old: String| if new.is_empty() { old } else { new };
    let basic_user = changes.basic_user.trim().to_owned();
    let url = changes.url.trim().to_owned();
    let same_server = cloud::same_snapshot_endpoint(&url, &saved.url);
    let config = cloud::CloudConfig {
        url: if same_server { saved.url } else { url },
        token: keep(changes.token, if same_server { saved.token } else { String::new() }),
        passphrase: keep(changes.passphrase, if same_server { saved.passphrase } else { String::new() }),
        basic_password: if basic_user.is_empty() {
            String::new()
        } else {
            keep(changes.basic_password, if same_server && basic_user == saved.basic_user { saved.basic_password } else { String::new() })
        },
        basic_user,
        auto_sync: saved.auto_sync,
        workspace: saved.workspace,
    };
    config.validate()?;
    Ok(config)
}

pub fn verify_cloud_password(config: &cloud::CloudConfig) -> Result<(), String> {
    let verify = |cloud: cloud::Cloud| {
        if let Some(remote) = cloud.fetch().map_err(|error| config.redact(&error))? {
            sync::validate_snapshot(&remote.snapshot)?;
        }
        Ok::<(), String>(())
    };
    verify(config.scoped(uuid::Uuid::nil()).validate()?)?;
    if !config.workspace.is_nil() {
        verify(config.validate()?)?;
    }
    config.catalog()?.fetch().map_err(|error| config.redact(&error))?;
    Ok(())
}

fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn configured(vault: &Vault) -> Result<(cloud::CloudConfig, cloud::Cloud), String> {
    let config = vault.cloud.clone().ok_or(NOT_CONFIGURED)?;
    let client = config.validate()?;
    Ok((config, client))
}

pub fn prepare_logs(vault: &mut Vault, logs_dir: &Path, active: &std::collections::HashSet<u64>) -> Option<String> {
    if vault.secrets_missing {
        return None;
    }
    let (config, client) = configured(vault).ok()?;
    match crate::log_backup::backup(&client, logs_dir, &vault.history) {
        Ok(outcome) => {
            crate::log_backup::prepare_catalog(vault, &outcome.ready, active);
            if outcome.pending_snapshot {
                Some("Журналы будут отправлены при следующей синхронизации после создания снимка.".into())
            } else if outcome.unsupported {
                Some("Сервер не поддерживает отправку логов.".into())
            } else if outcome.skipped > 0 {
                Some(format!("Логов больше 34 МиБ пропущено: {}", outcome.skipped))
            } else {
                None
            }
        }
        Err(error) => Some(format!("Не удалось отправить логи: {}", config.redact(&error))),
    }
}

/// Sync: merge, retrying twice when another device uploaded meanwhile.
pub fn sync(vault: &mut Vault, backups: &CloudBackups) -> Result<String, String> {
    let (config, client) = configured(vault)?;
    if vault.secrets_missing {
        return Err(SECRETS_MISSING.into());
    }
    let server = config.sync_key();
    let mut attempt = 0;
    loop {
        let saved = vault.sync_state.clone();
        let result = {
            let mut device = VaultDevice { vault: &mut *vault, now: now(), backup: Box::new(|vault| backups.save(vault)) };
            sync::sync_once(saved, &server, &mut device, &client, &now())
        };
        match result {
            Ok(outcome) => {
                vault.sync_state = outcome.state;
                let mut parts = Vec::new();
                if outcome.from_local > 0 {
                    parts.push(format!("отправлено изменений: {}", outcome.from_local));
                }
                if outcome.from_remote > 0 {
                    parts.push(format!("получено изменений: {}", outcome.from_remote));
                }
                if outcome.resolved_conflicts > 0 {
                    parts.push(format!("разрешено конфликтов: {}", outcome.resolved_conflicts));
                }
                let text = if parts.is_empty() { "Данные уже синхронизированы".to_owned() } else { format!("Синхронизировано: {}", parts.join(", ")) };
                return Ok(text);
            }
            Err(error) if attempt < 2 && error.contains(cloud::REVISION_CHANGED) => attempt += 1,
            Err(error) => return Err(config.redact(&error)),
        }
    }
}

/// «Отправить»: the cloud snapshot is replaced with this device's data.
pub fn upload_checked(vault: &mut Vault, preview: UploadPreview) -> Result<String, String> {
    let (config, client) = configured(vault)?;
    if vault.secrets_missing {
        return Err(SECRETS_MISSING.into());
    }
    let mut ids = vault.sync_state.as_ref().filter(|s| s.server == config.sync_key()).map(|s| s.ids.clone()).unwrap_or_default();
    let exported = sync::validate_snapshot(&portable::export(vault, &now()))?;
    let snapshot = sync::validate_snapshot(&sync::to_sync_form(&exported, &mut ids))?;
    let (remote, can_create) = client.fetch_for_sync().map_err(|e| config.redact(&e))?;
    let condition = match remote {
        Some(remote) => Condition::Revision(remote.revision.ok_or("Сервер не вернул ревизию снимка. Запись отменена.")?),
        None if can_create => Condition::Create,
        None => return Err("Сервер не подтвердил безопасное создание снимка.".into()),
    };
    if preview.server != config.sync_key() || preview.condition != condition {
        return Err(cloud::REVISION_CHANGED.into());
    }
    client.upload(&snapshot, preview.condition).map_err(|e| config.redact(&e))?;
    vault.sync_state = Some(sync::State { server: config.sync_key(), local: sync::digests_of(Some(&snapshot)), remote: sync::digests_of(Some(&snapshot)), ids });
    Ok("Данные загружены".into())
}

#[cfg(test)]
pub(crate) fn upload(vault: &mut Vault) -> Result<String, String> {
    let ticket = checked_preview(vault)?.1.ok_or("missing upload preview")?;
    upload_checked(vault, ticket)
}

#[cfg(test)]
fn preview(vault: &Vault) -> Result<Preview, String> {
    checked_preview(vault).map(|(preview, _)| preview)
}

#[cfg(test)]
pub(crate) fn download(vault: &mut Vault, backups: &CloudBackups) -> Result<String, String> {
    let ticket = checked_preview(vault)?.1.ok_or("Нет ревизии снимка.")?;
    download_checked(vault, backups, ticket)
}

/// «Получить»: the local data is replaced with the cloud snapshot, after a backup.
pub fn download_checked(vault: &mut Vault, backups: &CloudBackups, preview: UploadPreview) -> Result<String, String> {
    let (config, client) = configured(vault)?;
    let remote = client.fetch().map_err(|e| config.redact(&e))?.ok_or("В облаке ещё нет данных")?;
    if preview.server != config.sync_key() || remote.revision.map(Condition::Revision) != Some(preview.condition) {
        return Err(cloud::REVISION_CHANGED.into());
    }
    let snapshot = sync::validate_snapshot(&remote.snapshot)?;
    backups.save(vault)?;
    let server = config.sync_key();
    let mut matched = vault.sync_state.as_ref().filter(|state| state.server == server)
        .map(|state| state.ids.clone()).unwrap_or_default();
    let exported = portable::export(vault, &now());
    sync::match_ids(&exported, Some(&snapshot), &mut matched);
    let pairs = portable::import(vault, &snapshot, &matched)?;
    let mut ids = sync::ids_from_import(&pairs);
    let exported = sync::validate_snapshot(&portable::export(vault, &now()))?;
    sync::match_ids(&exported, Some(&snapshot), &mut ids);
    let local = sync::to_sync_form(&exported, &mut ids);
    vault.sync_state = Some(sync::State { server: config.sync_key(), local: sync::digests_of(Some(&local)), remote: sync::digests_of(Some(&snapshot)), ids });
    Ok("Данные загружены и применены".into())
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
    let Some(url) = url else { return (crate::language::text("Сервер не настроен").into(), Tone::Idle) };
    let host = server_host(url);
    let summary = |preview: &Preview| match preview {
        Preview::Synced => crate::language::text("данные совпадают").to_owned(),
        Preview::Empty => crate::language::text("снимок пуст").to_owned(),
        Preview::Conflicts(_) => crate::language::text("изменения есть на обеих сторонах").to_owned(),
        Preview::Changes(_) => match preview.counts() {
            (up, 0) => crate::ui_format!("local changes: {up}", "есть локальные изменения: {up}"),
            (0, down) => crate::ui_format!("new server data: {down}", "на сервере новые данные: {down}"),
            _ => crate::language::text("изменения есть на обеих сторонах").to_owned(),
        },
        Preview::Unconfigured => String::new(),
    };
    match status {
        Status::Checking | Status::Syncing => (crate::ui_format!("Checking connection to {host}…", "Проверка подключения к {host}…"), Tone::Busy),
        Status::Checked(Preview::Unconfigured) => (crate::language::text("Сервер не настроен").into(), Tone::Idle),
        Status::Checked(preview) => (crate::ui_format!("Connected to {host} · {}", "Подключено к {host} · {}", summary(preview)), Tone::Ok),
        Status::Error { .. } => (crate::ui_format!("Cannot connect to {host}", "Нет подключения к {host}"), Tone::Error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Host;

    fn test_backups(dir: &Path) -> CloudBackups {
        crate::store::Store::new(dir.join("vault.bin"), [3; 32]).cloud_backups()
    }

    fn host(id: u64, label: &str) -> Host {
        Host { id, label: label.into(), address: format!("{label}.lan"), os: None, group: None, username: None, tags: vec![], port: None, password: None, key: None, identity: None, settings: Default::default() }
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
    fn rotation_verification_leaves_the_vault_and_server_untouched() {
        let (url, stored) = crate::cloud::tests::server();
        let mut local = vault(&url, &["web"]);
        upload(&mut local).unwrap();
        let previous = local.clone();
        let mut next = local.cloud.clone().unwrap();
        next.passphrase = "different test password".into();
        let before = stored.lock().unwrap().clone();
        assert_eq!(verify_cloud_password(&next), Err(cloud::WRONG_PASSPHRASE.into()));
        assert_eq!(*stored.lock().unwrap(), before);
        assert_eq!(local, previous);

        let remote = local.cloud.as_ref().unwrap().validate().unwrap().fetch().unwrap().unwrap();
        next.validate().unwrap().upload(&remote.snapshot, Condition::Revision(remote.revision.unwrap())).unwrap();
        let rotated = stored.lock().unwrap().clone();
        assert!(verify_cloud_password(&next).is_ok());
        assert!(upload(&mut local).is_err());
        assert_eq!(*stored.lock().unwrap(), rotated);
        local.cloud = Some(next);
        let backups = tempfile::tempdir().unwrap();
        assert_eq!(sync(&mut local, &test_backups(backups.path())).unwrap(), "Данные уже синхронизированы");
    }

    #[test]
    fn sync_merges_and_reports_what_moved() {
        let (url, _) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let (mut a, mut b) = (vault(&url, &["web"]), vault(&url, &["db"]));
        assert_eq!(sync(&mut a, &test_backups(backups.path())).unwrap(), "Синхронизировано: отправлено изменений: 2");
        assert!(sync(&mut b, &test_backups(backups.path())).unwrap().contains("получено изменений: 2"));
        assert_eq!(labels(&b), ["db", "web"]);
        assert_eq!(std::fs::read_dir(backups.path().join("self-cloud-backups/device")).unwrap().count(), 1, "a backup before the import");
        sync(&mut a, &test_backups(backups.path())).unwrap();
        assert_eq!(sync(&mut a, &test_backups(backups.path())).unwrap(), "Данные уже синхронизированы");
    }

    #[test]
    fn preview_lists_conflicts_that_sync_resolves() {
        let (url, _) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let (mut a, mut b) = (vault(&url, &["web"]), vault(&url, &[]));
        sync(&mut a, &test_backups(backups.path())).unwrap();
        sync(&mut b, &test_backups(backups.path())).unwrap();
        a.hosts[0].address = "a.lan".into();
        b.hosts[0].address = "b.lan".into();
        sync(&mut a, &test_backups(backups.path())).unwrap();
        let Preview::Conflicts(conflicts) = preview(&b).unwrap() else { panic!("expected a conflict") };
        assert_eq!(conflicts[0].label, "web");
        assert_eq!(b.hosts[0].address, "b.lan");
        assert!(sync(&mut b, &test_backups(backups.path())).unwrap().contains("разрешено конфликтов: 1"));
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
        assert_eq!(sync(&mut legacy, &test_backups(backups.path())).unwrap_err(), SECRETS_MISSING);
        assert_eq!(upload(&mut legacy).unwrap_err(), SECRETS_MISSING);
        let before = stored.lock().unwrap().clone();
        download(&mut legacy, &test_backups(backups.path())).unwrap();
        assert_eq!(stored.lock().unwrap().clone(), before, "the cloud is untouched");
        assert_eq!(legacy.hosts[0].password.as_deref(), Some("secret"));
        assert!(!legacy.secrets_missing);
        assert!(sync(&mut legacy, &test_backups(backups.path())).is_ok(), "sync is allowed after a download");
    }

    #[test]
    fn the_preview_says_which_way_changes_go_without_moving_them() {
        let (url, stored) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        assert_eq!(preview(&Vault::default()).unwrap(), Preview::Unconfigured);
        let (mut a, mut b) = (vault(&url, &["web"]), vault(&url, &[]));
        assert_eq!(preview(&a).unwrap(), Preview::Empty);
        sync(&mut a, &test_backups(backups.path())).unwrap();
        sync(&mut b, &test_backups(backups.path())).unwrap();
        assert_eq!(preview(&a).unwrap(), Preview::Synced);
        a.hosts[0].label = "web-1".into();
        let before = stored.lock().unwrap().clone();
        let Preview::Changes(changes) = preview(&a).unwrap() else { panic!("expected changes") };
        assert_eq!(stored.lock().unwrap().clone(), before, "the preview sends nothing");
        assert_eq!(changes[0].text(), "Хост «web-1»: изменено");
        assert!(changes.iter().all(|c| c.upload));
        assert_eq!(Preview::Changes(changes).headline(), "Есть новые локальные изменения → требуется отправка в облако");
        sync(&mut a, &test_backups(backups.path())).unwrap();
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
        assert_eq!((header.sync_title(|_| String::new()), header.sync_badge()), ("Sync: cloud is not configured".into(), None));
        header.synced_at = Some(1);
        header.set(Status::Checked(Preview::Synced));
        header.set(Status::Checking);
        assert_eq!(header.sync_title(|_| "10:00".into()), "Sync complete · 10:00");
        assert_eq!(header.sync_badge(), Some(("✓", TONE_OK)));
        header.set(Status::Syncing);
        assert_eq!((header.sync_title(|_| String::new()).as_str(), header.sync_badge()), ("Syncing…", None));
        let change = sync::Change { upload: true, kind: "hosts".into(), label: "web".into(), action: "modified" };
        header.set(Status::Checked(Preview::Changes(vec![change])));
        assert_eq!(header.sync_badge(), Some(("!", TONE_WARNING)));
        let message = "Загрузка из облака: сервер отклонил авторизацию (HTTP 401). Проверьте токен";
        assert!(is_credential_error(message) && is_credential_error(crate::cloud::WRONG_PASSPHRASE));
        assert!(!is_credential_error(crate::cloud::REVISION_CHANGED));
        header.set(Status::Error { message: message.into(), credentials: true });
        assert!(header.paused());
    }

    #[test]
    fn settings_keep_credentials_only_for_the_same_server() {
        let mut vault = Vault::default();
        let mut changes = ConfigChanges {
            url: "https://sync.example".into(),
            token: "tok".into(),
            passphrase: "long enough passphrase".into(),
            basic_user: "proxy".into(),
            basic_password: "test password".into(),
        };
        save_config(&mut vault, changes.clone()).unwrap();
        let same = ConfigChanges { url: changes.url.clone(), basic_user: changes.basic_user.clone(), ..ConfigChanges::default() };
        let saved = vault.cloud.as_ref().unwrap();
        for url in ["https://sync.example", "https://SYNC.EXAMPLE:443/", "https://sync.example/v1/snapshot/"] {
            let equivalent = ConfigChanges { url: url.into(), ..same.clone() };
            assert_eq!(&candidate_config(&vault, equivalent).unwrap(), saved);
        }
        for url in ["https://sync.example/another/", "https://sync.example:444/"] {
            let different = ConfigChanges { url: url.into(), token: "other-token".into(), basic_user: String::new(), ..same.clone() };
            assert!(candidate_config(&vault, different).unwrap().passphrase.is_empty());
        }
        let other = ConfigChanges { url: "https://other.example".into(), ..same };
        assert!(save_config(&mut vault, other.clone()).is_err());
        assert!(candidate_config(&vault, ConfigChanges { token: changes.token.clone(), passphrase: changes.passphrase.clone(), ..other.clone() }).is_err());
        assert_eq!(vault.cloud.as_ref().unwrap().url, changes.url);
        changes.url = other.url;
        save_config(&mut vault, changes).unwrap();
        assert_eq!(vault.cloud.unwrap().url, "https://other.example");
    }

    #[test]
    fn equivalent_cloud_address_keeps_the_snapshot_password() {
        let (url, stored) = crate::cloud::tests::server();
        let mut local = vault(&url, &["local"]);
        upload(&mut local).unwrap();
        let before = stored.lock().unwrap().clone();
        let baseline = local.sync_state.as_ref().unwrap().server.clone();

        save_config(&mut local, ConfigChanges {
            url: format!("{url}/v1/snapshot/"),
            token: "test-token".into(),
            ..ConfigChanges::default()
        }).unwrap();

        assert_eq!(preview(&local).unwrap(), Preview::Synced);
        assert_eq!(sync::server_key(&local.cloud.as_ref().unwrap().url), baseline);

        let passphrase = std::mem::take(&mut local.cloud.as_mut().unwrap().passphrase);
        assert_eq!(preview(&local), Err(cloud::WRONG_PASSPHRASE.into()));
        let corrected = candidate_config(&local, ConfigChanges { url, passphrase, ..ConfigChanges::default() }).unwrap();
        verify_cloud_password(&corrected).unwrap();
        local.cloud = Some(corrected);
        assert_eq!(preview(&local).unwrap(), Preview::Synced);
        assert_eq!(*stored.lock().unwrap(), before);
    }

    #[test]
    fn forced_upload_requires_the_previewed_revision() {
        let (url, stored) = crate::cloud::tests::server();
        let mut local = vault(&url, &["local"]);
        let mut peer = vault(&url, &["peer"]);
        for _ in 0..2 {
            let ticket = checked_preview(&local).unwrap().1.unwrap();
            upload(&mut peer).unwrap();
            let after = stored.lock().unwrap().clone();
            assert!(upload_checked(&mut local, ticket).unwrap_err().contains(cloud::REVISION_CHANGED));
            assert_eq!(*stored.lock().unwrap(), after);
        }
    }

    #[test]
    fn changed_download_revision_leaves_vault_and_backups_untouched() {
        let (url, _) = crate::cloud::tests::server();
        let mut local = vault(&url, &["local"]);
        let mut peer = vault(&url, &["peer"]);
        let dir = tempfile::tempdir().unwrap();
        let backups = test_backups(dir.path());
        upload(&mut peer).unwrap();
        let ticket = checked_preview(&local).unwrap().1.unwrap();
        peer.hosts[0].label = "updated".into();
        upload(&mut peer).unwrap();
        let original = local.clone();
        assert!(download_checked(&mut local, &backups, ticket).unwrap_err().contains(cloud::REVISION_CHANGED));
        assert_eq!(local, original);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        download(&mut local, &backups).unwrap();
        assert_eq!(labels(&local), ["updated"]);
    }

    #[test]
    fn upload_replaces_and_backups_are_capped() {
        let (url, _) = crate::cloud::tests::server();
        let backups = tempfile::tempdir().unwrap();
        let mut a = vault(&url, &["web"]);
        a.cloud.as_mut().unwrap().passphrase.clear();
        let store = crate::store::Store::new(backups.path().join("vault.bin"), [3; 32]);
        let copies = store.cloud_backups();
        let legacy = backups.path().join("self-cloud-backups/old.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"older backup").unwrap();
        assert!(matches!(download(&mut a, &copies), Err(e) if e == "В облаке ещё нет данных"));
        upload(&mut a).unwrap();
        let mut b = vault(&url, &["other"]);
        b.cloud.as_mut().unwrap().passphrase.clear();
        let original = b.clone();
        download(&mut b, &copies).unwrap();
        let dir = backups.path().join("self-cloud-backups/device");
        let saved = std::fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
        assert_eq!(crate::store::Store::new(saved.clone(), [3; 32]).load().unwrap(), Some(original));
        assert!(crate::store::Store::new(saved, [4; 32]).load().is_err());
        for _ in 0..12 {
            download(&mut b, &copies).unwrap();
        }
        assert_eq!(labels(&b), ["web"]);
        assert_eq!(std::fs::read_dir(backups.path().join("self-cloud-backups/device")).unwrap().count(), 10);
        assert_eq!(std::fs::read(legacy).unwrap(), b"older backup");
        assert_eq!(sync(&mut Vault::default(), &copies).unwrap_err(), NOT_CONFIGURED);
    }

    #[test]
    fn settings_status_reads_like_the_legacy_account_tab() {
        let url = Some("https://sync.example:8443/base");
        let change = |upload| crate::sync::Change { upload, kind: "hosts".into(), label: "web".into(), action: "modified" };
        assert_eq!(settings_status(&Status::default(), None), ("Server not configured".to_owned(), Tone::Idle));
        assert_eq!(settings_status(&Status::Checking, url).0, "Checking connection to sync.example:8443…");
        assert_eq!(settings_status(&Status::Checked(Preview::Synced), url), ("Connected to sync.example:8443 · data matches".to_owned(), Tone::Ok));
        assert!(settings_status(&Status::Checked(Preview::Changes(vec![change(true), change(true)])), url).0.ends_with("local changes: 2"));
        assert!(settings_status(&Status::Checked(Preview::Changes(vec![change(false)])), url).0.ends_with("new server data: 1"));
        assert!(settings_status(&Status::Checked(Preview::Changes(vec![change(false), change(true)])), url).0.ends_with("both sides have changes"));
        let error = Status::Error { message: "x".into(), credentials: false };
        assert_eq!(settings_status(&error, url), ("Cannot connect to sync.example:8443".to_owned(), Tone::Error));
        assert!(connected(&Status::Checked(Preview::Empty)) && !connected(&error) && !connected(&Status::Checking));
        assert_eq!(server_host("https://h.example/x"), "h.example");
    }
}
