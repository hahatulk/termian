//! Terminal session logs (Logs screen). The renderer encrypts each log with
//! its own key (src/bridge/session-log.js, libtermius `terminalOutput`);
//! here they are only stored as opaque files in `session-logs-v2/`, pruned
//! after 60 days and silently backed up to the self-hosted cloud.
//!
//! The cloud copy is separate from the vault snapshot: it never changes the
//! snapshot, its revision or the sync status in the window header.

use crate::{native, termian_data_dir};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const DIR_NAME: &str = "session-logs-v2";
/// Logs older than this (by last write) are deleted locally and on the server.
pub const RETENTION: Duration = Duration::from_secs(60 * 24 * 60 * 60);
const RETENTION_CHECK: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_READ_BYTES: u64 = 512 * 1024 * 1024;
const LEDGER: &str = "session-log-uploads.json";
const CLOUD_FORMAT: &str = "termian-session-log";
static LEDGER_LOCK: Mutex<()> = Mutex::new(());

/// Names come from `${connection.id}.log`; anything else is refused so a
/// renderer can never reach outside the log directory.
pub fn valid_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && !name.starts_with('.')
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn logs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = termian_data_dir(app)?.join(DIR_NAME);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn log_path(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    if !valid_name(name) {
        return Err("EINVAL: invalid session log name".into());
    }
    Ok(logs_dir(app)?.join(name))
}

fn io_error(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => format!("ENOENT: {error}"),
        io::ErrorKind::PermissionDenied => format!("EACCES: {error}"),
        _ => format!("EIO: {error}"),
    }
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_millis() as u64).unwrap_or(0)
}

#[tauri::command]
pub async fn session_log_append(app: tauri::AppHandle, name: String, data: String) -> Result<(), String> {
    let path = log_path(&app, &name)?;
    let bytes = STANDARD.decode(data).map_err(|_| "EINVAL: invalid data")?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut file = fs::OpenOptions::new().create(true).append(true).open(&path).map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Raw file bytes (an ArrayBuffer in JS).
#[tauri::command]
pub async fn session_log_read(app: tauri::AppHandle, name: String) -> Result<tauri::ipc::Response, String> {
    let path = log_path(&app, &name)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut bytes = Vec::new();
        fs::File::open(&path)
            .map_err(io_error)?
            .take(MAX_READ_BYTES)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        Ok(tauri::ipc::Response::new(bytes))
    })
    .await
    .map_err(|error| error.to_string())?
}

/// The few `fs/promises` calls the background window makes on the log
/// directory (file statuses, folder size, deleting logs).
#[tauri::command]
pub async fn session_logs_fs(app: tauri::AppHandle, op: String, name: Option<String>) -> Result<Value, String> {
    let dir = logs_dir(&app)?;
    let path = match name.as_deref() {
        Some(name) => Some(log_path(&app, name)?),
        None => None,
    };
    tauri::async_runtime::spawn_blocking(move || match (op.as_str(), path) {
        ("readdir", None) => Ok(json!(list_logs(&dir).map_err(io_error)?
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>())),
        ("stat", Some(path)) => {
            let metadata = fs::metadata(path).map_err(io_error)?;
            Ok(json!({
                "size": metadata.len(),
                "mtimeMs": metadata.modified().map(millis).unwrap_or(0),
                "isFile": metadata.is_file(),
            }))
        }
        ("stat", None) => Ok(json!({"size": 0, "mtimeMs": 0, "isFile": false})),
        ("access", Some(path)) => fs::metadata(path).map(|_| Value::Null).map_err(io_error),
        ("access", None) => Ok(Value::Null),
        ("unlink", Some(path)) => fs::remove_file(path).map(|_| Value::Null).map_err(io_error),
        _ => Err("EINVAL: unsupported session log operation".into()),
    })
    .await
    .map_err(|error| error.to_string())?
}

fn list_logs(dir: &Path) -> io::Result<Vec<(String, fs::Metadata)>> {
    let mut logs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata()?;
        if metadata.is_file() && valid_name(&name) {
            logs.push((name, metadata));
        }
    }
    logs.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(logs)
}

