//! Encrypted session logs in the legacy format (see legacy/src/bridge/session-log.js):
//! `TMNLOG1\n`, then records of `u32 BE length | 24-byte nonce | secretbox(chunk)`,
//! where length counts the nonce and the box. Each log has its own random key,
//! kept in the history entry of the vault. A torn last record is ignored.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

use crypto_secretbox::{aead::Aead, KeyInit, Nonce, XSalsa20Poly1305};

pub const MAGIC: &[u8] = b"TMNLOG1\n";
const V2_MAGIC: &[u8] = b"TMNLOG2\n";
const NONCE: usize = 24;
const FLUSH_BYTES: usize = 64 * 1024;
const FLUSH_EVERY: Duration = Duration::from_secs(1);
/// Logs not written to for this long are deleted (FR-L5).
pub const RETENTION: Duration = Duration::from_secs(60 * 24 * 60 * 60);

pub const MISSING_LOG: &str = "Файл лога сессии не найден: он удалён (хранится 60 дней) или не был записан.";
pub const UNREADABLE_LOG: &str = "Не удалось расшифровать лог сессии";
pub const INCOMPLETE_LOG: &str = "Запись лога не завершена или файл повреждён. Показана доступная часть.";

/// `<data>/termian-native/session-logs-v2`.
pub fn default_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("termian-native").join("session-logs-v2")
}

/// Names as the legacy writer accepts them: `[A-Za-z0-9._-]`, 1–128, no leading dot.
pub fn valid_name(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The file of a log: native names get `.log`; logs imported from legacy
/// keep their own file name, which already ends in `.log`.
pub fn file_name(name: &str) -> String {
    if name.ends_with(".log") { name.to_owned() } else { format!("{name}.log") }
}

pub fn new_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).expect("system random source");
    key
}

const CHAIN_MARKER: &[u8; 8] = b"TMN3OPEN";
const LOG_PREFIX: &str = "v3-";

pub fn new_name() -> String {
    format!("{LOG_PREFIX}{}", uuid::Uuid::new_v4())
}

fn requires_chain(name: &str) -> bool {
    name.strip_suffix(".log").unwrap_or(name).strip_prefix(LOG_PREFIX)
        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
}

fn transcript() -> sha2::Sha256 {
    use sha2::Digest;
    sha2::Sha256::new_with_prefix(b"Termian session log transcript v3\0")
}

fn final_nonce(hash: sha2::Sha256) -> [u8; NONCE] {
    use sha2::Digest;
    hash.finalize()[..NONCE].try_into().expect("24 digest bytes")
}

fn random_nonce() -> [u8; NONCE] {
    let mut nonce = [0u8; NONCE];
    getrandom::getrandom(&mut nonce).expect("system random source");
    nonce
}

pub fn encode_record(chunk: &[u8], key: &[u8; 32]) -> Vec<u8> {
    seal_record(chunk, key, random_nonce())
}

fn seal_record(chunk: &[u8], key: &[u8; 32], nonce: [u8; NONCE]) -> Vec<u8> {
    let sealed = XSalsa20Poly1305::new(key.into()).encrypt(Nonce::from_slice(&nonce), chunk).expect("secretbox never fails to seal");
    let length = (NONCE + sealed.len()) as u32;
    [&length.to_be_bytes()[..], &nonce, &sealed].concat()
}

/// The terminal output in a log, and an error if part of it could not be decrypted.
pub fn decode(bytes: &[u8], key: &[u8; 32]) -> (Vec<u8>, Option<&'static str>) {
    decode_log(bytes, key, false)
}

