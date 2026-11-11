//! SFTP subsystem for the libtermius replacement (`SshClient.sftp`). The
//! renderer's file browser uses a handle-based API (listDir, open/read/write/
//! close, …) that maps one-to-one onto raw SFTP requests, so every operation
//! is a single command answering `{code, …}` with SFTP status codes (0 = ok).

use crate::ssh_client::{client_handle, SshClients};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use russh_sftp::client::{error::Error as SftpError, RawSftpSession};
use russh_sftp::protocol::{FileAttributes, OpenFlags, StatusCode};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const MAX_READ: u32 = 256 * 1024;
// Values the bridge exposes as fs.constants (src/bridge/node-shims.js).
const O_WRONLY: u32 = 1;
const O_RDWR: u32 = 2;
const O_CREAT: u32 = 64;
const O_EXCL: u32 = 128;
const O_TRUNC: u32 = 512;
const O_APPEND: u32 = 1024;

#[derive(Default)]
pub struct SftpSessions(Mutex<HashMap<String, Arc<RawSftpSession>>>);

fn status_code(error: &SftpError) -> u32 {
    match error {
        SftpError::Status(status) => status.status_code as u32,
        _ => StatusCode::Failure as u32,
    }
}

fn failure(error: SftpError) -> Value {
    json!({"code": status_code(&error), "message": error.to_string()})
}

fn attributes(attrs: &FileAttributes) -> Value {
    json!({
        "size": attrs.size.unwrap_or(0),
        "mode": attrs.permissions.unwrap_or(0),
        "uid": attrs.uid.unwrap_or(0),
        "gid": attrs.gid.unwrap_or(0),
        "atime": attrs.atime.unwrap_or(0),
        "mtime": attrs.mtime.unwrap_or(0),
    })
}

fn open_flags(flags: u32) -> OpenFlags {
    let mut result = if flags & O_RDWR != 0 {
        OpenFlags::READ | OpenFlags::WRITE
    } else if flags & O_WRONLY != 0 {
        OpenFlags::WRITE
    } else {
        OpenFlags::READ
    };
    if flags & O_CREAT != 0 {
        result |= OpenFlags::CREATE;
    }
    if flags & O_TRUNC != 0 {
        result |= OpenFlags::TRUNCATE;
    }
    if flags & O_EXCL != 0 {
        result |= OpenFlags::EXCLUDE;
    }
    if flags & O_APPEND != 0 {
        result |= OpenFlags::APPEND;
    }
    result
}

fn mode_attributes(mode: u32) -> FileAttributes {
    let mut attrs = FileAttributes::empty();
    attrs.permissions = Some(mode & 0o7777);
    attrs
}

#[tauri::command]
pub async fn sftp_start(
    clients: tauri::State<'_, SshClients>,
    sessions: tauri::State<'_, SftpSessions>,
    id: String,
    sftp: String,
) -> Result<(), String> {
    let (_client, handle) = client_handle(&clients, &id)?;
    let channel = handle.channel_open_session().await.map_err(|error| error.to_string())?;
    channel.request_subsystem(true, "sftp").await.map_err(|error| error.to_string())?;
    let session = RawSftpSession::new(channel.into_stream());
    session.init().await.map_err(|error| error.to_string())?;
    sessions
        .0
        .lock()
        .map_err(|_| "SFTP state is unavailable")?
        .insert(sftp, Arc::new(session));
    Ok(())
}

#[tauri::command]
pub fn sftp_stop(sessions: tauri::State<'_, SftpSessions>, sftp: String) {
    // Dropping the last reference closes the subsystem channel.
    if let Ok(mut map) = sessions.0.lock() {
        map.remove(&sftp);
    }
}

fn arg_str<'a>(args: &'a Value, name: &str) -> &'a str {
    args.get(name).and_then(Value::as_str).unwrap_or("")
}

fn arg_u64(args: &Value, name: &str) -> u64 {
    args.get(name).and_then(Value::as_u64).unwrap_or(0)
}