/// Delete logs whose last write is older than `max_age`. Returns the names.
pub fn prune(dir: &Path, max_age: Duration, now: SystemTime) -> Vec<String> {
    let Ok(logs) = list_logs(dir) else { return Vec::new() };
    logs.into_iter()
        .filter(|(_, metadata)| {
            metadata.modified().ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > max_age)
        })
        .filter(|(name, _)| fs::remove_file(dir.join(name)).is_ok())
        .map(|(name, _)| name)
        .collect()
}

/// Retention runs at startup and then every few hours.
pub fn start_retention(app: &tauri::AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        if let Ok(dir) = logs_dir(&app) {
            let removed = prune(&dir, RETENTION, SystemTime::now());
            if !removed.is_empty() {
                forget_uploads(&app, &removed);
            }
        }
        std::thread::sleep(RETENTION_CHECK);
    });
}

// ------------------------------------------------------------ cloud copy --

/// What was last uploaded, so unchanged logs are not sent again. Contains
/// only file names, sizes and times — never keys or log contents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Uploaded {
    size: u64,
    modified: u64,
}

fn ledger_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(termian_data_dir(app)?.join(LEDGER))
}

fn read_ledger(app: &tauri::AppHandle) -> BTreeMap<String, Uploaded> {
    ledger_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_ledger(app: &tauri::AppHandle, ledger: &BTreeMap<String, Uploaded>) {
    if let (Ok(path), Ok(bytes)) = (ledger_path(app), serde_json::to_vec(ledger)) {
        let _ = fs::write(path, bytes);
    }
}

fn forget_uploads(app: &tauri::AppHandle, names: &[String]) {
    let Ok(_lock) = LEDGER_LOCK.lock() else { return };
    let mut ledger = read_ledger(app);
    for name in names {
        ledger.remove(name);
    }
    write_ledger(app, &ledger);
}

fn remember_upload(app: &tauri::AppHandle, name: &str, uploaded: Uploaded) {
    let Ok(_lock) = LEDGER_LOCK.lock() else { return };
    let mut ledger = read_ledger(app);
    ledger.insert(name.to_owned(), uploaded);
    write_ledger(app, &ledger);
}

/// `<server>/v1/logs/<name>` next to the configured snapshot endpoint.
pub fn log_endpoint(url: &url::Url, name: &str) -> url::Url {
    let mut endpoint = native::snapshot_endpoint(url);
    let path = endpoint.path().trim_end_matches("/snapshot").to_string();
    endpoint.set_path(&format!("{path}/logs/{name}"));
    endpoint
}

#[derive(Deserialize)]
pub struct LogItem {
    name: String,
    #[serde(default)]
    meta: Value,
}

enum Upload {
    Done,
    Unsupported,
}

fn upload(config: &native::ValidConfig, name: &str, meta: &Value, file: &[u8], modified: u64) -> Result<Upload, String> {
    // Sealed with the self-cloud passphrase like the snapshot: the server only
    // ever sees ciphertext. `meta` carries the per-log key from the vault, so
    // the copy can be restored on a fresh profile.
    let payload = json!({
        "format": CLOUD_FORMAT,
        "version": 1,
        "name": name,
        "modified": modified,
        "meta": meta,
        "file": STANDARD.encode(file),
    });
    let envelope = native::seal(&payload, &config.passphrase)?;
    let body = serde_json::to_vec(&envelope).map_err(|error| error.to_string())?;
    let response = ureq::request("PUT", log_endpoint(&config.url, name).as_str())
        .timeout(native::REQUEST_TIMEOUT)
        .set("authorization", &format!("Bearer {}", config.token))
        .set("content-type", "application/octet-stream")
        .send_bytes(&body);
    match response {
        Ok(_) => Ok(Upload::Done),
        // Servers older than the log API answer 404/405 for the route.
        Err(ureq::Error::Status(404 | 405, _)) => Ok(Upload::Unsupported),
        Err(ureq::Error::Status(code, response)) => Err(native::status_error("Отправка лога сессии", code, &response)),
        Err(error) => Err(error.to_string()),
    }
}

/// Upload new or changed logs. Silent by design: the caller ignores the
/// result except to back off, and nothing here touches the snapshot status.
#[tauri::command]
pub async fn session_logs_cloud_sync(app: tauri::AppHandle, items: Vec<LogItem>) -> Result<Value, String> {
    let stored = native::read_self_cloud(&app);
    let Ok(config) = native::validated(&stored) else {
        return Ok(json!({"configured": false}));
    };
    let dir = logs_dir(&app)?;
    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let ledger = read_ledger(&worker);
        let (mut uploaded, mut unchanged, mut skipped) = (0, 0, 0);
        for item in items.iter().filter(|item| valid_name(&item.name)) {
            let Ok(metadata) = fs::metadata(dir.join(&item.name)) else { continue };
            let current = Uploaded {
                size: metadata.len(),
                modified: metadata.modified().map(millis).unwrap_or(0),
            };
            if ledger.get(&item.name) == Some(&current) {
                unchanged += 1;
                continue;
            }
            let Ok(file) = fs::read(dir.join(&item.name)) else { continue };
            // A sealed payload is limited like a snapshot (47 MiB plaintext).
            if file.len() > 34 * 1024 * 1024 {
                skipped += 1;
                continue;
            }
            match upload(&config, &item.name, &item.meta, &file, current.modified)? {
                Upload::Done => {
                    remember_upload(&worker, &item.name, current);
                    uploaded += 1;
                }
                Upload::Unsupported => {
                    return Ok(json!({"configured": true, "unsupported": true, "uploaded": uploaded}));
                }
            }
        }
        Ok(json!({"configured": true, "uploaded": uploaded, "unchanged": unchanged, "skipped": skipped}))
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error: String| native::redact(error, &native::read_self_cloud(&app)))
}

