//! The native client's OpenSSH `known_hosts` file as the Known Hosts screen
//! sees it: entries to list and remove, and «Import» from `~/.ssh/known_hosts`.

use std::fs;
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

/// Removes the lines of `entries` from the file.
pub fn remove(path: &Path, entries: &[Entry]) -> std::io::Result<()> {
    let text = fs::read_to_string(path)?;
    let kept: Vec<&str> = text.lines().filter(|l| !entries.iter().any(|e| e.line == l.trim())).collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    fs::write(path, out)
}

/// Adds the host key lines of `from` that `to` does not have yet; returns how many.
pub fn import(from: &Path, to: &Path) -> std::io::Result<usize> {
    let incoming = parse(&fs::read_to_string(from)?);
    let mut text = fs::read_to_string(to).unwrap_or_default();
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
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(to, text)?;
    Ok(added)
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
    fn imports_new_lines_once_and_removes_them() {
        let dir = tempfile::tempdir().unwrap();
        let (from, to) = (dir.path().join("ssh_known_hosts"), dir.path().join("native/known_hosts"));
        fs::write(&from, SAMPLE).unwrap();
        assert_eq!(import(&from, &to).unwrap(), 3);
        assert_eq!(import(&from, &to).unwrap(), 0, "nothing twice");
        let entries = read(&to);
        remove(&to, &entries[..1]).unwrap();
        assert_eq!(read(&to).iter().map(|e| e.hostnames.as_str()).collect::<Vec<_>>(), ["[10.0.0.1]:2222,alias", "bad.host"]);
    }
}
