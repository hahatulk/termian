//! Silent backup of session logs to the own cloud, as legacy
//! `session_logs.rs` / `bridge/session-log-sync.js` do it: new or changed
//! files only (a ledger of size and mtime), each sealed with the cloud
//! passphrase as a `termian-session-log` payload at `/v1/logs/<name>`, and a
//! missing local file restored from its copy.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cloud::Cloud;
use crate::session_log;
use crate::vault::{bounded_log_comment, HistoryEntry, LogRef, Vault};

const FORMAT: &str = "termian-session-log";
const CATALOG: &str = "termian.sessionLog.";

/// A sealed payload is limited like a snapshot (legacy: 34 MiB of log).
const MAX_FILE: usize = 34 * 1024 * 1024;
pub const FIRST_RUN: Duration = Duration::from_secs(45);
pub const INTERVAL: Duration = Duration::from_secs(60 * 60);
pub const AFTER_SESSION: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Uploaded {
    size: u64,
    modified: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub uploaded: usize,
    pub unchanged: usize,
    pub ready: Vec<String>,
    pub skipped: usize,
    /// The server is older than the logs API: stop until restart, as legacy does.
    pub unsupported: bool,
}

/// `session-log-uploads.json` beside the logs folder.
pub fn ledger_path(logs_dir: &Path) -> PathBuf {
    logs_dir.parent().unwrap_or(logs_dir).join("session-log-uploads.json")
}

fn read_ledger(path: &Path) -> BTreeMap<String, Uploaded> {
    fs::read(path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// What a restored copy needs, in the legacy `meta` shape; the key goes as
/// legacy's `secretKey` text when it is one, and always as base64.
fn meta(entry: &HistoryEntry, name: &str, key: &str) -> Value {
    let secret = STANDARD.decode(key).ok().and_then(|bytes| String::from_utf8(bytes).ok()).filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_graphic()));
    json!({
        "local_id": entry.id,
        "label": entry.label,
        "connected_at": entry.connected_at.saturating_mul(1000),
        "disconnectedAt": entry.disconnected_at.map(|t| t.saturating_mul(1000)),
        "session_log_data": {"name": name, "secretKey": secret, "key": key},
    })
}

/// Uploads the logs of `history` that are new or changed since the last run.
pub fn backup(cloud: &Cloud, logs_dir: &Path, history: &[HistoryEntry]) -> Result<Outcome, String> {
    let ledger_file = ledger_path(logs_dir);
    if history.iter().all(|entry| entry.remote_log || entry.log.is_none()) {
        return Ok(Outcome::default());
    }
    let generation = cloud.log_generation()?;
    let prefix = format!("{}:{generation}:", cloud.storage_key());
    let mut ledger = read_ledger(&ledger_file);
    let mut outcome = Outcome::default();
    for entry in history {
        if entry.remote_log { continue; }
        let Some(log) = &entry.log else { continue };
        let name = session_log::file_name(&log.name);
        if !session_log::valid_name(&name) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(logs_dir.join(&name)) else { continue };
        if !metadata.is_file() { continue; }
        if metadata.len() > MAX_FILE as u64 {
            outcome.skipped += 1;
            continue;
        }
        let current = Uploaded { size: metadata.len(), modified: metadata.modified().map(millis).unwrap_or(0) };
        let entry_key = format!("{prefix}{name}");
        if ledger.get(&entry_key) == Some(&current) {
            outcome.unchanged += 1;
            outcome.ready.push(name);
            continue;
        }
        let Ok(file) = fs::read(logs_dir.join(&name)) else { continue };
        if file.len() > MAX_FILE {
            outcome.skipped += 1;
            continue;
        }
        let payload = json!({"format": FORMAT, "version": 1, "name": name, "modified": current.modified, "meta": meta(entry, &name, &log.key), "file": STANDARD.encode(&file)});
        if !cloud.put_log(&name, &payload, &generation)? {
            outcome.unsupported = true;
            break;
        }
        ledger.insert(entry_key, current);
        outcome.uploaded += 1;
        outcome.ready.push(name);
        if let Ok(bytes) = serde_json::to_vec(&ledger) {
            let _ = fs::write(&ledger_file, bytes);
        }
    }
    Ok(outcome)
}

