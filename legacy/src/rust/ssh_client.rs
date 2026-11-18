//! Native replacement for `SshClient` of the proprietary @termius/libtermius
//! module, built on russh (pure Rust, current KEX/host-key algorithms).
//!
//! The recovered Termius renderer drives a connection through callbacks: host
//! key verification, auth-method choice, keyboard-interactive prompts, logs.
//! Each callback becomes an event on the client's Tauri channel and the
//! renderer's answer comes back through `ssh_client_reply`. Host keys are
//! verified against the known-hosts list the renderer passes in; nothing is
//! trusted implicitly.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use russh::client::{self, Handle, KeyboardInteractiveAuthResponse, Msg};
use russh::keys::{decode_secret_key, HashAlg, PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate};
use russh::{ChannelMsg, ChannelWriteHalf, Disconnect, MethodKind, MethodSet};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_AUTH_ROUNDS: usize = 20;

#[derive(Default)]
pub struct SshClients(Mutex<HashMap<String, Arc<Client>>>);

pub(crate) enum ChannelCmd {
    Data(Vec<u8>),
    Resize(u32, u32),
    Eof,
    Close,
}

pub(crate) struct Client {
    events: Channel<Value>,
    pending: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    next_request: AtomicU64,
    handle: Mutex<Option<Arc<Handle<ClientHandler>>>>,
    channels: Mutex<HashMap<u32, mpsc::UnboundedSender<ChannelCmd>>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    /// Remote forwards: server-side port -> local target.
    remote_forwards: Mutex<HashMap<u32, (String, u16)>>,
    closed: AtomicBool,
    disconnected: AtomicBool,
    /// Enabled only for a one-shot Verbose (-v) retry.
    verbose: bool,
}

impl Client {
    pub(crate) fn emit(&self, event: Value) {
        if !self.closed.load(Ordering::SeqCst) {
            let _ = self.events.send(event);
        }
    }

    fn log(&self, severity: &str, message: impl Into<String>) {
        self.emit(json!({"kind": "log", "visibility": "user", "severity": severity, "message": message.into()}));
    }

    /// Verbose transport diagnostics; no passwords, private keys or answers.
    fn debug(&self, message: impl Into<String>) {
        if self.verbose {
            self.emit(json!({"kind": "log", "visibility": "debug", "severity": "info", "message": message.into()}));
        }
    }

    /// Ask the renderer and wait for its answer; `None` means it was cancelled.
    async fn request(&self, kind: &str, mut payload: Value) -> Option<Value> {
        let id = self.next_request.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().ok()?.insert(id, tx);
        payload["kind"] = json!(kind);
        payload["request"] = json!(id);
        self.emit(payload);
        rx.await.ok().filter(|value| !value.is_null())
    }

    fn handle(&self) -> Result<Arc<Handle<ClientHandler>>, String> {
        self.handle
            .lock()
            .map_err(|_| "SSH client state is unavailable".to_string())?
            .clone()
            .ok_or_else(|| "SSH session is not connected".to_string())
    }

    pub(crate) fn spawn(&self, task: impl std::future::Future<Output = ()> + Send + 'static) {
        let handle = tauri::async_runtime::spawn(task);
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.retain(|task| !task.inner().is_finished());
            tasks.push(handle);
        }
    }

    pub(crate) fn add_remote_forward(&self, port: u32, target: (String, u16)) {
        if let Ok(mut forwards) = self.remote_forwards.lock() {
            forwards.insert(port, target);
        }
    }

    pub(crate) fn remote_forward(&self, port: u32) -> Option<(String, u16)> {
        self.remote_forwards.lock().ok()?.get(&port).cloned()
    }

    fn mark_disconnected(&self) {
        if !self.disconnected.swap(true, Ordering::SeqCst) {
            self.emit(json!({"kind": "disconnected"}));
        }
    }

    fn shutdown(&self) {
        self.closed.store(true, Ordering::SeqCst);
        if let Ok(mut pending) = self.pending.lock() {
            pending.clear();
        }
        if let Ok(mut channels) = self.channels.lock() {
            for (_, tx) in channels.drain() {
                let _ = tx.send(ChannelCmd::Close);
            }
        }
        let handle = self.handle.lock().ok().and_then(|mut handle| handle.take());
        if let Some(handle) = handle {
            tauri::async_runtime::spawn(async move {
                let _ = handle.disconnect(Disconnect::ByApplication, "", "en").await;
            });
        }
        if let Ok(mut tasks) = self.tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}

