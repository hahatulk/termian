//! The native client's OpenSSH `known_hosts` file as the Known Hosts screen
//! sees it: entries to list and remove, and «Import» from `~/.ssh/known_hosts`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The file SSH connections check and learn host keys in; the demo
/// (`TERMIAN_DEMO`) keeps its own.
pub fn default_path() -> PathBuf {
    if std::env::var_os("TERMIAN_DEMO").is_some() {
        return std::env::temp_dir().join("termian-native-demo").join("known_hosts");
    }
    dirs::data_dir().unwrap_or_default().join("termian-native").join("known_hosts")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// `host`, `[host]:port`, or several separated by commas (hashed names stay as they are).
    pub hostnames: String,
    /// `ssh-ed25519`, `ecdsa-sha2-nistp256`, …
    pub key_type: String,
    /// The line as written, used to remove it.
    pub line: String,
}

/// Host key lines; comments, blank lines and malformed lines are skipped.
/// A `@cert-authority` / `@revoked` marker comes first when present.
pub fn parse(text: &str) -> Vec<Entry> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let mut fields = trimmed.split_whitespace();
            let mut hostnames = fields.next()?;
            if hostnames.starts_with('@') {
                hostnames = fields.next()?;
            }
            let key_type = fields.next()?;
            fields.next()?;
            Some(Entry { hostnames: hostnames.to_owned(), key_type: key_type.to_owned(), line: trimmed.to_owned() })
        })
        .collect()
}

pub fn read(path: &Path) -> Vec<Entry> {
    fs::read_to_string(path).map(|t| parse(&t)).unwrap_or_default()
}

fn writer_lock(path: &Path) -> io::Result<fs::File> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let mut name = path.as_os_str().to_owned();
    name.push(".lock");
    let lock = fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(name)?;
    lock.lock()?;
    Ok(lock)
}

fn publish(path: &Path, text: &str) -> io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut stage = tempfile::NamedTempFile::new_in(dir)?;
    stage.write_all(text.as_bytes())?;
    stage.as_file().sync_all()?;
    stage.persist(path)?;
    #[cfg(unix)]
    fs::File::open(dir)?.sync_all()?;
    Ok(())
}

pub(crate) fn learn(host: &str, port: u16, key: &russh::keys::PublicKey, path: &Path) -> io::Result<()> {
    let _lock = writer_lock(path)?;
    match check(host, port, key, path)? {
        Trust::Match => return Ok(()),
        Trust::Changed(_) => return Err(io::Error::other("host key changed while awaiting confirmation")),
        Trust::Unknown => {}
    }
    if host.is_empty() || host.contains(|c: char| c.is_whitespace() || c.is_control())
        || host.contains([',', '*', '?', '|', '!']) || host.starts_with(['@', '#']) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid SSH host name"));
    }
    let mut text = read_for_trust(path)?.unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') { text.push('\n'); }
    let host = if port == 22 { host.to_owned() } else { format!("[{host}]:{port}") };
    let public = key.to_openssh().map_err(io::Error::other)?;
    text.push_str(&format!("{host} {public}\n"));
    publish(path, &text)
}

/// Removes the lines of `entries` from the file.
pub fn remove(path: &Path, entries: &[Entry]) -> std::io::Result<()> {
    let _lock = writer_lock(path)?;
    let text = fs::read_to_string(path)?;
    let kept: Vec<&str> = text.lines().filter(|l| !entries.iter().any(|e| e.line == l.trim())).collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    publish(path, &out)
}

/// Adds the host key lines of `from` that `to` does not have yet; returns how many.
pub fn import(from: &Path, to: &Path) -> std::io::Result<usize> {
    let incoming = parse(&fs::read_to_string(from)?);
    let _lock = writer_lock(to)?;
    let mut text = read_for_trust(to)?.unwrap_or_default();
    let existing = parse(&text);
    let mut added = 0;
    for entry in incoming {
        if existing.iter().any(|e| e.line == entry.line) || text.lines().any(|l| l.trim() == entry.line) {
            continue;
        }
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&entry.line);
        text.push('\n');
        added += 1;
    }
    if added > 0 { publish(to, &text)?; }
    Ok(added)
}

pub(crate) enum Trust {
    Unknown,
    Match,
    Changed(usize),
}

fn read_for_trust(path: &Path) -> std::io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match fs::symlink_metadata(path) {
            Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(error),
        },
        Err(error) => Err(error),
    }
}