fn decode_log(bytes: &[u8], key: &[u8; 32], require_chain: bool) -> (Vec<u8>, Option<&'static str>) {
    use sha2::Digest;
    let first_nonce = MAGIC.len() + 4;
    let chained = bytes.starts_with(MAGIC)
        && bytes.get(MAGIC.len()..first_nonce) == Some(&((NONCE + 16) as u32).to_be_bytes())
        && bytes.get(first_nonce..first_nonce + CHAIN_MARKER.len()) == Some(CHAIN_MARKER);
    if require_chain && !chained {
        return (Vec::new(), Some(INCOMPLETE_LOG));
    }
    let cipher = XSalsa20Poly1305::new(key.into());
    let mut out = Vec::new();
    if bytes.is_empty() {
        return (out, Some(INCOMPLETE_LOG));
    }
    let mut framed = bytes.starts_with(V2_MAGIC);
    let mut offset = if framed { V2_MAGIC.len() } else { 0 };
    while offset < bytes.len() {
        let rest = &bytes[offset..];
        if !framed && rest.starts_with(MAGIC) {
            offset += MAGIC.len();
            continue;
        }
        let Some(head) = rest.get(..4) else { return (out, Some(INCOMPLETE_LOG)) };
        let length = u32::from_be_bytes(head.try_into().expect("four bytes")) as usize;
        let Some(record) = rest[4..].get(..length).filter(|_| length >= NONCE + 16) else {
            return (out, Some(INCOMPLETE_LOG));
        };
        let first_v1_record = !framed && offset == MAGIC.len() && bytes.starts_with(MAGIC);
        offset += 4 + length;
        match cipher.decrypt(Nonce::from_slice(&record[..NONCE]), &record[NONCE..]) {
            Ok(plain) if first_v1_record && plain.is_empty() => framed = true,
            Ok(plain) if framed && plain.is_empty() => {
                if chained {
                    let mut hash = transcript();
                    hash.update(&bytes[..offset - 4 - length]);
                    if record[..NONCE] != final_nonce(hash) {
                        return (Vec::new(), Some(UNREADABLE_LOG));
                    }
                }
                return (out, (offset != bytes.len()).then_some(INCOMPLETE_LOG));
            }
            Ok(plain) => out.extend(plain),
            Err(_) => return (out, Some(UNREADABLE_LOG)),
        }
    }
    (out, framed.then_some(INCOMPLETE_LOG))
}

/// Reads and decrypts a log; a missing file gives the legacy "not found" text.
pub fn read(dir: &Path, name: &str, key: &[u8; 32]) -> Result<(Vec<u8>, Option<&'static str>), String> {
    if !valid_name(name) {
        return Err(MISSING_LOG.into());
    }
    match std::fs::read(dir.join(file_name(name))) {
        Ok(bytes) => Ok(decode_log(&bytes, key, requires_chain(name))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(MISSING_LOG.into()),
        Err(error) => Err(error.to_string()),
    }
}

/// Deletes logs not modified within `max_age`; returns how many.
pub fn prune(dir: &Path, now: SystemTime, max_age: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let mut removed = 0;
    for entry in entries.flatten() {
        let old = entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > max_age));
        if old && entry.path().extension().is_some_and(|e| e == "log") && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// `Nov 13, 2026`, the first line of a Logs row (local time).
pub fn date_label(at: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(at, 0).single().map(|t| t.format("%b %d, %Y").to_string()).unwrap_or_default()
}

pub fn clock_label(connected: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(connected, 0).single()
        .map(|t| t.format("%H:%M").to_string()).unwrap_or_default()
}

pub fn duration_label(connected: i64, disconnected: i64) -> String {
    let seconds = disconnected.saturating_sub(connected).max(0);
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m {}s", seconds / 60, seconds % 60),
        3600..=86399 => format!("{}h {}m", seconds / 3600, seconds / 60 % 60),
        _ => format!("{}d {}h", seconds / 86400, seconds / 3600 % 24),
    }
}

/// Legacy `formatSessionLogTitle`: `Log: web, 21:05:12 Nov 14, 2026`.
pub fn log_tab_title(label: &str, connected: i64) -> String {
    use chrono::TimeZone;
    let time = chrono::Local.timestamp_opt(connected, 0).single().map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_default();
    format!("Log: {label}, {time} {}", date_label(connected))
}

