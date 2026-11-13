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
const NONCE: usize = 24;
const FLUSH_BYTES: usize = 64 * 1024;
const FLUSH_EVERY: Duration = Duration::from_secs(1);
/// Logs not written to for this long are deleted (FR-L5).
pub const RETENTION: Duration = Duration::from_secs(60 * 24 * 60 * 60);

pub const MISSING_LOG: &str = "Файл лога сессии не найден: он удалён (хранится 60 дней) или не был записан.";
pub const UNREADABLE_LOG: &str = "Не удалось расшифровать лог сессии";

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

pub fn new_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).expect("system random source");
    key
}

pub fn encode_record(chunk: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let mut nonce = [0u8; NONCE];
    getrandom::getrandom(&mut nonce).expect("system random source");
    let sealed = XSalsa20Poly1305::new(key.into()).encrypt(Nonce::from_slice(&nonce), chunk).expect("secretbox never fails to seal");
    let length = (NONCE + sealed.len()) as u32;
    [&length.to_be_bytes()[..], &nonce, &sealed].concat()
}

/// The terminal output in a log, and an error if part of it could not be decrypted.
pub fn decode(bytes: &[u8], key: &[u8; 32]) -> (Vec<u8>, Option<&'static str>) {
    let cipher = XSalsa20Poly1305::new(key.into());
    let mut out = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes[offset..].starts_with(MAGIC) {
            offset += MAGIC.len();
            continue;
        }
        let Some(head) = bytes.get(offset..offset + 4) else { break };
        let length = u32::from_be_bytes(head.try_into().expect("four bytes")) as usize;
        let Some(record) = bytes.get(offset + 4..offset + 4 + length).filter(|_| length > NONCE) else { break };
        match cipher.decrypt(Nonce::from_slice(&record[..NONCE]), &record[NONCE..]) {
            Ok(plain) => out.extend(plain),
            Err(_) => return (out, Some(UNREADABLE_LOG)),
        }
        offset += 4 + length;
    }
    (out, None)
}

/// Reads and decrypts a log; a missing file gives the legacy "not found" text.
pub fn read(dir: &Path, name: &str, key: &[u8; 32]) -> Result<(Vec<u8>, Option<&'static str>), String> {
    if !valid_name(name) {
        return Err(MISSING_LOG.into());
    }
    match std::fs::read(dir.join(format!("{name}.log"))) {
        Ok(bytes) => Ok(decode(&bytes, key)),
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
        let mut buffer = [0 as libc::c_char; 256];
        // SAFETY: the buffer is valid for its length; gethostname NUL-terminates on success.
        if unsafe { libc::gethostname(buffer.as_mut_ptr(), buffer.len()) } == 0 {
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
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(format!("{name}.log"))).map_err(|e| e.to_string())?;
        file.write_all(MAGIC).map_err(|e| e.to_string())?;
        let (sender, receiver) = channel::<Message>();
        std::thread::spawn(move || {
            let mut pending = Vec::new();
            let mut flush = |pending: &mut Vec<u8>| {
                if !pending.is_empty() {
                    let _ = file.write_all(&encode_record(pending, &key));
                    pending.clear();
                }
            };
            loop {
                match receiver.recv_timeout(FLUSH_EVERY) {
                    Ok(Message::Data(data)) => {
                        pending.extend(data);
                        if pending.len() >= FLUSH_BYTES {
                            flush(&mut pending);
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => flush(&mut pending),
                    Ok(Message::Finish) | Err(RecvTimeoutError::Disconnected) => {
                        flush(&mut pending);
                        return;
                    }
                }
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

#[cfg(test)]
mod tests {
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
        let bytes = wait_for(&path, |b| b.len() > MAGIC.len());
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
        let bytes = wait_for(&dir.path().join("idle.log"), |b| b.len() > MAGIC.len());
        assert_eq!(decode(&bytes, &key).0, b"prompt$ ");
        drop(log);
    }

    #[test]
    fn reopened_logs_torn_records_and_wrong_keys() {
        let key = [9u8; 32];
        let mut bytes = [MAGIC, &encode_record(b"one", &key), MAGIC, &encode_record(b"two", &key)].concat();
        assert_eq!(decode(&bytes, &key), (b"onetwo".to_vec(), None), "a second header after reopening is skipped");
        bytes.extend(&encode_record(b"torn", &key)[..10]);
        assert_eq!(decode(&bytes, &key), (b"onetwo".to_vec(), None), "a torn last record is ignored");
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
}
