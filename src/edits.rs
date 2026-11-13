//! Remote files opened in a local app, as the legacy SFTP panel does it:
//! the file is downloaded to a temporary copy, the copy is watched for saves
//! and a saved copy is offered for upload. Also the apps chosen with
//! "Open with…", remembered per file extension.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub type EditId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct Edited {
    pub id: EditId,
    pub remote_path: String,
    pub local_path: PathBuf,
    /// Modification time when the download finished; `None` until then.
    baseline: Option<SystemTime>,
    pub changed: bool,
}

impl Edited {
    pub fn name(&self) -> String {
        self.local_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn local_dir(&self) -> String {
        self.local_path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn size(&self) -> u64 {
        std::fs::metadata(&self.local_path).map(|m| m.len()).unwrap_or(0)
    }

    pub fn modified(&self) -> Option<i64> {
        let time = std::fs::metadata(&self.local_path).and_then(|m| m.modified()).ok()?;
        time.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64)
    }
}

/// Temporary copies of remote files, one folder each so names never clash.
pub struct Edits {
    root: PathBuf,
    files: Vec<Edited>,
    next: EditId,
}

impl Edits {
    pub fn new(root: PathBuf) -> Self {
        Self { root, files: Vec::new(), next: 0 }
    }

    /// Default place: a folder in the system temp directory.
    pub fn in_temp_dir() -> Self {
        Self::new(std::env::temp_dir().join("termian-native-sftp"))
    }

    /// Reserves a temporary folder for a download of `remote_path`.
    pub fn prepare(&mut self, remote_path: &str) -> Result<&Edited, String> {
        self.next += 1;
        let name = remote_path.rsplit('/').next().filter(|n| !n.is_empty()).ok_or("no file name")?;
        let dir = self.root.join(format!("{}-{}", std::process::id(), self.next));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        self.files.push(Edited { id: self.next, remote_path: remote_path.to_owned(), local_path: dir.join(name), baseline: None, changed: false });
        Ok(self.files.last().expect("just pushed"))
    }

    pub fn get(&self, id: EditId) -> Option<&Edited> {
        self.files.iter().find(|f| f.id == id)
    }

    /// The download finished: later saves count as changes.
    pub fn downloaded(&mut self, id: EditId) {
        if let Some(file) = self.files.iter_mut().find(|f| f.id == id) {
            file.baseline = mtime(&file.local_path);
            file.changed = false;
        }
    }

    /// The copy was uploaded: it matches the server again.
    pub fn uploaded(&mut self, id: EditId) {
        self.downloaded(id);
    }

    /// Checks the copies for saves; returns whether anything changed state.
    pub fn poll(&mut self) -> bool {
        let mut any = false;
        for file in &mut self.files {
            let Some(baseline) = file.baseline else { continue };
            let changed = mtime(&file.local_path).is_some_and(|now| now != baseline);
            if changed != file.changed {
                file.changed = changed;
                any = true;
            }
        }
        any
    }

    pub fn changed(&self) -> Vec<&Edited> {
        self.files.iter().filter(|f| f.changed).collect()
    }

    /// Stops tracking a copy and deletes it.
    pub fn discard(&mut self, id: EditId) {
        if let Some(index) = self.files.iter().position(|f| f.id == id) {
            let file = self.files.remove(index);
            if let Some(dir) = file.local_path.parent() {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }

    pub fn discard_all(&mut self) {
        for id in self.files.iter().map(|f| f.id).collect::<Vec<_>>() {
            self.discard(id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Apps chosen with "Open with…", by lower-case extension, kept in a JSON file.
pub struct Associations {
    path: PathBuf,
    apps: BTreeMap<String, PathBuf>,
}

impl Associations {
    pub fn load(path: PathBuf) -> Self {
        let apps = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, String>>(&bytes).ok())
            .map(|map| map.into_iter().map(|(k, v)| (k, PathBuf::from(v))).collect())
            .unwrap_or_default();
        Self { path, apps }
    }

    /// `~/Library/Application Support/termian-native/file-apps.json` and the like.
    pub fn load_default() -> Self {
        let path = dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("termian-native").join("file-apps.json");
        Self::load(path)
    }

    pub fn app_for(&self, file_name: &str) -> Option<&Path> {
        self.apps.get(&extension(file_name)).map(PathBuf::as_path)
    }

    pub fn remember(&mut self, file_name: &str, app: PathBuf) -> Result<(), String> {
        self.apps.insert(extension(file_name), app);
        let map: BTreeMap<&String, String> = self.apps.iter().map(|(k, v)| (k, v.to_string_lossy().into_owned())).collect();
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&self.path, serde_json::to_vec_pretty(&map).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
}

/// Extension the way the legacy panel keys associations; files without one share `""`.
fn extension(file_name: &str) -> String {
    match file_name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_lowercase(),
        _ => String::new(),
    }
}

/// Opens a file in its default app, or in `app` when given.
pub fn open_path(path: &Path, app: Option<&Path>) -> Result<(), String> {
    let mut command = open_command(path, app);
    command.spawn().map(|_| ()).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(target_os = "macos")]
fn open_command(path: &Path, app: Option<&Path>) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    if let Some(app) = app {
        command.arg("-a").arg(app);
    }
    command.arg(path);
    command
}

#[cfg(target_os = "windows")]
fn open_command(path: &Path, app: Option<&Path>) -> std::process::Command {
    match app {
        Some(app) => {
            let mut command = std::process::Command::new(app);
            command.arg(path);
            command
        }
        None => {
            let mut command = std::process::Command::new("cmd");
            command.args(["/C", "start", ""]).arg(path);
            command
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn open_command(path: &Path, app: Option<&Path>) -> std::process::Command {
    let mut command = std::process::Command::new(app.unwrap_or(Path::new("xdg-open")));
    command.arg(path);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_copy_is_reported_until_uploaded() {
        let root = tempfile::tempdir().unwrap();
        let mut edits = Edits::new(root.path().to_path_buf());
        let file = edits.prepare("/etc/nginx/nginx.conf").unwrap().clone();
        assert_eq!(file.name(), "nginx.conf");
        std::fs::write(&file.local_path, b"old").unwrap();
        assert!(!edits.poll(), "nothing is tracked before the download finishes");
        edits.downloaded(file.id);

        let later = SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options().write(true).open(&file.local_path).unwrap().set_modified(later).unwrap();
        assert!(edits.poll());
        assert_eq!(edits.changed().len(), 1);
        assert!(!edits.poll(), "no news on the second look");

        edits.uploaded(file.id);
        assert!(edits.changed().is_empty());
        edits.discard(file.id);
        assert!(edits.is_empty() && !file.local_path.exists());
    }

    #[test]
    fn copies_of_the_same_name_do_not_clash() {
        let root = tempfile::tempdir().unwrap();
        let mut edits = Edits::new(root.path().to_path_buf());
        let a = edits.prepare("/a/config").unwrap().local_path.clone();
        let b = edits.prepare("/b/config").unwrap().local_path.clone();
        assert_ne!(a, b);
        assert!(edits.prepare("/").is_err());
    }

    #[test]
    fn open_with_choices_are_remembered_per_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("apps.json");
        let mut apps = Associations::load(path.clone());
        apps.remember("Notes.TXT", "/Applications/TextEdit.app".into()).unwrap();
        let again = Associations::load(path);
        assert_eq!(again.app_for("todo.txt"), Some(Path::new("/Applications/TextEdit.app")));
        assert_eq!(again.app_for("image.png"), None);
        assert_eq!(again.app_for(".bashrc"), None);
    }
}
