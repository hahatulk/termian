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
    temp_dir: PathBuf,
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
        Self::new(std::env::temp_dir())
    }

    /// Reserves a temporary folder for a download of `remote_path`.
    pub fn prepare(&mut self, remote_path: &str) -> Result<&Edited, String> {
        let name = remote_path.rsplit('/').next().filter(|n| crate::files::valid_local_name(n))
            .ok_or("Invalid file name")?;
        let dir = tempfile::Builder::new().prefix("termian-edit-").tempdir_in(&self.root)
            .map_err(|e| e.to_string())?.keep();
        self.next += 1;
        self.files.push(Edited { id: self.next, remote_path: remote_path.to_owned(), local_path: dir.join(name), baseline: None, temp_dir: dir, changed: false });
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
            let _ = std::fs::remove_dir_all(file.temp_dir);
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
        self.set(&extension(file_name), app)
    }

    /// Settings → SFTP → «File type associations»: extension and app, sorted.
    pub fn list(&self) -> Vec<(String, PathBuf)> {
        self.apps.iter().map(|(ext, app)| (ext.clone(), app.clone())).collect()
    }

    /// Another app for `ext`; an empty path is ignored, as the legacy field does.
    pub fn set(&mut self, ext: &str, app: PathBuf) -> Result<(), String> {
        if app.as_os_str().is_empty() {
            return Ok(());
        }
        self.apps.insert(ext.to_owned(), app);
        self.save()
    }

    pub fn forget(&mut self, ext: &str) -> Result<(), String> {
        self.apps.remove(ext);
        self.save()
    }

    fn save(&self) -> Result<(), String> {
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
#[cfg(not(windows))]
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

#[cfg(windows)]
pub fn open_path(path: &Path, app: Option<&Path>) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC};
    if let Some(app) = app {
        return std::process::Command::new(app).arg(path).spawn().map(|_| ()).map_err(|e| format!("{}: {e}", path.display()));
    }
    let mut filename: Vec<u16> = path.as_os_str().encode_wide().collect();
    if filename.contains(&0) {
        return Err("Invalid file name".into());
    }
    filename.push(0);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI | SEE_MASK_NOASYNC,
        lpVerb: windows_sys::core::w!("open"),
        lpFile: filename.as_ptr(),
        nShow: windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        ..Default::default()
    };
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(format!("{}: {}", path.display(), std::io::Error::last_os_error()));
    }
    Ok(())
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

        let mut apps = again;
        apps.remember("a.log", "/Applications/Console.app".into()).unwrap();
        assert_eq!(apps.list().iter().map(|(ext, _)| ext.as_str()).collect::<Vec<_>>(), ["log", "txt"]);
        apps.set("txt", "/Applications/Nova.app".into()).unwrap();
        apps.set("txt", PathBuf::new()).unwrap();
        apps.forget("log").unwrap();
        let saved = Associations::load(dir.path().join("apps.json"));
        assert_eq!(saved.list(), vec![("txt".to_owned(), PathBuf::from("/Applications/Nova.app"))]);
    }
}
