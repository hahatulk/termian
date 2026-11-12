//! Local file system for the SFTP screen's "Local" panel. Electron gave the
//! renderer Node's `fs`; here src/bridge/local-fs.js exposes the same
//! callback API backed by these commands. Errors carry Node-style codes
//! (ENOENT, EACCES, EPERM, …) because the recovered UI branches on them.
//! Every command is async: on macOS the first access to a protected folder
//! can block while the system privacy prompt waits for the user.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

const MAX_CHUNK: u32 = 1024 * 1024;

fn error_code(error: &io::Error) -> &'static str {
    // macOS privacy (TCC) denials surface as EPERM, plain permissions as EACCES.
    if error.raw_os_error() == Some(1) {
        return "EPERM";
    }
    match error.kind() {
        io::ErrorKind::NotFound => "ENOENT",
        io::ErrorKind::PermissionDenied => "EACCES",
        io::ErrorKind::AlreadyExists => "EEXIST",
        io::ErrorKind::NotADirectory => "ENOTDIR",
        io::ErrorKind::IsADirectory => "EISDIR",
        io::ErrorKind::DirectoryNotEmpty => "ENOTEMPTY",
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidFilename => "EINVAL",
        _ => "EIO",
    }
}

fn failure(error: io::Error, path: &str) -> Value {
    json!({"code": error_code(&error), "message": format!("{}: {error}", path)})
}

fn invalid(message: &str) -> Value {
    json!({"code": "EINVAL", "message": message})
}

fn checked(path: &str) -> Result<PathBuf, Value> {
    if path.is_empty() || path.len() > 4096 || path.contains('\0') {
        return Err(invalid("invalid path"));
    }
    Ok(PathBuf::from(path))
}

/// Forward slashes and no `\\?\` prefix: the UI joins paths POSIX-style.
fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        let text = text.strip_prefix(r"\\?\").unwrap_or(&text).replace('\\', "/");
        return text;
    }
    #[cfg(not(windows))]
    text
}

pub fn home_dir() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn millis(time: io::Result<std::time::SystemTime>) -> f64 {
    time.ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|elapsed| elapsed.as_millis() as f64)
        .unwrap_or(0.0)
}

fn stat_value(meta: &fs::Metadata) -> Value {
    #[cfg(unix)]
    let (mode, uid, gid) = {
        use std::os::unix::fs::MetadataExt;
        (meta.mode(), meta.uid(), meta.gid())
    };
    #[cfg(not(unix))]
    let (mode, uid, gid) = {
        let kind = if meta.is_dir() { 0o040000 } else if meta.file_type().is_symlink() { 0o120000 } else { 0o100000 };
        let access = if meta.permissions().readonly() { 0o555 } else { 0o755 };
        (kind | access, 0u32, 0u32)
    };
    json!({
        "mode": mode,
        "uid": uid,
        "gid": gid,
        "size": meta.len(),
        "atimeMs": millis(meta.accessed()),
        "mtimeMs": millis(meta.modified()),
        "isDirectory": meta.is_dir(),
        "isFile": meta.is_file(),
        "isSymbolicLink": meta.file_type().is_symlink(),
    })
}