fn host_glob(pattern: &[u8], host: &[u8]) -> bool {
    let (mut p, mut h) = (0, 0);
    let mut star = None;
    while h < host.len() {
        match pattern.get(p) {
            Some(b'*') => {
                p += 1;
                star = Some((p, h));
            }
            Some(c) if *c == b'?' || c.eq_ignore_ascii_case(&host[h]) => {
                p += 1;
                h += 1;
            }
            _ => match star {
                Some((next, matched)) => {
                    h = matched + 1;
                    p = next;
                    star = Some((next, h));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|c| *c == b'*')
}

fn host_list_matches(names: &str, host: &str) -> bool {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use hmac::{Hmac, KeyInit, Mac};
    use sha1::Sha1;

    let mut positive = false;
    for entry in names.split(',') {
        let negative = entry.starts_with('!');
        let name = entry.strip_prefix('!').unwrap_or(entry);
        let matches = if let Some((salt, hash)) = name.strip_prefix("|1|").and_then(|value| value.split_once('|')) {
            let (Ok(salt), Ok(hash)) = (STANDARD.decode(salt), STANDARD.decode(hash)) else { continue };
            Hmac::<Sha1>::new_from_slice(&salt).is_ok_and(|mac| {
                mac.clone().chain_update(host).verify_slice(&hash).is_ok()
                    || mac.chain_update(host.to_ascii_lowercase()).verify_slice(&hash).is_ok()
            })
        } else {
            host_glob(name.as_bytes(), host.as_bytes())
        };
        if matches {
            if negative {
                return false;
            }
            positive = true;
        }
    }
    positive
}

pub(crate) fn check(host: &str, port: u16, key: &russh::keys::PublicKey, path: &Path) -> std::io::Result<Trust> {
    let Some(text) = read_for_trust(path)? else { return Ok(Trust::Unknown) };
    let host = if port == 22 { host.to_owned() } else { format!("[{host}]:{port}") };
    let mut changed = None;
    let mut matched = false;
    for (index, line) in text.lines().enumerate() {
        let mut fields = line.split_whitespace();
        let Some(mut names) = fields.next().filter(|field| !field.starts_with('#')) else { continue };
        let marker = if names.starts_with('@') {
            let marker = names;
            names = fields.next().unwrap_or_default();
            Some(marker)
        } else {
            None
        };
        if !host_list_matches(names, &host) || marker == Some("@cert-authority") {
            continue;
        }
        let invalid = || std::io::Error::new(std::io::ErrorKind::InvalidData, format!("invalid known_hosts entry on line {}", index + 1));
        let (kind, data) = (fields.next().ok_or_else(invalid)?, fields.next().ok_or_else(invalid)?);
        let recorded = russh::keys::PublicKey::from_openssh(&format!("{kind} {data}")).map_err(|_| invalid())?;
        let same = recorded.key_data() == key.key_data();
        match marker {
            Some("@revoked") if same => return Ok(Trust::Changed(index + 1)),
            Some("@revoked") => {}
            Some(_) => return Err(invalid()),
            None if same => matched = true,
            None => { changed.get_or_insert(index + 1); }
        }
    }
    Ok(if matched { Trust::Match } else { changed.map_or(Trust::Unknown, Trust::Changed) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# comment\n\nexample.com ssh-ed25519 AAAA1\n[10.0.0.1]:2222,alias ecdsa-sha2-nistp256 AAAA2 note\n@revoked bad.host ssh-rsa AAAA3\nbroken-line\n";

    #[test]
    fn parses_host_key_lines() {
        let entries = parse(SAMPLE);
        let names: Vec<_> = entries.iter().map(|e| (e.hostnames.as_str(), e.key_type.as_str())).collect();
        assert_eq!(names, [("example.com", "ssh-ed25519"), ("[10.0.0.1]:2222,alias", "ecdsa-sha2-nistp256"), ("bad.host", "ssh-rsa")]);
    }

    #[test]
    fn hostname_patterns_respect_exclusions_ports_and_revocation() {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use hmac::{Hmac, KeyInit, Mac};
        let salt = [3; 20];
        for host in ["node.example", "[node.example]:2222"] {
            let hash = Hmac::<sha1::Sha1>::new_from_slice(&salt).unwrap().chain_update(host).finalize().into_bytes();
            let pattern = format!("|1|{}|{}", STANDARD.encode(salt), STANDARD.encode(hash));
            assert!(host_list_matches(&pattern, &host.to_ascii_uppercase()));
            assert!(!host_list_matches(&pattern, "other.example"));
        }
        assert!(host_list_matches("[*.example]:2222", "[NODE.example]:2222"));
        assert!(!host_list_matches("[*.example]:2222", "[node.example]:22"));
        assert!(!host_list_matches("!node.example", "other.example"));
        assert!(!host_list_matches("*.example,!node.example", "NODE.example"));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("known_hosts");
        let key = crate::session::tests::server::host_key();
        let public = key.public_key().to_openssh().unwrap();
        let pinned = format!("node1.example {public}");
        let revoked = format!("@revoked !node2.example,NODE?.example {public}");
        for text in [format!("{pinned}\n{revoked}"), format!("{revoked}\n{pinned}")] {
            fs::write(&path, text).unwrap();
            assert!(matches!(check("node1.example", 22, key.public_key(), &path).unwrap(), Trust::Changed(_)));
            assert!(matches!(check("node2.example", 22, key.public_key(), &path).unwrap(), Trust::Unknown));
        }
    }

    #[test]
    fn imports_new_lines_once_and_removes_them() {
        let dir = tempfile::tempdir().unwrap();
        let (from, to) = (dir.path().join("ssh_known_hosts"), dir.path().join("native/known_hosts"));
        fs::write(&from, SAMPLE).unwrap();
        assert_eq!(import(&from, &to).unwrap(), 3);
        assert_eq!(import(&from, &to).unwrap(), 0, "nothing twice");
        let entries = read(&to);
        remove(&to, &entries[..1]).unwrap();
        assert_eq!(read(&to).iter().map(|e| e.hostnames.as_str()).collect::<Vec<_>>(), ["[10.0.0.1]:2222,alias", "bad.host"]);
        let key = crate::session::tests::server::host_key();
        assert!(matches!(check("host", 22, key.public_key(), &dir.path().join("missing")).unwrap(), Trust::Unknown));
        fs::write(&to, [0xff]).unwrap();
        assert_eq!(check("host", 22, key.public_key(), &to).err().unwrap().kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(import(&from, &to).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(fs::read(&to).unwrap(), [0xff]);
    }
}