/// Brings a missing log file back from the cloud; `false` when there is no copy.
pub fn restore(cloud: &Cloud, logs_dir: &Path, log: &LogRef) -> Result<bool, String> {
    let name = session_log::file_name(&log.name);
    if !session_log::valid_name(&name) {
        return Err("invalid session log name".into());
    }
    let path = logs_dir.join(&name);
    if path.exists() {
        return Ok(true);
    }
    let Some(payload) = cloud.get_log(&name)? else { return Ok(false) };
    if payload["format"] != FORMAT || payload["name"] != name.as_str() {
        return Err("the cloud copy is not this session log".into());
    }
    let key = |value: &str| STANDARD.decode(value).ok().filter(|bytes| bytes.len() == 32);
    let cloud_key = payload["meta"]["session_log_data"]["key"].as_str().and_then(key)
        .or_else(|| payload["meta"]["session_log_data"]["secretKey"].as_str()
            .filter(|value| value.len() == 32).map(|value| value.as_bytes().to_vec()));
    if cloud_key.is_none() || cloud_key != key(&log.key) {
        return Err("the cloud copy uses a different log key".into());
    }
    let file = payload["file"].as_str().filter(|f| f.len() <= MAX_FILE.div_ceil(3) * 4)
        .and_then(|f| STANDARD.decode(f).ok()).filter(|file| file.len() <= MAX_FILE)
        .ok_or("the cloud copy has invalid log data")?;
    fs::create_dir_all(logs_dir).map_err(|e| e.to_string())?;
    let mut staged = tempfile::NamedTempFile::new_in(logs_dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut staged, &file).map_err(|e| e.to_string())?;
    match staged.persist_noclobber(path) {
        Ok(_) => {}
        Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    Ok(true)
}

fn catalog_name(log: &LogRef) -> String {
    format!("{CATALOG}{}", session_log::file_name(&log.name))
}

pub(crate) fn catalog_label(name: &str, value: &Value) -> Option<String> {
    name.strip_prefix(CATALOG)?;
    Some(value["label"].as_str().unwrap_or_default().chars().filter(|c| !c.is_control()).take(100).collect())
}

pub fn catalogued(settings: &Value, entry: &HistoryEntry) -> bool {
    entry.log.as_ref().is_some_and(|log| settings[&catalog_name(log)]["key"].as_str() == Some(log.key.as_str()))
}

fn descriptor(entry: &HistoryEntry, key: &str, os: Option<&str>) -> Value {
    let label: String = entry.label.chars().take(512).collect();
    let subtitle: String = entry.subtitle.chars().take(512).collect();
    let device: String = entry.device.chars().take(255).collect();
    json!({
        "key": key, "label": label, "subtitle": subtitle, "device": device, "os": os,
        "connected": entry.connected_at, "disconnected": entry.disconnected_at, "successful": entry.connected,
        "saved": entry.saved, "comment": bounded_log_comment(&entry.comment),
    })
}

pub fn reset_catalog(vault: &mut Vault) {
    if let Some(settings) = vault.foreign["settings"].as_object_mut() {
        settings.retain(|key, _| !key.starts_with(CATALOG));
    }
}

pub fn prepare_catalog(vault: &mut Vault, ready: &[String], active: &std::collections::HashSet<u64>) {
    let mut settings = export_settings(vault);
    let object = settings.as_object_mut().expect("portable settings object");
    for entry in &vault.history {
        let Some(log) = &entry.log else { continue };
        if !active.contains(&entry.id) && ready.contains(&session_log::file_name(&log.name)) {
            let os = entry.log_os.as_deref().or_else(|| entry.host.and_then(|id| vault.host(id)).and_then(|host| host.os.as_deref()));
            object.insert(catalog_name(log), descriptor(entry, &log.key, os));
        }
    }
    vault.foreign["settings"] = settings;
}

pub(crate) fn export_settings(vault: &Vault) -> Value {
    let mut settings = vault.foreign["settings"].as_object().cloned().unwrap_or_default();
    settings.retain(|key, _| !key.starts_with(CATALOG));
    for entry in &vault.history {
        let Some(log) = &entry.log else { continue };
        if catalogued(&vault.foreign["settings"], entry) {
            let os = entry.log_os.as_deref().or_else(|| entry.host.and_then(|id| vault.host(id)).and_then(|host| host.os.as_deref()));
            settings.insert(catalog_name(log), descriptor(entry, &log.key, os));
        }
    }
    Value::Object(settings)
}

pub(crate) fn apply_catalog(vault: &mut Vault, settings: &Value) {
    let previous = &vault.foreign["settings"];
    let mut incoming = std::collections::BTreeMap::new();
    for (catalog_key, value) in settings.as_object().into_iter().flatten() {
        let Some(name) = catalog_key.strip_prefix(CATALOG).filter(|name| name.ends_with(".log") && session_log::valid_name(name)) else { continue };
        let Some(key) = value["key"].as_str().filter(|key| key.len() == 44 && STANDARD.decode(key).is_ok_and(|bytes| bytes.len() == 32)) else { continue };
        let Some(connected_at) = value["connected"].as_i64().filter(|value| chrono::DateTime::from_timestamp(*value, 0).is_some()) else { continue };
        let text = |field: &str, limit: usize| -> String {
            value[field].as_str().unwrap_or_default().chars().take(limit).collect()
        };
        incoming.insert(catalog_key.as_str(), HistoryEntry {
            label: text("label", 512), subtitle: text("subtitle", 512), device: text("device", 255),
            log_os: value["os"].as_str().map(|os| os.chars().take(32).collect()),
            connected_at,
            disconnected_at: value["disconnected"].as_i64().filter(|end| *end >= connected_at && chrono::DateTime::from_timestamp(*end, 0).is_some()),
            log: Some(LogRef { name: name.to_owned(), key: key.to_owned() }),
            remote_log: true,
            connected: value["successful"].as_bool().unwrap_or_default(),
            saved: value["saved"].as_bool().unwrap_or(false),
            comment: bounded_log_comment(value["comment"].as_str().unwrap_or_default()).to_owned(),
            ..HistoryEntry::default()
        });
    }
    let mut id = vault.history.iter().map(|entry| entry.id).max().unwrap_or(0);
    vault.history.retain_mut(|entry| {
        let Some(log) = &entry.log else { return true };
        let name = catalog_name(log);
        if let Some(mut synced) = incoming.remove(name.as_str()) {
            if synced.log.as_ref().is_some_and(|s| s.key == log.key) {
                synced.id = entry.id;
                synced.host = entry.host;
                synced.log = entry.log.clone();
                synced.remote_log = entry.remote_log;
                *entry = synced;
            }
            return true;
        }
        !catalogued(previous, entry) || settings.get(name.as_str()).is_some()
    });
    for (_, mut entry) in incoming {
        let Some(next) = id.checked_add(1) else { break };
        id = next;
        entry.id = id;
        vault.history.push(entry);
    }
    vault.history.sort_by_key(|entry| std::cmp::Reverse((entry.connected_at, entry.id)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::tests::{config, server};

    fn entry(id: u64, name: &str) -> HistoryEntry {
        HistoryEntry { id, label: "web".into(), connected_at: 1_794_000_000, log: Some(LogRef { name: name.into(), key: STANDARD.encode([7u8; 32]) }), ..HistoryEntry::default() }
    }

    #[test]
    fn uploads_new_and_changed_logs_once_and_restores_them() {
        let (url, _) = server();
        let cloud = config(&url).validate().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("session-logs-v2");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("a-1.log"), b"TMNLOG1 first").unwrap();
        let history = vec![entry(1, "a-1"), entry(2, "missing-2")];

        assert_eq!(backup(&cloud, &logs, &history).unwrap(), Outcome { uploaded: 1, ready: vec!["a-1.log".into()], ..Outcome::default() });
        assert_eq!(backup(&cloud, &logs, &history).unwrap(), Outcome { unchanged: 1, ready: vec!["a-1.log".into()], ..Outcome::default() }, "nothing twice");
        fs::write(logs.join("a-1.log"), b"TMNLOG1 first and more").unwrap();
        assert_eq!(backup(&cloud, &logs, &history).unwrap().uploaded, 1, "a grown log goes again");

        fs::remove_file(logs.join("a-1.log")).unwrap();
        let mismatched = LogRef { name: "a-1".into(), key: STANDARD.encode([8; 32]) };
        assert!(restore(&cloud, &logs, &mismatched).is_err());
        assert!(!logs.join("a-1.log").exists());
        assert!(restore(&cloud, &logs, history[0].log.as_ref().unwrap()).unwrap());
        assert_eq!(fs::read(logs.join("a-1.log")).unwrap(), b"TMNLOG1 first and more");
        assert!(!restore(&cloud, &logs, history[1].log.as_ref().unwrap()).unwrap(), "no copy in the cloud");
        assert!(restore(&cloud, &logs, entry(3, "../x").log.as_ref().unwrap()).is_err());
    }

    #[test]
    fn sync_shares_logs_and_saved_comments_between_vaults() {
        use crate::self_cloud::{prepare_logs, sync};
        let (url, _) = server();
        let cloud = config(&url);
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("a/logs");
        let target = root.path().join("b/logs");
        let backups = crate::store::Store::new(root.path().join("vault.bin"), [3; 32]).cloud_backups();
        fs::create_dir_all(&source).unwrap();
        let mut a = Vault { cloud: Some(cloud.clone()), ..Vault::default() };
        let mut b = a.clone();
        let mut row = entry(0, "completed");
        row.device = "Test device".into();
        row.log_os = Some("debian".into());
        row.saved = true;
        row.comment = "first note".into();
        let a_id = a.record_connection(row);
        let mut bytes = b"TMNLOG1\n".to_vec();
        bytes.extend(session_log::encode_record(b"test output", &[7; 32]));
        fs::write(source.join("completed.log"), bytes).unwrap();
        let active_id = a.record_connection(entry(0, "open"));
        fs::write(source.join("open.log"), b"TMNLOG1\n").unwrap();

        assert_eq!(prepare_logs(&mut a, &source, &[active_id].into_iter().collect()), None);
        sync(&mut a, &backups).unwrap();
        b.record_connection(HistoryEntry { label: "device-local".into(), ..HistoryEntry::default() });
        sync(&mut b, &backups).unwrap();

        let synced = b.logs(false);
        assert_eq!(synced.len(), 1);
        assert_eq!((&synced[0].device[..], &synced[0].comment[..]), ("Test device", "first note"));
        assert!(!target.join("completed.log").exists());
        assert!(restore(&cloud.validate().unwrap(), &target, synced[0].log.as_ref().unwrap()).unwrap());
        assert_eq!(session_log::read(&target, "completed", &[7; 32]).unwrap(), (b"test output".to_vec(), None));

        let b_id = synced[0].id;
        assert!(b.save_log_comment(b_id, "from second device"));
        sync(&mut b, &backups).unwrap();
        sync(&mut a, &backups).unwrap();
        assert_eq!(a.history_entry_mut(a_id).unwrap().comment, "from second device");
        assert!(b.history.iter().any(|entry| entry.label == "device-local"));
        assert!(a.history.iter().any(|entry| entry.id == active_id));

        b.history.retain(|entry| entry.id != b_id);
        sync(&mut b, &backups).unwrap();
        sync(&mut a, &backups).unwrap();
        assert!(a.history.iter().all(|entry| entry.id != a_id));
        assert!(source.join("completed.log").exists());
    }

    #[test]
    fn meta_keeps_a_legacy_text_key() {
        let legacy = STANDARD.encode("0123456789abcdef0123456789abcdef");
        let value = meta(&entry(1, "a"), "a.log", &legacy);
        assert_eq!(value["session_log_data"]["secretKey"], "0123456789abcdef0123456789abcdef");
        assert!(meta(&entry(1, "a"), "a.log", &STANDARD.encode([0u8; 32]))["session_log_data"]["secretKey"].is_null());
    }

    /// Against a running cloud-sync-server: `TERMIAN_TEST_SYNC_URL` and
    /// `TERMIAN_TEST_SYNC_TOKEN`, then `cargo test -- --ignored live_server`.
    #[test]
    #[ignore]
    fn live_server_round_trip() {
        let url = std::env::var("TERMIAN_TEST_SYNC_URL").expect("TERMIAN_TEST_SYNC_URL");
        let token = std::env::var("TERMIAN_TEST_SYNC_TOKEN").expect("TERMIAN_TEST_SYNC_TOKEN");
        let config = crate::cloud::CloudConfig { token, ..config(&url) };
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("source");
        let received = dir.path().join("received");
        let backups = crate::store::Store::new(dir.path().join("vault.bin"), [3; 32]).cloud_backups();
        fs::create_dir_all(&logs).unwrap();
        let mut source = Vault { cloud: Some(config.clone()), ..Vault::default() };
        let mut target = source.clone();
        let name = uuid::Uuid::new_v4().to_string();
        source.record_connection(entry(0, &name));
        let mut file = b"TMNLOG1\n".to_vec();
        file.extend(session_log::encode_record(b"test output", &[7; 32]));
        fs::write(logs.join(session_log::file_name(&name)), file).unwrap();
        assert_eq!(crate::self_cloud::prepare_logs(&mut source, &logs, &Default::default()), None);
        crate::self_cloud::sync(&mut source, &backups).unwrap();
        crate::self_cloud::sync(&mut target, &backups).unwrap();
        let remote = target.history.iter().find(|row| row.log.as_ref().is_some_and(|log| log.name == session_log::file_name(&name))).unwrap();
        assert!(restore(&config.validate().unwrap(), &received, remote.log.as_ref().unwrap()).unwrap());
        assert_eq!(session_log::read(&received, &name, &[7; 32]).unwrap().0, b"test output");
    }
}