async fn run(session: &RawSftpSession, op: &str, args: &Value) -> Result<Value, SftpError> {
    let path = arg_str(args, "path");
    Ok(match op {
        "listDir" => {
            let handle = session.opendir(path).await?.handle;
            let mut entries = Vec::new();
            loop {
                match session.readdir(handle.as_str()).await {
                    Ok(name) => entries.extend(name.files.into_iter().map(|file| {
                        json!({"filename": file.filename, "longname": file.longname, "attrs": attributes(&file.attrs)})
                    })),
                    Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => break,
                    Err(error) => {
                        let _ = session.close(handle.as_str()).await;
                        return Err(error);
                    }
                }
            }
            let _ = session.close(handle.as_str()).await;
            json!({"entries": entries})
        }
        "realPath" => {
            let name = session.realpath(path).await?;
            json!({"path": name.files.first().map(|file| file.filename.clone()).unwrap_or_default()})
        }
        "readLink" => {
            let name = session.readlink(path).await?;
            json!({"path": name.files.first().map(|file| file.filename.clone()).unwrap_or_default()})
        }
        "stat" => json!({"attrs": attributes(&session.stat(path).await?.attrs)}),
        "lstat" => json!({"attrs": attributes(&session.lstat(path).await?.attrs)}),
        "symlink" => {
            session.symlink(path, arg_str(args, "target")).await?;
            json!({})
        }
        "removeDir" => {
            session.rmdir(path).await?;
            json!({})
        }
        "removeFile" => {
            session.remove(path).await?;
            json!({})
        }
        "rename" => {
            session.rename(path, arg_str(args, "target")).await?;
            json!({})
        }
        "makeDir" => {
            session.mkdir(path, mode_attributes(arg_u64(args, "mode") as u32)).await?;
            json!({})
        }
        "chmod" => {
            session.setstat(path, mode_attributes(arg_u64(args, "mode") as u32)).await?;
            json!({})
        }
        "open" => {
            let flags = open_flags(arg_u64(args, "flags") as u32);
            let mode = args.get("mode").and_then(Value::as_u64).filter(|mode| *mode != 0);
            let attrs = mode.map(|mode| mode_attributes(mode as u32)).unwrap_or_else(FileAttributes::empty);
            json!({"handle": session.open(path, flags, attrs).await?.handle})
        }
        "read" => {
            let length = (arg_u64(args, "length") as u32).clamp(1, MAX_READ);
            match session.read(arg_str(args, "handle"), arg_u64(args, "offset"), length).await {
                Ok(data) => json!({"data": STANDARD.encode(&data.data), "eof": data.data.is_empty()}),
                Err(SftpError::Status(status)) if status.status_code == StatusCode::Eof => json!({"data": "", "eof": true}),
                Err(error) => return Err(error),
            }
        }
        "write" => {
            let data = STANDARD.decode(arg_str(args, "data")).map_err(|_| SftpError::UnexpectedBehavior("invalid data".into()))?;
            session.write(arg_str(args, "handle"), arg_u64(args, "offset"), data).await?;
            json!({})
        }
        "close" => {
            session.close(arg_str(args, "handle")).await?;
            json!({})
        }
        other => return Err(SftpError::UnexpectedBehavior(format!("unsupported SFTP operation {other}"))),
    })
}

#[tauri::command]
pub async fn sftp_op(
    sessions: tauri::State<'_, SftpSessions>,
    sftp: String,
    op: String,
    args: Value,
) -> Result<Value, String> {
    let session = sessions
        .0
        .lock()
        .map_err(|_| "SFTP state is unavailable")?
        .get(&sftp)
        .cloned();
    let Some(session) = session else {
        return Ok(json!({"code": StatusCode::NoConnection as u32, "message": "SFTP session is closed"}));
    };
    Ok(match run(&session, &op, &args).await {
        Ok(mut value) => {
            value["code"] = json!(0);
            value
        }
        Err(error) => failure(error),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_flags_follow_bridge_constants() {
        assert_eq!(open_flags(0).bits(), OpenFlags::READ.bits());
        assert_eq!(
            open_flags(O_WRONLY | O_CREAT | O_TRUNC).bits(),
            (OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE).bits()
        );
        assert_eq!(
            open_flags(O_RDWR | O_APPEND).bits(),
            (OpenFlags::READ | OpenFlags::WRITE | OpenFlags::APPEND).bits()
        );
    }
}