// ---------------------------------------------------------------- connect --

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectOptions {
    host: String,
    port: u16,
    username: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    keys: Vec<KeyOption>,
    #[serde(default)]
    known_hosts: Vec<String>,
    /// Host name the known-hosts entry is recorded under (differs from
    /// `host` for chained/proxied connections).
    known_host_alias: Option<HostAlias>,
    #[serde(default)]
    keepalive_interval: u64,
    #[serde(default)]
    verbose: bool,
    #[serde(default)]
    environment: Vec<(String, String)>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyOption {
    #[serde(default)]
    private_key: String,
    #[serde(default)]
    passphrase: Option<String>,
}

#[derive(Deserialize)]
struct HostAlias {
    host: String,
    port: u16,
}

pub(crate) struct ClientHandler {
    client: Arc<Client>,
    known_host_name: String,
    known_hosts: Vec<String>,
}

fn known_host_name(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

fn key_type_label(key: &PublicKey) -> String {
    let name = key.algorithm().as_str().to_string();
    let label = name.trim_start_matches("ssh-").trim_start_matches("sk-");
    if label.starts_with("ecdsa") {
        "ECDSA".into()
    } else {
        label.split('@').next().unwrap_or(label).to_uppercase()
    }
}

enum KnownHostStatus {
    Trusted,
    Mismatch,
    NotFound,
}

/// Match OpenSSH-format entries (`hostnames keytype base64`) from the vault.
fn check_known_hosts(entries: &[String], host_name: &str, key: &PublicKey) -> KnownHostStatus {
    let Ok(offered) = key.to_openssh() else {
        return KnownHostStatus::NotFound;
    };
    let mut offered_parts = offered.split_whitespace();
    let (offered_type, offered_data) = (offered_parts.next().unwrap_or(""), offered_parts.next().unwrap_or(""));
    let mut mismatch = false;
    for entry in entries {
        let mut parts = entry.split_whitespace();
        let (Some(hosts), Some(kind), Some(data)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        if !hosts.split(',').any(|candidate| candidate == host_name) {
            continue;
        }
        if kind == offered_type && data == offered_data {
            return KnownHostStatus::Trusted;
        }
        if kind == offered_type {
            mismatch = true;
        }
    }
    if mismatch {
        KnownHostStatus::Mismatch
    } else {
        KnownHostStatus::NotFound
    }
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let key = match server_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(certificate) => certificate.public_key().clone().into(),
        };
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        let fingerprint = fingerprint.trim_start_matches("SHA256:").to_string();
        let key_type = key_type_label(&key);
        self.client.log("info", format!("Server host key: {key_type} SHA256:{fingerprint}"));
        let status = match check_known_hosts(&self.known_hosts, &self.known_host_name, &key) {
            KnownHostStatus::Trusted => {
                self.client.debug("debug1: Host key matches a trusted known_hosts entry");
                return Ok(true);
            },
            KnownHostStatus::Mismatch => "mismatch",
            KnownHostStatus::NotFound => "notfound",
        };
        self.client.debug(format!("debug1: Host key needs explicit confirmation ({status})"));
        let Ok(public_key) = key.to_openssh() else {
            return Ok(false);
        };
        let answer = self
            .client
            .request(
                "hostKey",
                json!({"status": status, "fingerprint": fingerprint, "publicKey": public_key, "keyType": key_type}),
            )
            .await;
        Ok(answer.and_then(|value| value.as_bool()).unwrap_or(false))
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<Msg>,
        _connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: client::ChannelOpenHandle,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        if self.client.remote_forward(connected_port).is_none() {
            reply.reject(russh::ChannelOpenFailure::AdministrativelyProhibited).await;
            return Ok(());
        }
        reply.accept().await;
        let client = self.client.clone();
        tauri::async_runtime::spawn(crate::ssh_forward::accept_remote(client, channel, connected_port));
        Ok(())
    }

    async fn disconnected(&mut self, reason: client::DisconnectReason<Self::Error>) -> Result<(), Self::Error> {
        match reason {
            client::DisconnectReason::ReceivedDisconnect(info) => {
                self.client.log("info", format!("Disconnected by server: {info:?}"));
            }
            client::DisconnectReason::Error(error) => {
                self.client.log("warning", format!("Connection closed: {error}"));
            }
        }
        self.client.mark_disconnected();
        Ok(())
    }
}

/// Pass-through stream recording the server identification line, which russh
/// consumes internally but Termius shows as the session's software version.
struct IdSniffer {
    inner: TcpStream,
    id: Arc<Mutex<Vec<u8>>>,
    done: bool,
}

impl AsyncRead for IdSniffer {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if !self.done {
            let chunk = buf.filled()[before..].to_vec();
            let mut done = false;
            if let Ok(mut id) = self.id.lock() {
                for byte in chunk {
                    if byte == b'\n' {
                        if id.starts_with(b"SSH-") {
                            done = true;
                            break;
                        }
                        id.clear();
                    } else if id.len() < 512 {
                        id.push(byte);
                    }
                }
            }
            self.done = done;
        }
        result
    }
}