fn run(op: &str, path: &str, target: Option<&str>, mode: Option<u32>) -> Result<Value, Value> {
    if op == "home" {
        return Ok(json!(display_path(&home_dir())));
    }
    let file = checked(path)?;
    let fail = |error: io::Error| failure(error, path);
    match op {
        "realpath" => Ok(json!(display_path(&fs::canonicalize(&file).map_err(fail)?))),
        "readdir" => {
            let mut names = Vec::new();
            for entry in fs::read_dir(&file).map_err(fail)? {
                names.push(entry.map_err(fail)?.file_name().to_string_lossy().to_string());
            }
            Ok(json!(names))
        }
        "lstat" => Ok(stat_value(&fs::symlink_metadata(&file).map_err(fail)?)),
        "stat" => Ok(stat_value(&fs::metadata(&file).map_err(fail)?)),
        "access" => fs::symlink_metadata(&file).map(|_| Value::Null).map_err(fail),
        "mkdir" => fs::create_dir(&file).map(|_| Value::Null).map_err(fail),
        "rmdir" => fs::remove_dir(&file).map(|_| Value::Null).map_err(fail),
        "unlink" => fs::remove_file(&file).map(|_| Value::Null).map_err(fail),
        "rename" => {
            let destination = checked(target.unwrap_or(""))?;
            fs::rename(&file, destination).map(|_| Value::Null).map_err(fail)
        }
        "chmod" => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = mode.ok_or_else(|| invalid("missing mode"))?;
                fs::set_permissions(&file, fs::Permissions::from_mode(mode & 0o7777)).map_err(fail)?;
            }
            #[cfg(not(unix))]
            let _ = mode;
            Ok(Value::Null)
        }
        "readlink" => Ok(json!(display_path(&fs::read_link(&file).map_err(fail)?))),
        "symlink" => {
            let original = checked(target.unwrap_or(""))?;
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(original, &file).map_err(fail)?;
            }
            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_file(original, &file).map_err(fail)?;
            }
            Ok(Value::Null)
        }
        // Start of a write stream: create or truncate, like fs.createWriteStream.
        "truncate" => {
            let handle = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&file).map_err(fail)?;
            #[cfg(unix)]
            if let Some(mode) = mode {
                use std::os::unix::fs::PermissionsExt;
                let _ = handle.set_permissions(fs::Permissions::from_mode(mode & 0o7777));
            }
            drop(handle);
            Ok(Value::Null)
        }
        _ => Err(invalid("unsupported operation")),
    }
}

#[tauri::command]
pub async fn local_fs(op: String, path: String, target: Option<String>, mode: Option<u32>) -> Result<Value, Value> {
    tauri::async_runtime::spawn_blocking(move || run(&op, &path, target.as_deref(), mode))
        .await
        .map_err(|error| invalid(&error.to_string()))?
}

fn read_chunk(path: &str, offset: u64, length: u32) -> Result<Vec<u8>, Value> {
    let file = checked(path)?;
    let mut handle = fs::File::open(&file).map_err(|error| failure(error, path))?;
    handle.seek(SeekFrom::Start(offset)).map_err(|error| failure(error, path))?;
    let mut buffer = Vec::with_capacity(length.min(MAX_CHUNK) as usize);
    handle
        .take(u64::from(length.min(MAX_CHUNK)))
        .read_to_end(&mut buffer)
        .map_err(|error| failure(error, path))?;
    Ok(buffer)
}

/// Raw bytes (an ArrayBuffer in JS); an empty answer means end of file.
#[tauri::command]
pub async fn local_fs_read(path: String, offset: u64, length: u32) -> Result<tauri::ipc::Response, Value> {
    tauri::async_runtime::spawn_blocking(move || read_chunk(&path, offset, length))
        .await
        .map_err(|error| invalid(&error.to_string()))?
        .map(tauri::ipc::Response::new)
}

fn write_chunk(path: &str, offset: u64, data: &str) -> Result<(), Value> {
    let file = checked(path)?;
    let bytes = STANDARD.decode(data).map_err(|_| invalid("invalid data"))?;
    let mut handle = fs::OpenOptions::new().write(true).open(&file).map_err(|error| failure(error, path))?;
    handle.seek(SeekFrom::Start(offset)).map_err(|error| failure(error, path))?;
    handle.write_all(&bytes).map_err(|error| failure(error, path))
}

#[tauri::command]
pub async fn local_fs_write(path: String, offset: u64, data: String) -> Result<(), Value> {
    tauri::async_runtime::spawn_blocking(move || write_chunk(&path, offset, &data))
        .await
        .map_err(|error| invalid(&error.to_string()))?
}

