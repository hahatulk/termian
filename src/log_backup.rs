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
use crate::vault::HistoryEntry;

const FORMAT: &str = "termian-session-log";
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub uploaded: usize,
    pub unchanged: usize,
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
        "connected_at": entry.connected_at * 1000,
        "disconnectedAt": entry.disconnected_at.map(|t| t * 1000),
        "session_log_data": {"name": name, "secretKey": secret, "key": key},
    })
}

/// Uploads the logs of `history` that are new or changed since the last run.
pub fn backup(cloud: &Cloud, logs_dir: &Path, history: &[HistoryEntry]) -> Result<Outcome, String> {
    let ledger_file = ledger_path(logs_dir);
    if history.iter().all(|entry| entry.log.is_none()) {
        return Ok(Outcome::default());
    }
    let generation = cloud.log_generation()?;
    let prefix = format!("{}:", cloud.storage_key());
    let mut ledger = read_ledger(&ledger_file);
    let mut outcome = Outcome::default();
    for entry in history {
        let Some(log) = &entry.log else { continue };
        let name = session_log::file_name(&log.name);
        if !session_log::valid_name(&name) {
            continue;
        }
        let Ok(metadata) = fs::metadata(logs_dir.join(&name)) else { continue };
        let current = Uploaded { size: metadata.len(), modified: metadata.modified().map(millis).unwrap_or(0) };
        let entry_key = format!("{prefix}{name}");
        if ledger.get(&entry_key) == Some(&current) {
            outcome.unchanged += 1;
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
        if let Ok(bytes) = serde_json::to_vec(&ledger) {
            let _ = fs::write(&ledger_file, bytes);
        }
    }
    Ok(outcome)
}

/// Brings a missing log file back from the cloud; `false` when there is no copy.
pub fn restore(cloud: &Cloud, logs_dir: &Path, name: &str) -> Result<bool, String> {
    let name = session_log::file_name(name);
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
    let file = payload["file"].as_str().and_then(|f| STANDARD.decode(f).ok()).ok_or("the cloud copy has no log data")?;
    fs::create_dir_all(logs_dir).map_err(|e| e.to_string())?;
    fs::write(&path, file).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::tests::{config, server};
    use crate::vault::LogRef;

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

        assert_eq!(backup(&cloud, &logs, &history).unwrap(), Outcome { uploaded: 1, ..Outcome::default() });
        assert_eq!(backup(&cloud, &logs, &history).unwrap(), Outcome { unchanged: 1, ..Outcome::default() }, "nothing twice");
        fs::write(logs.join("a-1.log"), b"TMNLOG1 first and more").unwrap();
        assert_eq!(backup(&cloud, &logs, &history).unwrap().uploaded, 1, "a grown log goes again");

        fs::remove_file(logs.join("a-1.log")).unwrap();
        assert!(restore(&cloud, &logs, "a-1").unwrap());
        assert_eq!(fs::read(logs.join("a-1.log")).unwrap(), b"TMNLOG1 first and more");
        assert!(!restore(&cloud, &logs, "missing-2").unwrap(), "no copy in the cloud");
        assert!(restore(&cloud, &logs, "../x").is_err());
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
        let cloud = crate::cloud::CloudConfig { token, ..config(&url) }.validate().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("session-logs-v2");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("live-1.log"), b"TMNLOG1 live").unwrap();
        assert_eq!(backup(&cloud, &logs, &[entry(1, "live-1")]).unwrap().uploaded, 1);
        fs::remove_file(logs.join("live-1.log")).unwrap();
        assert!(restore(&cloud, &logs, "live-1").unwrap());
        assert_eq!(fs::read(logs.join("live-1.log")).unwrap(), b"TMNLOG1 live");
        assert_eq!(cloud.delete_logs(0).unwrap(), crate::cloud::LogsRemoved::Removed(1));
        assert!(!restore(&cloud, &logs.join("gone"), "live-1").unwrap());
    }
}