/// `21:05 - 22:40`, with `(+1d)` when the session ran past midnight.
pub fn times_label(connected: i64, disconnected: i64) -> String {
    use chrono::TimeZone;
    let (Some(start), Some(end)) = (chrono::Local.timestamp_opt(connected, 0).single(), chrono::Local.timestamp_opt(disconnected, 0).single()) else {
        return String::new();
    };
    let days = (end.date_naive() - start.date_naive()).num_days();
    let range = format!("{} - {}", start.format("%H:%M"), end.format("%H:%M"));
    if days == 0 { range } else { format!("{range} (+{days}d)") }
}

/// This computer's name, shown under "No account" in Logs.
pub fn device_name() -> String {
    #[cfg(unix)]
    {
        let mut buffer = [0 as libc::c_char; 257];
        // SAFETY: the buffer is valid for its length; gethostname NUL-terminates on success.
        if unsafe { libc::gethostname(buffer.as_mut_ptr(), buffer.len() - 1) } == 0 {
            // SAFETY: as above.
            return unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy().trim_end_matches(".local").to_owned();
        }
    }
    std::env::var("COMPUTERNAME").unwrap_or_default()
}

enum Message {
    Data(Vec<u8>),
    Finish,
}

/// Appends a session's output on a background thread, flushing every 64 KiB
/// or once a second. Dropping it finishes the log.
pub struct SessionLog {
    sender: Sender<Message>,
}

impl SessionLog {
    pub fn start(dir: &Path, name: &str, key: [u8; 32]) -> Result<Self, String> {
        if !valid_name(name) {
            return Err("invalid session log name".into());
        }
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(dir.join(format!("{name}.log"))).map_err(|e| e.to_string())?;
        use sha2::Digest;
        let mut start_nonce = random_nonce();
        start_nonce[..CHAIN_MARKER.len()].copy_from_slice(CHAIN_MARKER);
        let header = [MAGIC, &seal_record(&[], &key, start_nonce)].concat();
        let mut hash = transcript();
        hash.update(&header);
        file.write_all(&header).map_err(|e| e.to_string())?;
        let (sender, receiver) = channel::<Message>();
        std::thread::spawn(move || {
            let write_log = || -> std::io::Result<()> {
                let mut pending = Vec::new();
                loop {
                    let (flush, finished) = match receiver.recv_timeout(FLUSH_EVERY) {
                        Ok(Message::Data(data)) => {
                            pending.extend(data);
                            (false, false)
                        }
                        Err(RecvTimeoutError::Timeout) => (true, false),
                        Ok(Message::Finish) | Err(RecvTimeoutError::Disconnected) => (true, true),
                    };
                    if !pending.is_empty() && (flush || pending.len() >= FLUSH_BYTES) {
                        let frame = encode_record(&pending, &key);
                        file.write_all(&frame)?;
                        hash.update(&frame);
                        pending.clear();
                    }
                    if finished {
                        file.write_all(&seal_record(&[], &key, final_nonce(hash)))?;
                        file.sync_data()?;
                        return Ok(());
                    }
                }
            };
            if let Err(error) = write_log() {
                eprintln!("[Termian] Session log recording stopped: {error}");
            }
        });
        Ok(Self { sender })
    }

    pub fn append(&self, data: &[u8]) {
        if !data.is_empty() {
            let _ = self.sender.send(Message::Data(data.to_vec()));
        }
    }
}

impl Drop for SessionLog {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Finish);
    }
}

/// Settings → Account → «Логи сессий»: id, button, confirmation wording, period.
pub const PERIODS: [(&str, &str, &str, Duration); 4] = [
    ("hour", "За час", "за последний час", Duration::from_secs(3600)),
    ("day", "За день", "за последний день", Duration::from_secs(86_400)),
    ("week", "За неделю", "за последнюю неделю", Duration::from_secs(604_800)),
    ("all", "Все", "все", Duration::ZERO),
];

/// Unix ms from which a period clears; 0 clears everything.
pub fn period_start(period: Duration, now: SystemTime) -> u64 {
    if period.is_zero() {
        return 0;
    }
    now.checked_sub(period).and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64)
}