/// Bring a missing local log back from the cloud copy. `false` when there is
/// none (not configured, never uploaded, or already pruned by retention).
#[tauri::command]
pub async fn session_log_cloud_restore(app: tauri::AppHandle, name: String) -> Result<bool, String> {
    let path = log_path(&app, &name)?;
    if path.exists() {
        return Ok(true);
    }
    let stored = native::read_self_cloud(&app);
    let Ok(config) = native::validated(&stored) else { return Ok(false) };
    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let response = ureq::request("GET", log_endpoint(&config.url, &name).as_str())
            .timeout(native::REQUEST_TIMEOUT)
            .set("authorization", &format!("Bearer {}", config.token))
            .call();
        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::Status(404 | 405, _)) => return Ok(false),
            Err(ureq::Error::Status(code, response)) => return Err(native::status_error("Загрузка лога сессии", code, &response)),
            Err(error) => return Err(error.to_string()),
        };
        let mut body = Vec::new();
        response
            .into_reader()
            .take(native::MAX_BYTES as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|error| error.to_string())?;
        let payload = native::open(&body, &config.passphrase)?;
        if payload["format"] != CLOUD_FORMAT || payload["name"] != name.as_str() {
            return Err("Недопустимая копия лога сессии".into());
        }
        let file = STANDARD
            .decode(payload["file"].as_str().unwrap_or(""))
            .map_err(|_| "Недопустимая копия лога сессии")?;
        let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&temporary, &file).map_err(io_error)?;
        // Keep the original age so retention still counts from the session.
        if let Some(modified) = payload["modified"].as_u64().filter(|value| *value > 0) {
            if let Ok(handle) = fs::File::options().write(true).open(&temporary) {
                let _ = handle.set_modified(UNIX_EPOCH + Duration::from_millis(modified));
            }
        }
        fs::rename(&temporary, &path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            io_error(error)
        })?;
        if let Ok(metadata) = fs::metadata(&path) {
            remember_upload(&worker, &name, Uploaded {
                size: metadata.len(),
                modified: metadata.modified().map(millis).unwrap_or(0),
            });
        }
        Ok(true)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error: String| native::redact(error, &native::read_self_cloud(&app)))
}

/// Local files a cleanup removes: logs written at or after `since` (0 = all)
/// that no remaining history entry still uses.
fn clear_local(dir: &Path, since: u64, keep: &[String]) -> (Vec<String>, Vec<String>) {
    let (mut removed, mut recent) = (Vec::new(), Vec::new());
    for (name, metadata) in list_logs(dir).unwrap_or_default() {
        let modified = metadata.modified().map(millis).unwrap_or(0);
        if since != 0 && modified < since {
            continue;
        }
        if keep.contains(&name) {
            recent.push(name);
        } else if fs::remove_file(dir.join(&name)).is_ok() {
            removed.push(name);
        }
    }
    (removed, recent)
}

