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
    /// Permission bits (with setuid, setgid and sticky), when the side reports them.
    pub mode: Option<u32>,
    /// Owning user and group names, for Edit Permissions.
    pub owner: Option<(String, String)>,
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
    /// Selected names, in the order they were picked.
    pub selected: Vec<String>,
    /// Last clicked name: the start of a Shift-click range.
    anchor: Option<String>,
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
            selected: Vec::new(),
            anchor: None,
            back: Vec::new(),
            forward: Vec::new(),
        }
    }

    pub fn set_entries(&mut self, entries: Vec<Entry>) {
        self.entries = entries.into_iter().filter(|e| e.name != "." && e.name != "..").collect();
        let entries = &self.entries;
        self.selected.retain(|name| entries.iter().any(|e| &e.name == name));
        if self.anchor.as_ref().is_some_and(|a| !self.selected.contains(a)) {
            self.anchor = None;
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
        self.clear_selection();
        self.filter.clear();
    }

    pub fn is_selected(&self, name: &str) -> bool {
        self.selected.iter().any(|s| s == name)
    }

    /// Plain click: only this row.
    pub fn select(&mut self, name: &str) {
        self.selected = vec![name.to_owned()];
        self.anchor = Some(name.to_owned());
    }

    /// Cmd-click (Ctrl on Windows and Linux): adds or removes one row.
    pub fn toggle(&mut self, name: &str) {
        if self.is_selected(name) {
            self.selected.retain(|s| s != name);
        } else {
            self.selected.push(name.to_owned());
        }
        self.anchor = Some(name.to_owned());
    }

    /// Shift-click: the visible rows from the last clicked one to this one.
    /// Like the legacy table, the clicked row becomes the next range start.
    pub fn select_range(&mut self, name: &str) {
        let names: Vec<String> = self.rows().iter().map(|e| e.name.clone()).collect();
        let Some(end) = names.iter().position(|n| n == name) else { return };
        let start = self.anchor.as_ref().and_then(|a| names.iter().position(|n| n == a)).unwrap_or(0);
        let (from, to) = if start <= end { (start, end) } else { (end, start) };
        self.selected = names[from..=to].to_vec();
        self.anchor = Some(name.to_owned());
    }

    pub fn select_all(&mut self) {
        self.selected = self.rows().iter().map(|e| e.name.clone()).collect();
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    /// Selected entries in listing order.
    pub fn selected_entries(&self) -> Vec<Entry> {
        self.rows().into_iter().filter(|e| self.is_selected(&e.name)).cloned().collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    pub fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

/// `Do you want to delete these files?` wording of the legacy confirmation.
pub fn delete_title(entries: &[Entry]) -> String {
    let folders = entries.iter().filter(|e| e.is_dir && !e.is_link).count();
    let files = entries.len() - folders;
    let what = |count: usize, one: &str, many: &str| if count == 1 { format!("this {one}") } else { format!("these {many}") };
    let subject = match (folders, files) {
        (0, n) => what(n, "file", "files"),
        (n, 0) => what(n, "folder", "folders"),
        _ => "these items".to_owned(),
    };
    format!("Do you want to delete {subject}?")
}

/// Error under the name field of Rename and New folder, as in the legacy
/// dialogs. `current` is the name being renamed, which may be kept.
pub fn name_error(name: &str, taken: &[String], current: Option<&str>) -> Option<&'static str> {
    if name.is_empty() {
        Some("Required")
    } else if Some(name) != current && taken.iter().any(|t| t == name) {
        Some("Name is already used")
    } else {
        None
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
            let mode = Some(std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o7777);
            #[cfg(not(unix))]
            let mode = None;
            #[cfg(unix)]
            let owner = {
                use std::os::unix::fs::MetadataExt;
                Some((user_name(meta.uid()), group_name(meta.gid())))
            };
            #[cfg(not(unix))]
            let owner = None;
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
                owner,
            })
        })
        .collect())
}

/// User name for a uid, or the number when it has none.
#[cfg(unix)]
pub fn user_name(uid: u32) -> String {
    let mut buffer = vec![0 as libc::c_char; 1024];
    let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
    let mut found = std::ptr::null_mut();
    // SAFETY: the buffers outlive the call; the result only points into them.
    let status = unsafe { libc::getpwuid_r(uid, &mut entry, buffer.as_mut_ptr(), buffer.len(), &mut found) };
    if status == 0 && !found.is_null() {
        // SAFETY: getpwuid_r succeeded, so pw_name is a NUL-terminated string in `buffer`.
        return unsafe { std::ffi::CStr::from_ptr(entry.pw_name) }.to_string_lossy().into_owned();
    }
    uid.to_string()
}

