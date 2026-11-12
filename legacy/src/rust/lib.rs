mod keys;
mod local_fs;
mod native;
mod session_logs;
mod sftp;
mod ssh_client;
mod ssh_forward;
mod updater;

// SSH, SFTP and port forwarding: ssh_client.rs, sftp.rs, ssh_forward.rs
// (russh, driven by the recovered Termius connection flow). This file keeps
// the local PTY and serial terminals and wires the application together.
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine as _};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Sender},
        Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{Emitter, Manager};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SerialConfig {
    port: String,
    baud_rate: u32,
    data_bits: u8,
    stop_bits: u8,
    parity: String,
    flow_control: String,
}

impl SerialConfig {
    fn validate(&self) -> Result<(), String> {
        if self.port.is_empty()
            || self.port.len() > 260
            || self.port.chars().any(char::is_control)
            || !(75..=4_000_000).contains(&self.baud_rate)
            || !(5..=8).contains(&self.data_bits)
            || (self.stop_bits != 1 && self.stop_bits != 2)
            || !matches!(self.parity.as_str(), "none" | "even" | "odd")
            || !matches!(self.flow_control.as_str(), "none" | "xon/xoff" | "rts/cts")
        {
            return Err("Invalid serial settings: check port, baud, data bits, parity, stop bits and flow control".into());
        }
        Ok(())
    }

    fn open(&self) -> Result<serial2::SerialPort, String> {
        self.validate()?;
        // Only enumerated serial ports can be opened by this command. This
        // prevents the UI from passing arbitrary filesystem paths to Rust.
        let listed = serial2::SerialPort::available_ports().map_err(|err| err.to_string())?;
        if !listed
            .iter()
            .any(|candidate| candidate.to_string_lossy() == self.port)
        {
            return Err(
                "Serial port is not connected or was not found by the operating system".into(),
            );
        }
        let mut port = serial2::SerialPort::open(&self.port, |mut settings: serial2::Settings| {
            settings.set_raw();
            settings.set_baud_rate(self.baud_rate)?;
            settings.set_char_size(serial2::CharSize::try_from(self.data_bits).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
            })?);
            settings.set_stop_bits(serial2::StopBits::try_from(self.stop_bits).map_err(|err| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
            })?);
            settings.set_parity(serial2::Parity::try_from(self.parity.as_str()).map_err(
                |err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string()),
            )?);
            settings.set_flow_control(
                serial2::FlowControl::try_from(self.flow_control.as_str()).map_err(|err| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
                })?,
            );
            Ok(settings)
        })
        .map_err(|err| format!("Unable to open serial port {}: {err}", self.port))?;
        port.set_read_timeout(Duration::from_millis(140))
            .map_err(|err| err.to_string())?;
        port.set_write_timeout(Duration::from_secs(2))
            .map_err(|err| err.to_string())?;
        Ok(port)
    }
}

#[tauri::command]
fn list_serial_ports() -> Result<Vec<String>, String> {
    let mut ports = serial2::SerialPort::available_ports()
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|port| port.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    ports.sort();
    ports.dedup();
    Ok(ports)
}