/// Folder access the Local panel may need, for the permission guide.
/// macOS asks the user once per protected folder; afterwards a denial is
/// only reversible in System Settings → Privacy & Security.
fn permission_status(path: &Path) -> &'static str {
    match fs::read_dir(path) {
        Ok(_) => "granted",
        Err(error) if error.kind() == io::ErrorKind::NotFound => "missing",
        Err(error) if matches!(error_code(&error), "EPERM" | "EACCES") => "denied",
        Err(_) => "unknown",
    }
}

fn permissions() -> Value {
    if !cfg!(target_os = "macos") {
        return json!([]);
    }
    let home = home_dir();
    const FILES: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_FilesAndFolders";
    const FULL_DISK: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles";
    let folder = |id: &str, title: &str, name: &str| {
        let path = home.join(name);
        json!({"id": id, "title": title, "path": display_path(&path), "status": permission_status(&path), "required": true, "settingsUrl": FILES})
    };
    let full_disk_probe = home.join("Library/Application Support/com.apple.TCC");
    json!([
        {"id": "home", "title": "Домашняя папка", "path": display_path(&home), "status": permission_status(&home), "required": true, "settingsUrl": FILES},
        folder("desktop", "Рабочий стол", "Desktop"),
        folder("documents", "Документы", "Documents"),
        folder("downloads", "Загрузки", "Downloads"),
        {"id": "fullDisk", "title": "Полный доступ к диску", "path": "~/Library и системные папки",
         "status": permission_status(&full_disk_probe), "required": false, "settingsUrl": FULL_DISK},
    ])
}

#[tauri::command]
pub async fn local_fs_permissions() -> Value {
    tauri::async_runtime::spawn_blocking(permissions).await.unwrap_or_else(|_| json!([]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("termian-local-fs-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn file_operations_round_trip_with_node_error_codes() {
        let dir = temp();
        let base = display_path(&dir);
        let file = format!("{base}/a.txt");
        run("truncate", &file, None, Some(0o640)).unwrap();
        write_chunk(&file, 0, &STANDARD.encode(b"hello ")).unwrap();
        write_chunk(&file, 6, &STANDARD.encode(b"world")).unwrap();
        assert_eq!(read_chunk(&file, 0, 1024).unwrap(), b"hello world");
        assert_eq!(read_chunk(&file, 6, 3).unwrap(), b"wor");
        assert!(read_chunk(&file, 11, 10).unwrap().is_empty());
        let stat = run("lstat", &file, None, None).unwrap();
        assert_eq!(stat["size"], 11);
        assert_eq!(stat["isDirectory"], false);
        #[cfg(unix)]
        assert_eq!(stat["mode"].as_u64().unwrap() & 0o777, 0o640);
        run("mkdir", &format!("{base}/sub"), None, None).unwrap();
        assert_eq!(run("mkdir", &format!("{base}/sub"), None, None).unwrap_err()["code"], "EEXIST");
        let names = run("readdir", &base, None, None).unwrap();
        let mut names: Vec<_> = names.as_array().unwrap().iter().map(|n| n.as_str().unwrap().to_string()).collect();
        names.sort();
        assert_eq!(names, ["a.txt", "sub"]);
        run("rename", &file, Some(&format!("{base}/sub/b.txt")), None).unwrap();
        assert_eq!(run("rmdir", &format!("{base}/sub"), None, None).unwrap_err()["code"], "ENOTEMPTY");
        assert_eq!(run("lstat", &file, None, None).unwrap_err()["code"], "ENOENT");
        run("unlink", &format!("{base}/sub/b.txt"), None, None).unwrap();
        run("rmdir", &format!("{base}/sub"), None, None).unwrap();
        assert_eq!(run("realpath", &base, None, None).unwrap(), json!(display_path(&fs::canonicalize(&dir).unwrap())));
        assert_eq!(run("lstat", "", None, None).unwrap_err()["code"], "EINVAL");
        fs::remove_dir_all(dir).unwrap();
    }
}