/// What a cleanup removed on this device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cleared {
    /// Connections removed from Logs (and Recent connections).
    pub entries: usize,
    /// Log files deleted.
    pub files: usize,
}

/// The legacy cleanup: connections started since `since` (Unix ms, 0 = all)
/// leave the history with their logs, then every other log file written since
/// then goes too, except logs of the connections that stay.
pub fn clear(history: &mut Vec<crate::vault::HistoryEntry>, dir: &Path, since: u64) -> Cleared {
    let started = |entry: &crate::vault::HistoryEntry| since == 0 || (entry.connected_at.max(0) as u64).saturating_mul(1000) >= since;
    let before = history.len();
    history.retain(|entry| !started(entry));
    let keep: std::collections::HashSet<String> = history.iter().filter_map(|e| e.log.as_ref()).map(|log| file_name(&log.name)).collect();
    let mut files = 0;
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let modified = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64);
        if name.ends_with(".log") && (since == 0 || modified >= since) && !keep.contains(&name) && std::fs::remove_file(entry.path()).is_ok() {
            files += 1;
        }
    }
    Cleared { entries: before - history.len(), files }
}

/// Settings → Logs → «Disable Logs»: the logs of this vault are deleted; the
/// connections stay in history without them. Returns how many files went.
pub fn delete_all(history: &mut [crate::vault::HistoryEntry], dir: &Path) -> usize {
    let mut files = 0;
    for entry in history.iter_mut() {
        if let Some(log) = entry.log.take() {
            files += usize::from(valid_name(&log.name) && std::fs::remove_file(dir.join(file_name(&log.name))).is_ok());
        }
    }
    files
}

/// Result line under the cleanup buttons; `remote` is the cloud's answer, if
/// the cloud is configured.
pub fn cleared_text(cleared: Cleared, remote: Option<Result<crate::cloud::LogsRemoved, String>>) -> String {
    let cloud = match remote {
        None => String::new(),
        Some(Err(error)) => format!(" Облако: {error}"),
        Some(Ok(crate::cloud::LogsRemoved::Unsupported)) => " Сервер не поддерживает удаление логов.".into(),
        Some(Ok(crate::cloud::LogsRemoved::Removed(n))) => format!(" В облаке удалено логов: {n}."),
    };
    format!("Удалено записей: {}, лишних файлов: {}.{cloud}", cleared.entries, cleared.files)
}

#[cfg(test)]
mod tests {

    #[test]
    fn log_tabs_are_titled_like_legacy() {
        use chrono::TimeZone;
        let at = chrono::Local.with_ymd_and_hms(2026, 11, 14, 21, 5, 12).unwrap().timestamp();
        assert_eq!(log_tab_title("web", at), "Log: web, 21:05:12 Nov 14, 2026");
    }
    use super::*;