impl AsyncWrite for IdSniffer {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

fn method_names(methods: &MethodSet) -> Vec<String> {
    methods
        .iter()
        .filter(|method| !matches!(method, MethodKind::None))
        .map(|method| <&str>::from(method).to_string())
        .collect()
}

fn auth_result(result: client::AuthResult, methods: &mut Vec<String>) -> bool {
    match result {
        client::AuthResult::Success => true,
        client::AuthResult::Failure { remaining_methods, .. } => {
            *methods = method_names(&remaining_methods);
            false
        }
    }
}

async fn try_key(
    handle: &mut Handle<ClientHandler>,
    user: &str,
    key: &KeyOption,
    methods: &mut Vec<String>,
) -> Result<bool, String> {
    let passphrase = key.passphrase.as_deref().filter(|value| !value.is_empty());
    let private = decode_secret_key(&key.private_key, passphrase).map_err(|error| format!("Cannot use the private key: {error}"))?;
    let hash = handle.best_supported_rsa_hash().await.ok().flatten().flatten();
    let result = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(private), hash))
        .await
        .map_err(|error| error.to_string())?;
    Ok(auth_result(result, methods))
}

async fn keyboard_interactive(
    client: &Client,
    handle: &mut Handle<ClientHandler>,
    user: &str,
    methods: &mut Vec<String>,
) -> Result<Option<bool>, String> {
    let mut response = handle
        .authenticate_keyboard_interactive_start(user, None)
        .await
        .map_err(|error| error.to_string())?;
    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(Some(true)),
            KeyboardInteractiveAuthResponse::Failure { remaining_methods, .. } => {
                *methods = method_names(&remaining_methods);
                return Ok(Some(false));
            }
            KeyboardInteractiveAuthResponse::InfoRequest { name, instructions, prompts } => {
                let answers = if prompts.is_empty() {
                    Vec::new()
                } else {
                    let prompts: Vec<Value> = prompts
                        .iter()
                        .map(|prompt| json!({"text": prompt.prompt, "echo": prompt.echo}))
                        .collect();
                    let Some(answer) = client
                        .request("keyboardInteractive", json!({"name": name, "instruction": instructions, "prompts": prompts}))
                        .await
                    else {
                        return Ok(None);
                    };
                    answer
                        .as_array()
                        .map(|items| {
                            items
                                .iter()
                                .map(|item| item.get("text").and_then(Value::as_str).unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default()
                };
                response = handle
                    .authenticate_keyboard_interactive_respond(answers)
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
    }
}

async fn authenticate(client: &Client, handle: &mut Handle<ClientHandler>, options: &ConnectOptions) -> Result<(), String> {
    let user = options.username.as_str();
    let mut methods = Vec::new();
    if auth_result(handle.authenticate_none(user).await.map_err(|error| error.to_string())?, &mut methods) {
        return Ok(());
    }
    client.log("info", format!("Authentication methods: {}", methods.join(", ")));
    client.debug(format!("debug1: Server offers authentication methods: {}", methods.join(", ")));

    for (index, key) in options.keys.iter().enumerate() {
        if !methods.iter().any(|method| method == "publickey") || key.private_key.trim().is_empty() {
            continue;
        }
        client.debug(format!("debug1: Offering configured public key #{}", index + 1));
        client.emit(json!({"kind": "authKeyState", "index": index, "state": "processing"}));
        match try_key(handle, user, key, &mut methods).await {
            Ok(true) => {
                client.emit(json!({"kind": "authKeyState", "index": index, "state": "succeeded"}));
                client.log("info", "Authenticated with public key");
                return Ok(());
            }
            Ok(false) => client.log("warning", "Public key was rejected by the server"),
            Err(error) => client.log("warning", error),
        }
        client.emit(json!({"kind": "authKeyState", "index": index, "state": "failed"}));
    }

    if let Some(password) = options.password.as_deref().filter(|value| !value.is_empty()) {
        if methods.iter().any(|method| method == "password") {
            client.debug("debug1: Attempting password authentication");
            let result = handle.authenticate_password(user, password).await.map_err(|error| error.to_string())?;
            if auth_result(result, &mut methods) {
                client.log("info", "Authenticated with password");
                return Ok(());
            }
            client.log("warning", "Password authentication failed");
        }
    }

    for _ in 0..MAX_AUTH_ROUNDS {
        if methods.is_empty() {
            break;
        }
        let Some(choice) = client.request("authMethods", json!({"methods": methods})).await else {
            return Err("Authentication was cancelled".into());
        };
        let method = choice.get("method").and_then(Value::as_str).unwrap_or("");
        client.debug(format!("debug1: Attempting {method} authentication"));
        let succeeded = match method {
            "password" => {
                let password = choice.get("password").and_then(Value::as_str).unwrap_or("");
                let result = handle.authenticate_password(user, password).await.map_err(|error| error.to_string())?;
                auth_result(result, &mut methods)
            }
            "publickey" => {
                let key = choice.get("key").cloned().unwrap_or(Value::Null);
                let key = KeyOption {
                    private_key: key.get("privateKey").and_then(Value::as_str).unwrap_or("").to_string(),
                    passphrase: key.get("passphrase").and_then(Value::as_str).map(str::to_string),
                };
                match try_key(handle, user, &key, &mut methods).await {
                    Ok(value) => value,
                    Err(error) => {
                        client.log("warning", error);
                        false
                    }
                }
            }
            "keyboard-interactive" => match keyboard_interactive(client, handle, user, &mut methods).await? {
                Some(value) => value,
                None => return Err("Authentication was cancelled".into()),
            },
            other => return Err(format!("Unsupported authentication method: {other}")),
        };
        if succeeded {
            client.log("info", format!("Authenticated with {method}"));
            return Ok(());
        }
        client.log("warning", format!("Authentication with {method} failed"));
    }
    Err("Authentication failed: no more authentication methods available".into())
}

async fn run_connect(client: Arc<Client>, options: ConnectOptions) -> Result<(), String> {
    let address = format!("{}:{}", options.host, options.port);
    client.log("info", format!("Connecting to {address}"));
    client.debug(format!("debug1: Connecting to host {} port {} using russh", options.host, options.port));
    let stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((options.host.as_str(), options.port)))
        .await
        .map_err(|_| format!("Connection to {address} timed out"))?
        .map_err(|error| {
            client.debug(format!("debug1: TCP connect failed: {error:?}"));
            let reason = if error.kind() == std::io::ErrorKind::ConnectionRefused {
                "Connection refused".to_string()
            } else {
                error.to_string()
            };
            format!("ssh: connect to host {} port {}: {reason}", options.host, options.port)
        })?;
    client.debug("debug1: TCP connection established");
    client.debug("debug1: Starting SSH protocol version exchange and key negotiation");
    let _ = stream.set_nodelay(true);
    let server_id = Arc::new(Mutex::new(Vec::new()));
    let sniffer = IdSniffer { inner: stream, id: server_id.clone(), done: false };

    let config = client::Config {
        keepalive_interval: (options.keepalive_interval > 0).then(|| Duration::from_secs(options.keepalive_interval)),
        keepalive_max: 3,
        ..Default::default()
    };
    let (alias_host, alias_port) = options
        .known_host_alias
        .as_ref()
        .map(|alias| (alias.host.clone(), alias.port))
        .unwrap_or_else(|| (options.host.clone(), options.port));
    let handler = ClientHandler {
        client: client.clone(),
        known_host_name: known_host_name(&alias_host, alias_port),
        known_hosts: options.known_hosts.clone(),
    };
    let mut handle = client::connect_stream(Arc::new(config), sniffer, handler)
        .await
        .map_err(|error| match error {
            russh::Error::UnknownKey => "The server host key was not accepted".to_string(),
            other => format!("SSH handshake failed: {other}"),
        })?;

    let software = server_id.lock().map(|id| String::from_utf8_lossy(&id).trim().to_string()).unwrap_or_default();
    let mut id_parts = software.splitn(3, '-');
    let protocol = format!("{}-{}", id_parts.next().unwrap_or("SSH"), id_parts.next().unwrap_or("2.0"));
    let software_version = id_parts.next().unwrap_or("").replace(' ', "  ");
    client.debug(format!("debug1: Remote protocol version {protocol}, remote software version {software_version}"));
    client.emit(json!({"kind": "sessionInfo", "softwareVersion": software_version, "protocolVersion": protocol}));

    client.debug(format!("debug1: Authenticating to {} as '{}'", options.host, options.username));
    authenticate(&client, &mut handle, &options).await?;
    client.debug("debug1: Authentication succeeded");
    if client.closed.load(Ordering::SeqCst) {
        return Ok(());
    }
    if let Ok(mut slot) = client.handle.lock() {
        *slot = Some(Arc::new(handle));
    }
    if !options.environment.is_empty() {
        client.log("debug", format!("{} environment variables will be sent", options.environment.len()));
    }
    client.emit(json!({"kind": "connected"}));
    Ok(())
}

fn get_client(state: &tauri::State<'_, SshClients>, id: &str) -> Result<Arc<Client>, String> {
    state
        .0
        .lock()
        .map_err(|_| "SSH client registry is unavailable".to_string())?
        .get(id)
        .cloned()
        .ok_or_else(|| "SSH client is closed".to_string())
}

#[tauri::command]
pub fn ssh_client_connect(
    state: tauri::State<'_, SshClients>,
    id: String,
    options: ConnectOptions,
    events: Channel<Value>,
) -> Result<(), String> {
    if options.host.trim().is_empty() || options.username.trim().is_empty() {
        return Err("SSH host and username are required".into());
    }
    let client = Arc::new(Client {
        events,
        pending: Mutex::new(HashMap::new()),
        next_request: AtomicU64::new(1),
        handle: Mutex::new(None),
        channels: Mutex::new(HashMap::new()),
        tasks: Mutex::new(Vec::new()),
        remote_forwards: Mutex::new(HashMap::new()),
        closed: AtomicBool::new(false),
        disconnected: AtomicBool::new(false),
        verbose: options.verbose,
    });
    state
        .0
        .lock()
        .map_err(|_| "SSH client registry is unavailable".to_string())?
        .insert(id, client.clone());
    let task_client = client.clone();
    client.spawn(async move {
        if let Err(message) = run_connect(task_client.clone(), options).await {
            task_client.emit(json!({"kind": "error", "message": message}));
        }
    });
    Ok(())
}

#[tauri::command]
pub fn ssh_client_reply(state: tauri::State<'_, SshClients>, id: String, request: u64, value: Value) -> Result<(), String> {
    let client = get_client(&state, &id)?;
    let sender = client.pending.lock().map_err(|_| "SSH client state is unavailable")?.remove(&request);
    if let Some(sender) = sender {
        let _ = sender.send(value);
    }
    Ok(())
}

#[tauri::command]
pub fn ssh_client_close(state: tauri::State<'_, SshClients>, id: String) -> Result<(), String> {
    let client = state.0.lock().map_err(|_| "SSH client registry is unavailable")?.remove(&id);
    if let Some(client) = client {
        client.shutdown();
    }
    Ok(())
}

// --------------------------------------------------------------- channels --

/// Pump channel messages to the renderer until the channel closes; writes and
/// resizes arrive through `commands`.
async fn pump_channel(
    client: Arc<Client>,
    channel_id: u32,
    channel: russh::Channel<Msg>,
    mut commands: mpsc::UnboundedReceiver<ChannelCmd>,
    collect: bool,
) {
    let (mut reader, writer): (_, ChannelWriteHalf<Msg>) = channel.split();
    let writer_task = tauri::async_runtime::spawn(async move {
        while let Some(command) = commands.recv().await {
            let result = match command {
                ChannelCmd::Data(data) => writer.data_bytes(data).await,
                ChannelCmd::Resize(cols, rows) => writer.window_change(cols, rows, 0, 0).await,
                ChannelCmd::Eof => writer.eof().await,
                ChannelCmd::Close => {
                    let _ = writer.close().await;
                    break;
                }
            };
            if result.is_err() {
                break;
            }
        }
    });
    let mut collected = Vec::new();
    let mut exit_status: Option<u32> = None;
    while let Some(message) = reader.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                if collect {
                    collected.extend_from_slice(&data);
                } else {
                    client.emit(json!({"kind": "data", "channel": channel_id, "data": STANDARD.encode(&data)}));
                }
            }
            ChannelMsg::ExitStatus { exit_status: status } => exit_status = Some(status),
            ChannelMsg::ExitSignal { signal_name, .. } => {
                client.log("info", format!("Remote process terminated by signal {signal_name:?}"));
            }
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    writer_task.abort();
    if let Ok(mut channels) = client.channels.lock() {
        channels.remove(&channel_id);
    }
    if collect {
        client.emit(json!({
            "kind": "execResult", "channel": channel_id,
            "exitCode": exit_status.unwrap_or(0), "data": STANDARD.encode(&collected),
        }));
    } else {
        client.emit(json!({"kind": "channelClosed", "channel": channel_id, "exitStatus": exit_status}));
    }
}

fn register_channel(client: &Arc<Client>, channel_id: u32) -> Result<mpsc::UnboundedReceiver<ChannelCmd>, String> {
    let (tx, rx) = mpsc::unbounded_channel();
    client.channels.lock().map_err(|_| "SSH client state is unavailable")?.insert(channel_id, tx);
    Ok(rx)
}

#[derive(Deserialize)]
pub struct PtyOptions {
    term: String,
    cols: u32,
    rows: u32,
}

#[tauri::command]
pub fn ssh_client_shell(
    state: tauri::State<'_, SshClients>,
    id: String,
    channel: u32,
    pty: PtyOptions,
    environment: Vec<(String, String)>,
) -> Result<(), String> {
    let client = get_client(&state, &id)?;
    let handle = client.handle()?;
    let commands = register_channel(&client, channel)?;
    let task_client = client.clone();
    client.spawn(async move {
        let opened = async {
            let session = handle.channel_open_session().await.map_err(|error| error.to_string())?;
            for (name, value) in &environment {
                let _ = session.set_env(false, name.as_str(), value.as_str()).await;
            }
            let term = if pty.term.is_empty() { "xterm-256color" } else { pty.term.as_str() };
            session
                .request_pty(true, term, pty.cols.max(1), pty.rows.max(1), 0, 0, &[])
                .await
                .map_err(|error| error.to_string())?;
            session.request_shell(true).await.map_err(|error| error.to_string())?;
            Ok::<_, String>(session)
        }
        .await;
        match opened {
            Ok(session) => {
                task_client.emit(json!({"kind": "channelReady", "channel": channel}));
                pump_channel(task_client, channel, session, commands, false).await;
            }
            Err(message) => {
                task_client.emit(json!({"kind": "channelError", "channel": channel, "message": message}));
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn ssh_client_exec(state: tauri::State<'_, SshClients>, id: String, channel: u32, command: String) -> Result<(), String> {
    let client = get_client(&state, &id)?;
    let handle = client.handle()?;
    let commands = register_channel(&client, channel)?;
    let task_client = client.clone();
    client.spawn(async move {
        let opened = async {
            let session = handle.channel_open_session().await.map_err(|error| error.to_string())?;
            session.exec(true, STANDARD.decode(&command).unwrap_or_default()).await.map_err(|error| error.to_string())?;
            Ok::<_, String>(session)
        }
        .await;
        match opened {
            Ok(session) => pump_channel(task_client, channel, session, commands, true).await,
            Err(message) => task_client.emit(json!({"kind": "channelError", "channel": channel, "message": message})),
        }
    });
    Ok(())
}

fn channel_sender(state: &tauri::State<'_, SshClients>, id: &str, channel: u32) -> Result<mpsc::UnboundedSender<ChannelCmd>, String> {
    get_client(state, id)?
        .channels
        .lock()
        .map_err(|_| "SSH client state is unavailable".to_string())?
        .get(&channel)
        .cloned()
        .ok_or_else(|| "SSH channel is closed".to_string())
}

#[tauri::command]
pub fn ssh_client_write(state: tauri::State<'_, SshClients>, id: String, channel: u32, data: String) -> Result<(), String> {
    let bytes = STANDARD.decode(data).map_err(|_| "Invalid terminal input encoding")?;
    channel_sender(&state, &id, channel)?
        .send(ChannelCmd::Data(bytes))
        .map_err(|_| "SSH channel is closed".to_string())
}

#[tauri::command]
pub fn ssh_client_resize(state: tauri::State<'_, SshClients>, id: String, channel: u32, cols: u32, rows: u32) -> Result<(), String> {
    channel_sender(&state, &id, channel)?
        .send(ChannelCmd::Resize(cols.clamp(1, 1000), rows.clamp(1, 1000)))
        .map_err(|_| "SSH channel is closed".to_string())
}

#[tauri::command]
pub fn ssh_client_close_channel(state: tauri::State<'_, SshClients>, id: String, channel: u32) -> Result<(), String> {
    if let Ok(sender) = channel_sender(&state, &id, channel) {
        let _ = sender.send(ChannelCmd::Eof);
        let _ = sender.send(ChannelCmd::Close);
    }
    Ok(())
}

pub(crate) fn client_handle(state: &tauri::State<'_, SshClients>, id: &str) -> Result<(Arc<Client>, Arc<Handle<ClientHandler>>), String> {
    let client = get_client(state, id)?;
    let handle = client.handle()?;
    Ok((client, handle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_names_follow_openssh_conventions() {
        assert_eq!(known_host_name("example.com", 22), "example.com");
        assert_eq!(known_host_name("10.0.0.1", 2001), "[10.0.0.1]:2001");
    }

    #[test]
    fn known_hosts_detect_trusted_mismatched_and_unknown_keys() {
        let key = PublicKey::from_openssh("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl")
            .expect("valid key");
        let other = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIB7oc+WrKa3J1ZB9sdIo5ITNq8QbSIjDaVbYz+o5QRVh";
        let entry = format!("[h]:2001 {}", key.to_openssh().expect("encode"));
        assert!(matches!(check_known_hosts(&[entry.clone()], "[h]:2001", &key), KnownHostStatus::Trusted));
        assert!(matches!(check_known_hosts(&[format!("[h]:2001 {other}")], "[h]:2001", &key), KnownHostStatus::Mismatch));
        assert!(matches!(check_known_hosts(&[entry], "h", &key), KnownHostStatus::NotFound));
    }
}
