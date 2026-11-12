//! File panel model for the SFTP tab: one side (local or remote) with a
//! current directory, sorted listing, selection and back/forward history.

use chrono::{Local, TimeZone};

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub is_link: bool,
    pub size: u64,
    /// Seconds since the Unix epoch.
    pub modified: Option<i64>,
    /// Unix mode bits, when the side reports them.
    pub mode: Option<u32>,
}

impl Entry {
    /// "Kind" column: `folder`, `link`, or the file extension.
    pub fn kind(&self) -> String {
        if self.is_link {
            "link".into()
        } else if self.is_dir {
            "folder".into()
        } else {
            match self.name.rsplit_once('.') {
                Some((stem, ext)) if !stem.is_empty() => ext.to_lowercase(),
                _ => "file".into(),
            }
        }
    }

    pub fn size_label(&self) -> String {
        if self.is_dir { "- -".into() } else { format_size(self.size) }
    }

    pub fn modified_label(&self) -> String {
        self.modified.map(format_time).unwrap_or_default()
    }

    /// `drwxr-xr-x`, shown under remote names.
    pub fn permissions_label(&self) -> Option<String> {
        let mode = self.mode?;
        let kind = if self.is_link { 'l' } else if self.is_dir { 'd' } else { '-' };
        let bits: String = (0..9)
            .map(|i| if mode & (0o400 >> i) != 0 { ['r', 'w', 'x'][i % 3] } else { '-' })
            .collect();
        Some(format!("{kind}{bits}"))
    }
}

/// Decimal units as in the legacy UI: `7.00 Bytes`, `12.74 kB`, `3.10 MB`.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["Bytes", "kB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000. && unit < UNITS.len() - 1 {
        value /= 1000.;
        unit += 1;
    }
    format!("{value:.2} {}", UNITS[unit])
}

/// `11/12/2026, 11:28 PM` in local time.
pub fn format_time(seconds: i64) -> String {
    Local.timestamp_opt(seconds, 0).single().map(|t| t.format("%-m/%-d/%Y, %-I:%M %p").to_string()).unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Modified,
    Size,
    Kind,
}

/// Joins remote-style paths (`/` separated) without touching the local OS rules.
pub fn join(dir: &str, name: &str) -> String {
    if dir.ends_with('/') { format!("{dir}{name}") } else { format!("{dir}/{name}") }
}

pub fn parent(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.rsplit_once('/') {
        Some(("", _)) => Some("/".into()),
        Some((parent, _)) => Some(parent.into()),
        None => None,
    }
}

/// Path components for the breadcrumb, with their full paths.
pub fn crumbs(path: &str) -> Vec<(String, String)> {
    let mut out = vec![("/".to_owned(), "/".to_owned())];
    let mut current = String::new();
    for part in path.split('/').filter(|p| !p.is_empty()) {
        current = format!("{current}/{part}");
        out.push((part.to_owned(), current.clone()));
    }
    out
}

#[derive(Debug)]
pub struct Panel {
    pub path: String,
    entries: Vec<Entry>,
    pub sort: SortKey,
    pub descending: bool,
    pub filter: String,
    /// Dotfiles are hidden unless this is set, as in the legacy panel.
    pub show_hidden: bool,
    pub selected: Option<String>,
    back: Vec<String>,
    forward: Vec<String>,
}