    fn wait_for(path: &Path, predicate: impl Fn(&[u8]) -> bool) -> Vec<u8> {
        for _ in 0..50 {
            if let Ok(bytes) = std::fs::read(path) {
                if predicate(&bytes) {
                    return bytes;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("log was not written");
    }

    #[test]
    fn writes_what_the_reader_gets_back() {
        let dir = tempfile::tempdir().unwrap();
        let key = new_key();
        let log = SessionLog::start(dir.path(), "conn-1", key).unwrap();
        log.append(b"hello ");
        log.append(b"world\r\n");
        drop(log);
        let path = dir.path().join("conn-1.log");
        let bytes = wait_for(&path, |b| decode(b, &key) == (b"hello world\r\n".to_vec(), None));
        assert!(bytes.starts_with(MAGIC));
        assert!(!bytes.windows(5).any(|w| w == b"hello"), "output is encrypted");
        assert_eq!(read(dir.path(), "conn-1", &key).unwrap(), (b"hello world\r\n".to_vec(), None));
    }

    #[test]
    fn idle_output_is_flushed_within_a_second() {
        let dir = tempfile::tempdir().unwrap();
        let key = new_key();
        let log = SessionLog::start(dir.path(), "idle", key).unwrap();
        log.append(b"prompt$ ");
        let bytes = wait_for(&dir.path().join("idle.log"), |b| decode(b, &key).0 == b"prompt$ ");
        assert_eq!(decode(&bytes, &key), (b"prompt$ ".to_vec(), Some(INCOMPLETE_LOG)));
        drop(log);
    }

    #[test]
    fn closed_log_checks_the_whole_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let key = new_key();
        let name = new_name();
        let log = SessionLog::start(dir.path(), &name, key).unwrap();
        let output = vec![b'x'; FLUSH_BYTES];
        log.append(&output);
        log.append(b"end");
        drop(log);
        let path = dir.path().join(file_name(&name));
        let bytes = wait_for(&path, |bytes| bytes.len() > FLUSH_BYTES && decode_log(bytes, &key, true).1.is_none());
        let open_end = MAGIC.len() + 4 + NONCE + 16;
        let first_end = open_end + 4 + NONCE + 16 + FLUSH_BYTES;
        let closing = bytes.len() - 4 - NONCE - 16;
        let (a, b) = (&bytes[open_end..first_end], &bytes[first_end..closing]);
        for middle in [b.to_vec(), [b, a].concat(), [a, a, b].concat()] {
            std::fs::write(&path, [&bytes[..open_end], &middle, &bytes[closing..]].concat()).unwrap();
            assert_eq!(read(dir.path(), &name, &key).unwrap().1, Some(UNREADABLE_LOG));
        }
        std::fs::write(&path, [MAGIC, a].concat()).unwrap();
        assert_eq!(read(dir.path(), &name, &key).unwrap().1, Some(INCOMPLETE_LOG));
    }

    #[test]
    fn empty_file_and_interrupted_recordings_report_incomplete() {
        let key = new_key();
        let marker = encode_record(&[], &key);
        assert_eq!(decode(&[], &key), (Vec::new(), Some(INCOMPLETE_LOG)));
        assert_eq!(decode(MAGIC, &key), (Vec::new(), None));
        for header in [[MAGIC, &marker].concat(), V2_MAGIC.to_vec()] {
            assert_eq!(decode(&header, &key), (Vec::new(), Some(INCOMPLETE_LOG)));
            let finished = [header.as_slice(), &encode_record(&[], &key)].concat();
            assert_eq!(decode(&finished, &key), (Vec::new(), None));
            let body = [header.as_slice(), &encode_record(b"recorded", &key)].concat();
            assert_eq!(decode(&body, &key), (b"recorded".to_vec(), Some(INCOMPLETE_LOG)));
            let finished = [body.as_slice(), &encode_record(&[], &key)].concat();
            assert_eq!(decode(&finished, &key), (b"recorded".to_vec(), None));
        }
    }

    #[test]
    fn reopened_logs_torn_records_and_wrong_keys() {
        let key = [9u8; 32];
        let mut bytes = [MAGIC, &encode_record(b"one", &key), MAGIC, &encode_record(b"two", &key)].concat();
        assert_eq!(decode(&bytes, &key), (b"onetwo".to_vec(), None), "a second header after reopening is skipped");
        bytes.extend(&encode_record(b"torn", &key)[..10]);
        assert_eq!(decode(&bytes, &key), (b"onetwo".to_vec(), Some(INCOMPLETE_LOG)), "a torn last record is reported");
        assert_eq!(decode(&bytes, &[1u8; 32]).1, Some(UNREADABLE_LOG));
    }

    #[test]
    fn missing_logs_and_names_are_reported_like_the_legacy_reader() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read(dir.path(), "nope", &[0; 32]), Err(MISSING_LOG.into()));
        assert!(!valid_name(".hidden") && !valid_name("a/b") && valid_name("a.B_1-2"));
        assert!(SessionLog::start(dir.path(), "../escape", [0; 32]).is_err());
    }