/// Settings → «Логи сессий» → clear the last hour/day/week or everything.
/// The background window has already deleted the history entries; this
/// removes leftover files and the cloud copies uploaded since `since`.
#[tauri::command]
pub async fn session_logs_clear(app: tauri::AppHandle, since: u64, keep: Vec<String>) -> Result<Value, String> {
    let dir = logs_dir(&app)?;
    let stored = native::read_self_cloud(&app);
    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (removed, recent) = clear_local(&dir, since, &keep);
        // Kept logs written recently may lose their cloud copy below:
        // forget them so the next backup sends them again.
        forget_uploads(&worker, &[removed.clone(), recent].concat());
        let remote = match native::validated(&stored) {
            Err(_) => json!(null),
            Ok(config) => {
                let mut endpoint = log_endpoint(&config.url, "x");
                let path = endpoint.path().trim_end_matches("/x").to_string();
                endpoint.set_path(&path);
                endpoint.set_query(Some(&format!("since={since}")));
                match ureq::request("DELETE", endpoint.as_str())
                    .timeout(native::REQUEST_TIMEOUT)
                    .set("authorization", &format!("Bearer {}", config.token))
                    .call()
                {
                    Ok(response) => json!({"removed": response.into_string().ok()
                        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                        .and_then(|body| body["removed"].as_u64())}),
                    Err(ureq::Error::Status(404 | 405, _)) => json!({"unsupported": true}),
                    Err(ureq::Error::Status(code, response)) => json!({"error": native::status_error("Очистка логов в облаке", code, &response)}),
                    Err(error) => json!({"error": error.to_string()}),
                }
            }
        };
        Ok::<_, String>(json!({"local": removed.len(), "remote": remote}))
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| native::redact(error, &native::read_self_cloud(&app)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_cannot_leave_the_log_directory() {
        assert!(valid_name("6f9a1c2e-1b2c-4d5e-9f00-112233445566.log"));
        assert!(valid_name("42.log"));
        for bad in ["", ".log", "../x.log", "a/b.log", "a\\b.log", "..", "x.log\0", &"a".repeat(129)] {
            assert!(!valid_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn retention_removes_only_logs_older_than_sixty_days() {
        let dir = std::env::temp_dir().join(format!("termian-session-logs-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        for name in ["old.log", "fresh.log"] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        fs::write(dir.join(".ignored.tmp"), b"x").unwrap();
        let now = SystemTime::now();
        let old = fs::File::options().write(true).open(dir.join("old.log")).unwrap();
        old.set_modified(now - RETENTION - Duration::from_secs(60)).unwrap();
        let fresh = fs::File::options().write(true).open(dir.join("fresh.log")).unwrap();
        fresh.set_modified(now - RETENTION + Duration::from_secs(3600)).unwrap();

        assert_eq!(prune(&dir, RETENTION, now), vec!["old.log".to_string()]);
        assert!(!dir.join("old.log").exists());
        assert!(dir.join("fresh.log").exists());
        assert!(dir.join(".ignored.tmp").exists(), "only valid log names are touched");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn clearing_keeps_older_logs_and_logs_of_remaining_history() {
        let dir = std::env::temp_dir().join(format!("termian-session-logs-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let now = SystemTime::now();
        for (name, age) in [("old.log", 7200), ("new.log", 60), ("active.log", 30)] {
            fs::write(dir.join(name), b"x").unwrap();
            fs::File::options().write(true).open(dir.join(name)).unwrap()
                .set_modified(now - Duration::from_secs(age)).unwrap();
        }
        let since = millis(now - Duration::from_secs(3600));
        let (removed, recent) = clear_local(&dir, since, &["active.log".to_string()]);
        assert_eq!((removed, recent), (vec!["new.log".to_string()], vec!["active.log".to_string()]));
        assert!(dir.join("old.log").exists() && dir.join("active.log").exists());
        let (removed, _) = clear_local(&dir, 0, &[]);
        assert_eq!(removed, vec!["active.log".to_string(), "old.log".to_string()]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn log_endpoint_sits_next_to_the_snapshot_endpoint() {
        for (base, expected) in [
            ("https://sync.example", "https://sync.example/v1/logs/a.log"),
            ("https://sync.example/termian/", "https://sync.example/termian/v1/logs/a.log"),
            ("http://10.0.0.2:8080/v1/snapshot", "http://10.0.0.2:8080/v1/logs/a.log"),
        ] {
            assert_eq!(log_endpoint(&url::Url::parse(base).unwrap(), "a.log").as_str(), expected);
        }
    }
}
