//! macOS folder access for the SFTP Local panel (legacy `local_fs_permissions`
//! and `file-access-guide.js`). macOS asks once per protected folder; after a
//! denial access can only be granted in System Settings → Privacy & Security.

use std::io;
use std::path::{Path, PathBuf};

const FILES: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders";
const FULL_DISK: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Granted,
    Denied,
    Missing,
    Unknown,
}

impl Status {
    /// The legacy status label.
    pub fn label(self) -> &'static str {
        match self {
            Status::Granted => "Доступ есть",
            Status::Denied => "Нет доступа",
            Status::Missing => "Папки нет",
            Status::Unknown => "Не удалось проверить",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub title: &'static str,
    pub path: String,
    pub status: Status,
    pub required: bool,
    pub settings_url: &'static str,
}

/// Reading a folder is the probe; it also lets macOS show its one-time prompt.
pub fn status(path: &Path) -> Status {
    match std::fs::read_dir(path) {
        Ok(_) => Status::Granted,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Status::Missing,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Status::Denied,
        Err(_) => Status::Unknown,
    }
}

fn display(home: &Path, path: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".into(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The folders of the guide under `home`, as in legacy.
pub fn folders(home: &Path) -> Vec<Folder> {
    let folder = |title, path: PathBuf, required, settings_url| Folder {
        title,
        path: display(home, &path),
        status: status(&path),
        required,
        settings_url,
    };
    let mut list = vec![
        folder("Домашняя папка", home.to_path_buf(), true, FILES),
        folder("Рабочий стол", home.join("Desktop"), true, FILES),
        folder("Документы", home.join("Documents"), true, FILES),
        folder("Загрузки", home.join("Downloads"), true, FILES),
        folder("Полный доступ к диску", home.join("Library/Application Support/com.apple.TCC"), false, FULL_DISK),
    ];
    list[4].path = "~/Library и системные папки".into();
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_report_status_and_settings_links() {
        let home = std::env::temp_dir().join(format!("termian-file-access-{}", std::process::id()));
        std::fs::create_dir_all(home.join("Desktop")).unwrap();
        let list = folders(&home);
        assert_eq!(list.iter().map(|f| f.title).collect::<Vec<_>>(), ["Домашняя папка", "Рабочий стол", "Документы", "Загрузки", "Полный доступ к диску"]);
        assert_eq!((list[0].path.as_str(), list[0].status), ("~", Status::Granted));
        assert_eq!((list[1].path.as_str(), list[1].status), ("~/Desktop", Status::Granted));
        assert_eq!(list[2].status, Status::Missing);
        assert!(!list[4].required);
        assert_eq!(list[4].settings_url, FULL_DISK);
        assert_eq!(list[4].path, "~/Library и системные папки");
        std::fs::remove_dir_all(&home).unwrap();
    }
}