    #[test]
    fn reads_a_record_written_by_the_legacy_client() {
        // Made with tweetnacl as legacy/src/bridge/session-log.js does: key 9s, nonce 7s.
        let hex = "544d4e4c4f47310a000000380707070707070707070707070707070707070707070707072d1c4aadfd2fe2c8ecb9f12866726f0a10214e73a882eb03e7bba08388fcd282";
        let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        assert_eq!(decode(&bytes, &[9; 32]), (b"legacy log chunk".to_vec(), None));
    }

    #[test]
    fn row_labels_match_the_legacy_logs() {
        use chrono::TimeZone;
        let start = chrono::Local.with_ymd_and_hms(2026, 11, 13, 21, 5, 0).unwrap().timestamp();
        assert_eq!(date_label(start), "Nov 13, 2026");
        assert_eq!(times_label(start, start + 95 * 60), "21:05 - 22:40");
        assert_eq!(times_label(start, start + 26 * 3600), "21:05 - 23:05 (+1d)");
        assert!(!device_name().is_empty());
    }

    #[test]
    fn old_logs_are_pruned() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.log"), b"x").unwrap();
        std::fs::write(dir.path().join("keep.txt"), b"x").unwrap();
        let later = SystemTime::now() + RETENTION + Duration::from_secs(60);
        assert_eq!(prune(dir.path(), later, RETENTION), 1);
        assert!(!dir.path().join("old.log").exists() && dir.path().join("keep.txt").exists());
        assert_eq!(prune(dir.path(), SystemTime::now(), RETENTION), 0);
    }

    #[test]
    fn cleanup_removes_recent_connections_and_their_logs() {
        use crate::vault::{HistoryEntry, LogRef};
        let dir = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        let now_s = now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64;
        let entry = |id: u64, connected_at: i64, log: &str| HistoryEntry { id, connected_at, log: Some(LogRef { name: log.into(), key: String::new() }), ..HistoryEntry::default() };
        let mut history = vec![entry(1, now_s - 60, "recent"), entry(2, now_s - 3 * 86_400, "old-but-written"), entry(3, now_s - 7200, "earlier")];
        for name in ["recent", "old-but-written", "earlier", "orphan"] {
            std::fs::write(dir.path().join(format!("{name}.log")), b"x").unwrap();
        }
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        let earlier = std::fs::File::options().write(true).open(dir.path().join("earlier.log")).unwrap();
        earlier.set_modified(now - Duration::from_secs(7200)).unwrap();

        let cleared = clear(&mut history, dir.path(), period_start(PERIODS[0].3, now));
        assert_eq!(cleared, Cleared { entries: 1, files: 2 }, "recent.log and the orphan written within the hour");
        assert_eq!(history.iter().map(|e| e.id).collect::<Vec<_>>(), [2, 3]);
        assert!(dir.path().join("old-but-written.log").exists(), "kept: its connection started earlier");
        assert!(dir.path().join("earlier.log").exists() && !dir.path().join("orphan.log").exists());

        let mut copy = history.clone();
        assert_eq!(delete_all(&mut copy, dir.path()), 2);
        assert!(copy.iter().all(|e| e.log.is_none()) && copy.len() == 2, "connections stay");
        for name in ["earlier", "old-but-written"] {
            std::fs::write(dir.path().join(format!("{name}.log")), b"x").unwrap();
        }
        assert_eq!(clear(&mut history, dir.path(), 0), Cleared { entries: 2, files: 2 });
        assert!(history.is_empty() && dir.path().join("notes.txt").exists());
        assert_eq!(period_start(Duration::ZERO, now), 0);
        assert_eq!(cleared_text(Cleared { entries: 3, files: 1 }, None), "Удалено записей: 3, лишних файлов: 1.");
        assert!(cleared_text(Cleared::default(), Some(Ok(crate::cloud::LogsRemoved::Removed(4)))).ends_with(" В облаке удалено логов: 4."));
        assert!(cleared_text(Cleared::default(), Some(Ok(crate::cloud::LogsRemoved::Unsupported))).ends_with("не поддерживает удаление логов."));
    }
}