impl Panel {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.into(),
            entries: Vec::new(),
            sort: SortKey::Name,
            descending: false,
            filter: String::new(),
            show_hidden: false,
            selected: None,
            back: Vec::new(),
            forward: Vec::new(),
        }
    }

    pub fn set_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries.into_iter().filter(|e| e.name != "." && e.name != "..").collect();
        if self.selected.as_ref().is_some_and(|s| !self.entries.iter().any(|e| &e.name == s)) {
            self.selected = None;
        }
    }

    /// Visible rows: filtered, folders first, then by the sort column.
    pub fn rows(&self) -> Vec<&Entry> {
        let filter = self.filter.to_lowercase();
        let mut rows: Vec<&Entry> =
            self.entries
                .iter()
                .filter(|e| self.show_hidden || !e.name.starts_with('.'))
                .filter(|e| filter.is_empty() || e.name.to_lowercase().contains(&filter))
                .collect();
        rows.sort_by(|a, b| {
            let folders_first = b.is_dir.cmp(&a.is_dir);
            let by_column = match self.sort {
                SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortKey::Modified => a.modified.cmp(&b.modified),
                SortKey::Size => a.size.cmp(&b.size),
                SortKey::Kind => a.kind().cmp(&b.kind()),
            };
            let by_column = if self.descending { by_column.reverse() } else { by_column };
            folders_first.then(by_column).then_with(|| a.name.cmp(&b.name))
        });
        rows
    }

    /// Clicking the active column flips direction; another column sorts ascending.
    pub fn sort_by(&mut self, key: SortKey) {
        if self.sort == key {
            self.descending = !self.descending;
        } else {
            self.sort = key;
            self.descending = false;
        }
    }

    /// Changes directory, recording history; the caller then loads entries.
    pub fn navigate(&mut self, path: String) {
        if path != self.path {
            self.back.push(std::mem::replace(&mut self.path, path));
            self.forward.clear();
            self.reset_view();
        }
    }

    pub fn up(&mut self) -> bool {
        match parent(&self.path) {
            Some(parent) => {
                self.navigate(parent);
                true
            }
            None => false,
        }
    }

    pub fn go_back(&mut self) -> bool {
        self.step(true)
    }

    pub fn go_forward(&mut self) -> bool {
        self.step(false)
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    fn step(&mut self, back: bool) -> bool {
        let (from, to) = if back { (&mut self.back, &mut self.forward) } else { (&mut self.forward, &mut self.back) };
        let Some(path) = from.pop() else { return false };
        to.push(std::mem::replace(&mut self.path, path));
        self.reset_view();
        true
    }

    fn reset_view(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.filter.clear();
    }

    pub fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

/// Lists a local directory. Unreadable entries are skipped, not fatal.
pub fn list_local(path: &str) -> Result<Vec<Entry>, String> {
    let read = std::fs::read_dir(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(read
        .flatten()
        .filter_map(|item| {
            let link = item.file_type().ok()?.is_symlink();
            let meta = std::fs::metadata(item.path()).or_else(|_| item.metadata()).ok()?;
            #[cfg(unix)]
            let mode = Some(std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o777);
            #[cfg(not(unix))]
            let mode = None;
            Some(Entry {
                name: item.file_name().to_string_lossy().into_owned(),
                is_dir: meta.is_dir(),
                is_link: link,
                size: meta.len(),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64),
                mode,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool, size: u64, modified: i64) -> Entry {
        Entry { name: name.into(), is_dir, is_link: false, size, modified: Some(modified), mode: None }
    }

    fn names(panel: &Panel) -> Vec<&str> {
        panel.rows().iter().map(|e| e.name.as_str()).collect()
    }

    fn panel() -> Panel {
        let mut p = Panel::new("/home/u");
        p.set_entries(vec![
            entry("README.md", false, 3250, 30),
            entry("tauri", true, 0, 50),
            entry("AGENTS.md", false, 12740, 20),
            entry(".", true, 0, 0),
            entry("docs", true, 0, 10),
        ]);
        p
    }

    #[test]
    fn folders_first_then_name() {
        assert_eq!(names(&panel()), ["docs", "tauri", "AGENTS.md", "README.md"]);
    }

    #[test]
    fn sorting_by_column_and_direction() {
        let mut p = panel();
        p.sort_by(SortKey::Modified);
        p.sort_by(SortKey::Modified);
        assert!(p.descending);
        assert_eq!(names(&p), ["tauri", "docs", "README.md", "AGENTS.md"]);
        p.sort_by(SortKey::Size);
        assert_eq!(names(&p), ["docs", "tauri", "README.md", "AGENTS.md"]);
    }

    #[test]
    fn dotfiles_are_hidden_by_default() {
        let mut p = panel();
        p.set_entries(vec![entry(".git", true, 0, 0), entry("src", true, 0, 0)]);
        assert_eq!(names(&p), ["src"]);
        p.show_hidden = true;
        assert_eq!(names(&p), [".git", "src"]);
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut p = panel();
        p.filter = "md".into();
        assert_eq!(names(&p), ["AGENTS.md", "README.md"]);
    }

    #[test]
    fn history_back_forward_and_up() {
        let mut p = panel();
        p.selected = Some("docs".into());
        p.navigate("/home/u/docs".into());
        assert_eq!(p.selected, None);
        assert!(p.up());
        assert_eq!(p.path, "/home/u");
        assert!(p.go_back());
        assert_eq!(p.path, "/home/u/docs");
        assert!(p.go_forward());
        assert_eq!(p.path, "/home/u");
        p.navigate("/".into());
        assert!(!p.can_go_forward());
        assert!(!p.up());
    }

    #[test]
    fn paths_and_breadcrumbs() {
        assert_eq!(parent("/opt/docker/"), Some("/opt".into()));
        assert_eq!(parent("/opt"), Some("/".into()));
        assert_eq!(parent("/"), None);
        assert_eq!(join("/", "etc"), "/etc");
        assert_eq!(join("/etc", "ssh"), "/etc/ssh");
        let labels: Vec<_> = crumbs("/Users/me").into_iter().map(|(l, _)| l).collect();
        assert_eq!(labels, ["/", "Users", "me"]);
    }

    #[test]
    fn labels_match_the_legacy_ui() {
        assert_eq!(format_size(7), "7.00 Bytes");
        assert_eq!(format_size(12_740), "12.74 kB");
        assert_eq!(format_size(47_990), "47.99 kB");
        let dir = entry("boot", true, 4096, 0);
        assert_eq!((dir.kind().as_str(), dir.size_label().as_str()), ("folder", "- -"));
        assert_eq!(entry("spec.md", false, 1, 0).kind(), "md");
        assert_eq!(entry(".bashrc", false, 1, 0).kind(), "file");
        let with_mode = Entry { mode: Some(0o755), ..dir };
        assert_eq!(with_mode.permissions_label().as_deref(), Some("drwxr-xr-x"));
    }

    #[test]
    fn lists_a_local_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        let mut entries = list_local(dir.path().to_str().unwrap()).unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!((entries[0].name.as_str(), entries[0].size, entries[0].is_dir), ("a.txt", 5, false));
        assert!(entries[1].is_dir);
        assert!(list_local("/nonexistent/termian").is_err());
    }
}