fn start_serial_session(
    app: tauri::AppHandle,
    session_id: String,
    config: SerialConfig,
    receiver: mpsc::Receiver<Input>,
) -> Result<(), String> {
    let port = config.open()?;
    emit_event(&app, &session_id, "status", "connected");
    let mut buffer = [0u8; 4096];

    loop {
        loop {
            match receiver.try_recv() {
                Ok(Input::Data(bytes)) => {
                    port.write_all(&bytes)
                        .map_err(|err| format!("Serial write: {err}"))?;
                }
                Ok(Input::Resize(_, _)) => {} // Physical UART ports have no PTY size.
                Ok(Input::Close) | Err(mpsc::TryRecvError::Disconnected) => {
                    emit_event(&app, &session_id, "status", "disconnected");
                    return Ok(());
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        match port.read(&mut buffer) {
            Ok(n) if n > 0 => emit_event(
                &app,
                &session_id,
                "output",
                STANDARD_NO_PAD.encode(&buffer[..n]),
            ),
            Ok(_) => {}
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(err) => return Err(format!("Serial read: {err}")),
        }
    }
}

enum Input {
    Data(Vec<u8>),
    Resize(u16, u16),
    Close,
}

#[derive(Default)]
struct Sessions(Mutex<HashMap<String, Sender<Input>>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalEvent {
    session_id: String,
    kind: String,
    data: String,
}

/// Windows ships a portable EXE and an NSIS installer built from the same
/// binary; the bundler stamps "nsis" into the installed copy only.
pub(crate) fn installed_build() -> bool {
    cfg!(target_os = "windows")
        && matches!(tauri::utils::platform::bundle_type(), Some(tauri::utils::config::BundleType::Nsis))
}

/// The portable Windows EXE keeps its profile in `termian-data/` next to
/// itself; the installed app and macOS (whose bundle is not writable) use the
/// OS app data directory, which an uninstall or update does not touch.
fn data_dir_for(portable: bool, exe: Option<PathBuf>, app_data: Option<PathBuf>) -> Result<PathBuf, String> {
    if portable {
        Ok(exe.as_deref().and_then(Path::parent).ok_or("No executable directory")?.join("termian-data"))
    } else {
        app_data.ok_or_else(|| "No application data directory".into())
    }
}

fn termian_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let portable = cfg!(target_os = "windows") && !installed_build();
    let dir = data_dir_for(portable, std::env::current_exe().ok(), app.path().app_data_dir().ok())?;
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    Ok(dir)
}

fn emit_event(app: &tauri::AppHandle, session_id: &str, kind: &str, data: impl Into<String>) {
    let _ = app.emit(
        "termian-terminal",
        TerminalEvent {
            session_id: session_id.to_owned(),
            kind: kind.to_owned(),
            data: data.into(),
        },
    );
}

/// The shell configured in Settings → Terminal → Local Terminal Path, or the
/// platform default. Only an existing executable file is accepted.
fn local_shell_command(shell: Option<String>) -> Result<CommandBuilder, String> {
    let configured = shell.map(|path| path.trim().to_string()).filter(|path| !path.is_empty());
    if let Some(path) = configured {
        if path.len() > 1024 || !std::path::Path::new(&path).is_file() {
            return Err(format!("Local terminal shell was not found: {path}"));
        }
        let mut builder = CommandBuilder::new(&path);
        if cfg!(not(target_os = "windows")) {
            builder.arg("-l");
        }
        return Ok(builder);
    }
    #[cfg(target_os = "windows")]
    let command = CommandBuilder::new("powershell.exe");
    #[cfg(not(target_os = "windows"))]
    let command = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let mut builder = CommandBuilder::new(shell);
        builder.arg("-l");
        builder
    };
    Ok(command)
}

fn start_local_session(
    app: tauri::AppHandle,
    session_id: String,
    shell: Option<String>,
    cols: u16,
    rows: u16,
    receiver: mpsc::Receiver<Input>,
) -> Result<(), String> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| err.to_string())?;
    let command = local_shell_command(shell)?;
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| err.to_string())?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| err.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|err| err.to_string())?;
    let out_app = app.clone();
    let out_id = session_id.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => emit_event(
                    &out_app,
                    &out_id,
                    "output",
                    STANDARD_NO_PAD.encode(&buf[..n]),
                ),
            }
        }
    });
    emit_event(&app, &session_id, "status", "connected");
    for input in receiver {
        match input {
            Input::Data(bytes) => {
                if writer.write_all(&bytes).is_err() || writer.flush().is_err() {
                    break;
                }
            }
            Input::Resize(cols, rows) => {
                let _ = pair.master.resize(PtySize {
                    cols,
                    rows,
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
            Input::Close => break,
        }
    }
    let _ = child.kill();
    emit_event(&app, &session_id, "status", "disconnected");
    Ok(())
}

#[derive(Deserialize)]
struct TerminalSize {
    cols: u16,
    rows: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalConnection {
    #[serde(default)]
    serial: Option<SerialConfig>,
    #[serde(default)]
    shell: Option<String>,
}

#[tauri::command]
fn start_terminal(
    app: tauri::AppHandle,
    state: tauri::State<'_, Sessions>,
    session_id: String,
    connection: TerminalConnection,
    size: TerminalSize,
) -> Result<(), String> {
    let TerminalSize { cols, rows } = size;
    let TerminalConnection { serial, shell } = connection;
    if session_id.is_empty() || session_id.len() > 80 {
        return Err("Invalid session ID".into());
    }
    if !(40..=400).contains(&cols) || !(10..=160).contains(&rows) {
        return Err("Terminal dimensions are out of range".into());
    }
    if let Some(config) = &serial {
        config.validate()?;
    }
    let (sender, receiver) = mpsc::channel();
    {
        let mut sessions = state.0.lock().map_err(|err| err.to_string())?;
        if sessions.contains_key(&session_id) || sessions.len() >= 16 {
            return Err("Terminal session exists or the 16-session limit was reached".into());
        }
        sessions.insert(session_id.clone(), sender);
    }
    let worker_app = app.clone();
    thread::spawn(move || {
        let result = match serial {
            Some(config) => start_serial_session(worker_app.clone(), session_id.clone(), config, receiver),
            None => start_local_session(worker_app.clone(), session_id.clone(), shell, cols, rows, receiver),
        };
        if let Err(err) = result {
            emit_event(&worker_app, &session_id, "error", err);
        }
        if let Some(state) = worker_app.try_state::<Sessions>() {
            if let Ok(mut sessions) = state.0.lock() {
                sessions.remove(&session_id);
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn write_terminal(
    state: tauri::State<'_, Sessions>,
    session_id: String,
    data: String,
    // "base64" for binary serial input; plain text otherwise.
    encoding: Option<String>,
) -> Result<(), String> {
    if data.len() > 256 * 1024 {
        return Err("Terminal input is too large".into());
    }
    let bytes = match encoding.as_deref() {
        Some("base64") => base64::engine::general_purpose::STANDARD
            .decode(&data)
            .map_err(|_| "Invalid terminal input encoding")?,
        _ => data.into_bytes(),
    };
    let sessions = state.0.lock().map_err(|err| err.to_string())?;
    sessions
        .get(&session_id)
        .ok_or("Terminal session is disconnected")?
        .send(Input::Data(bytes))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn resize_terminal(
    state: tauri::State<'_, Sessions>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    if !(40..=400).contains(&cols) || !(10..=160).contains(&rows) {
        return Err("Terminal size is out of range".into());
    }
    state
        .0
        .lock()
        .map_err(|err| err.to_string())?
        .get(&session_id)
        .ok_or("Terminal session is disconnected")?
        .send(Input::Resize(cols, rows))
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn stop_terminal(state: tauri::State<'_, Sessions>, session_id: String) -> Result<(), String> {
    if let Some(sender) = state
        .0
        .lock()
        .map_err(|err| err.to_string())?
        .remove(&session_id)
    {
        let _ = sender.send(Input::Close);
    }
    Ok(())
}

#[tauri::command]
fn app_info() -> serde_json::Value {
    serde_json::json!({
        "version": option_env!("TERMIAN_RELEASE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
        "runtime": "Tauri 2",
        "platform": std::env::consts::OS,
        "architecture": std::env::consts::ARCH
    })
}

pub fn run() {
    // The vault lives in the webviews' IndexedDB. Keep the portable Windows
    // build self-contained: WebView2 profile data goes to termian-data/ next
    // to the EXE, like the rest of the profile. The installed app keeps
    // Tauri's default (%LOCALAPPDATA%\app.termian.desktop); macOS uses app data.
    #[cfg(target_os = "windows")]
    if !installed_build() && std::env::var_os("WEBVIEW2_USER_DATA_FOLDER").is_none() {
        if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("termian-data").join("webview"))) {
            if fs::create_dir_all(&dir).is_ok() {
                std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", dir);
            }
        }
    }
    tauri::Builder::default()
        .manage(Sessions::default())
        .manage(ssh_client::SshClients::default())
        .manage(sftp::SftpSessions::default())
        .manage(updater::UpdateState::default())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            updater::start(app.handle());
            session_logs::start_retention(app.handle());
            Ok(())
        })
        // The hidden background window (vault process) must not keep the app
        // alive once the primary window is gone.
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                // A downloaded installer update runs as the app quits.
                updater::install_pending();
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_serial_ports,
            start_terminal,
            write_terminal,
            resize_terminal,
            stop_terminal,
            app_info,
            ssh_client::ssh_client_connect,
            ssh_client::ssh_client_reply,
            ssh_client::ssh_client_close,
            ssh_client::ssh_client_shell,
            ssh_client::ssh_client_exec,
            ssh_client::ssh_client_write,
            ssh_client::ssh_client_resize,
            ssh_client::ssh_client_close_channel,
            ssh_forward::ssh_forward_start,
            sftp::sftp_start,
            sftp::sftp_stop,
            sftp::sftp_op,
            keys::ssh_generate_key,
            local_fs::local_fs,
            local_fs::local_fs_read,
            local_fs::local_fs_write,
            local_fs::local_fs_permissions,
            native::keychain_get,
            native::keychain_set,
            native::keychain_delete,
            native::keychain_find,
            native::open_external,
            native::quit_app,
            native::self_cloud_get_config,
            native::self_cloud_save_config,
            native::self_cloud_probe,
            native::self_cloud_upload,
            native::self_cloud_download,
            native::self_cloud_backup,
            native::self_cloud_fetch,
            native::self_cloud_set_auto_sync,
            session_logs::session_logs_clear,
            session_logs::session_log_append,
            session_logs::session_log_read,
            session_logs::session_logs_fs,
            session_logs::session_logs_cloud_sync,
            session_logs::session_log_cloud_restore,
            updater::update_status,
            updater::update_check,
            updater::update_restart,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_profile_sits_next_to_the_exe_and_installed_uses_app_data() {
        let exe = PathBuf::from("D:/apps/termian-1.2.0-win-x64.exe");
        let app_data = PathBuf::from("C:/Users/u/AppData/Roaming/app.termian.desktop");
        assert_eq!(data_dir_for(true, Some(exe.clone()), Some(app_data.clone())).unwrap(), PathBuf::from("D:/apps/termian-data"));
        assert_eq!(data_dir_for(false, Some(exe), Some(app_data.clone())).unwrap(), app_data);
        assert!(data_dir_for(false, None, None).is_err());
        assert!(!installed_build() || cfg!(target_os = "windows"), "only Windows has an installer build");
    }

    #[test]
    fn serial_configuration_accepts_common_settings_and_rejects_invalid_values() {
        let valid = SerialConfig {
            port: if cfg!(windows) {
                "COM5".into()
            } else {
                "/dev/ttyUSB0".into()
            },
            baud_rate: 115_200,
            data_bits: 8,
            stop_bits: 1,
            parity: "none".into(),
            flow_control: "none".into(),
        };
        assert!(valid.validate().is_ok());
        let mut bad = valid.clone();
        bad.baud_rate = 0;
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.data_bits = 9;
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.stop_bits = 3;
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.parity = "mark".into();
        assert!(bad.validate().is_err());
        bad = valid.clone();
        bad.flow_control = "unknown".into();
        assert!(bad.validate().is_err());
        bad = valid;
        bad.port = "../hosts.json".into();
        // A path with traversal cannot pass the enumerated-port check.
        // Validation prevents control bytes; open() further requires a real serial device.
        assert!(bad.open().is_err());
    }

}