/// Group name for a gid, or the number when it has none.
#[cfg(unix)]
pub fn group_name(gid: u32) -> String {
    let mut buffer = vec![0 as libc::c_char; 1024];
    let mut entry: libc::group = unsafe { std::mem::zeroed() };
    let mut found = std::ptr::null_mut();
    // SAFETY: as in `user_name`.
    let status = unsafe { libc::getgrgid_r(gid, &mut entry, buffer.as_mut_ptr(), buffer.len(), &mut found) };
    if status == 0 && !found.is_null() {
        // SAFETY: getgrgid_r succeeded, so gr_name is a NUL-terminated string in `buffer`.
        return unsafe { std::ffi::CStr::from_ptr(entry.gr_name) }.to_string_lossy().into_owned();
    }
    gid.to_string()
}

/// User and group from an `ls -l` style SFTP long name:
/// `-rw-r--r--    1 root     staff    5 Nov 13 04:57 notes.txt`.
pub fn parse_owner(longname: &str) -> Option<(String, String)> {
    let mut fields = longname.split_whitespace();
    let (_, _, user, group) = (fields.next()?, fields.next()?, fields.next()?, fields.next()?);
    Some((user.to_owned(), group.to_owned()))
}

/// Read, write and execute for owner, group and others: the Edit Permissions grid.
pub const PERMISSION_BITS: [[u32; 3]; 3] = [[0o400, 0o200, 0o100], [0o040, 0o020, 0o010], [0o004, 0o002, 0o001]];

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool, size: u64, modified: i64) -> Entry {
        Entry { name: name.into(), is_dir, is_link: false, size, modified: Some(modified), mode: None, owner: None }
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
        p.select("docs");
        p.navigate("/home/u/docs".into());
        assert!(p.selected.is_empty());
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
    fn click_cmd_click_and_shift_click_select_like_the_legacy_table() {
        let mut p = panel();
        p.select("tauri");
        assert_eq!(p.selected, ["tauri"]);
        p.toggle("README.md");
        assert_eq!(p.selected, ["tauri", "README.md"]);
        p.toggle("tauri");
        assert_eq!(p.selected, ["README.md"]);
        p.select("docs");
        p.select_range("AGENTS.md");
        assert_eq!(p.selected, ["docs", "tauri", "AGENTS.md"]);
        p.select_range("tauri");
        assert_eq!(p.selected, ["tauri", "AGENTS.md"], "the next range starts at the last clicked row");
        p.select_all();
        assert_eq!(p.selected_entries().len(), 4);
        p.set_entries(vec![entry("docs", true, 0, 10)]);
        assert_eq!(p.selected, ["docs"], "vanished rows leave the selection");
    }

    #[test]
    fn delete_title_names_what_is_selected() {
        let (dir, file) = (entry("docs", true, 0, 0), entry("a.txt", false, 1, 0));
        assert_eq!(delete_title(std::slice::from_ref(&file)), "Do you want to delete this file?");
        assert_eq!(delete_title(&[file.clone(), file.clone()]), "Do you want to delete these files?");
        assert_eq!(delete_title(&[dir.clone(), dir.clone()]), "Do you want to delete these folders?");
        assert_eq!(delete_title(&[dir, file]), "Do you want to delete these items?");
    }

    #[test]
    fn owner_comes_from_the_long_name() {
        let line = "-rw-r--r--    1 root     staff           5 Nov 13 04:57 notes.txt";
        assert_eq!(parse_owner(line), Some(("root".into(), "staff".into())));
        assert_eq!(parse_owner(""), None);
    }

    #[cfg(unix)]
    #[test]
    fn local_entries_name_their_owner() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), b"x").unwrap();
        let entries = list_local(dir.path().to_str().unwrap()).unwrap();
        let (user, _) = entries[0].owner.clone().unwrap();
        assert_eq!(user, user_name(unsafe { libc::getuid() }));
        assert!(!user.chars().all(|c| c.is_ascii_digit()), "a real user has a name: {user}");
    }

    #[test]
    fn name_errors_match_the_legacy_dialogs() {
        let taken = ["docs".to_owned(), "a.txt".to_owned()];
        assert_eq!(name_error("", &taken, None), Some("Required"));
        assert_eq!(name_error("docs", &taken, None), Some("Name is already used"));
        assert_eq!(name_error("a.txt", &taken, Some("a.txt")), None, "renaming to the same name is allowed");
        assert_eq!(name_error("new", &taken, None), None);
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
