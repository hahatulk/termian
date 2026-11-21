//! Byte streams behind terminal tabs: a local shell (PTY) or an SSH shell.
//! SSH login questions (unknown host key, password, keyboard-interactive)
//! arrive as `SessionEvent::Login` and are answered with `Session::answer`,
//! so the UI can show them on the connection screen.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::future::BoxFuture;
use futures::{SinkExt, StreamExt};
use futures::channel::mpsc::{Receiver, Sender};
use russh::client::{self, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{
    load_secret_key, HashAlg, PrivateKeyWithHashAlg, PublicKey,
    PublicKeyOrCertificate,
};
use russh::ChannelMsg;
use tokio::sync::Mutex;

use crate::login::{Answer, HostKeyDecision, HostKeyInfo, LoginEvent, Stage};

#[derive(Debug, PartialEq)]
pub enum SessionEvent {
    Output(Vec<u8>),
    /// SSH login progress and questions; the shell output follows `Stage::Ready`.
    Login(LoginEvent),
    /// The session ended; `Some` carries the reason when it failed.
    Closed(Option<String>),
    /// «Detect OS» found the host's system (`debian`, `ubuntu`, ...).
    Os(String),
    /// Existing shell history, oldest first; never terminal output.
    History(Vec<String>),
}

enum Input {
    Data(Vec<u8>),
    Answer(Answer),
    Resize(u16, u16),
    Close,
}

/// A running session. Dropping it closes the connection.
pub struct Session {
    input: UnboundedSender<Input>,
    events: Option<Receiver<SessionEvent>>,
}

impl Session {
    pub fn write(&self, bytes: &[u8]) {
        let _ = self.input.unbounded_send(Input::Data(bytes.to_vec()));
    }

    /// Answers the pending login question.
    pub fn answer(&self, answer: Answer) {
        let _ = self.input.unbounded_send(Input::Answer(answer));
    }

    /// Session output; can be taken once.
    pub fn take_events(&mut self) -> Option<Receiver<SessionEvent>> {
        self.events.take()
    }

    pub fn resize(&self, columns: u16, rows: u16) {
        let _ = self.input.unbounded_send(Input::Resize(columns.max(1), rows.max(1)));
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.input.unbounded_send(Input::Close);
    }
}

#[derive(Clone)]
struct EventTx(Sender<SessionEvent>);

impl EventTx {
    fn send(&self, event: SessionEvent) -> Result<(), futures::channel::mpsc::SendError> {
        futures::executor::block_on(self.0.clone().send(event))
    }
}

fn channels() -> (UnboundedSender<Input>, UnboundedReceiver<Input>, EventTx, Session) {
    let (input, input_rx) = unbounded();
    let (events_tx, events) = futures::channel::mpsc::channel(32);
    (input.clone(), input_rx, EventTx(events_tx), Session { input, events: Some(events) })
}

/// A finished session's output, replayed read-only (Logs).
pub fn replay(data: Vec<u8>) -> Session {
    let (_, _, events, session) = channels();
    let _ = events.send(SessionEvent::Output(data));
    session
}

// ------------------------------------------------------------------ local --

/// Starts the user's login shell, or `shell` («Local Terminal Path»), with
/// `term` as `TERM`.
pub fn local_shell(shell: Option<&str>, term: &str, columns: u16, rows: u16, import_history: bool) -> Result<Session, String> {
    let mut command = match shell.filter(|s| !s.trim().is_empty()) {
        Some(shell) => {
            let mut command = portable_pty::CommandBuilder::new(shell.trim());
            if !cfg!(windows) {
                command.arg("-l"); // a login shell, as the default program is
            }
            if let Some(home) = dirs::home_dir() {
                command.cwd(home);
            }
            command
        }
        None => portable_pty::CommandBuilder::new_default_prog(),
    };
    command.env("TERM", term);
    #[cfg(unix)]
    let history_shell = import_history.then(|| shell.filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_owned()).unwrap_or_else(|| command.get_shell()));
    #[cfg(not(unix))]
    let history_shell = { let _ = import_history; None };
    local_command_with_history(command, columns, rows, history_shell)
}

pub fn local_command(command: portable_pty::CommandBuilder, columns: u16, rows: u16) -> Result<Session, String> {
    local_command_with_history(command, columns, rows, None)
}

fn local_command_with_history(command: portable_pty::CommandBuilder, columns: u16, rows: u16, history_shell: Option<String>) -> Result<Session, String> {
    use portable_pty::{native_pty_system, PtySize};
    let size = |cols: u16, rows: u16| PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
    let pair = native_pty_system().openpty(size(columns, rows)).map_err(|e| e.to_string())?;
    let mut command = command;
    if command.get_env("TERM").is_none() {
        command.env("TERM", "xterm-256color");
    }
    let mut child = pair.slave.spawn_command(command).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let (_, input_rx, events_tx, session) = channels();
    if let Some(path) = history_shell {
        let events = events_tx.clone();
        std::thread::spawn(move || {
            if let Some(commands) = crate::shell_history::local(&path) {
                let _ = events.send(SessionEvent::History(commands));
            }
        });
    }

    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if events_tx.send(SessionEvent::Output(buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
        let status = child.wait().ok();
        let reason = status.filter(|s| !s.success()).map(|s| format!("Process exited with code {}", s.exit_code()));
        let _ = events_tx.send(SessionEvent::Closed(reason));
    });

    let master = pair.master;
    std::thread::spawn(move || {
        for input in futures::executor::block_on_stream(input_rx) {
            match input {
                Input::Data(bytes) => {
                    if writer.write_all(&bytes).and_then(|_| writer.flush()).is_err() {
                        break;
                    }
                }
                Input::Resize(cols, rows) => {
                    let _ = master.resize(size(cols, rows));
                }
                Input::Answer(_) => {}
                Input::Close => break,
            }
        }
        // Dropping the master hangs up the shell.
    });
    Ok(session)
}

// ----------------------------------------------------------------- serial --

/// Translates a serial line in a legacy charset: device bytes to UTF-8 for
/// the terminal (keeping a split character for the next read), keys back.
pub struct Charset {
    encoding: &'static encoding_rs::Encoding,
    decoder: encoding_rs::Decoder,
}

impl Charset {
    pub fn new(encoding: &'static encoding_rs::Encoding) -> Self {
        Self { encoding, decoder: encoding.new_decoder_without_bom_handling() }
    }

    pub fn decode(&mut self, bytes: &[u8]) -> Vec<u8> {
        if self.encoding == encoding_rs::UTF_8 {
            return bytes.to_vec();
        }
        let mut text = String::with_capacity(bytes.len() * 3);
        let _ = self.decoder.decode_to_string(bytes, &mut text, false);
        text.into_bytes()
    }

    pub fn encode(&self, bytes: &[u8]) -> Vec<u8> {
        if self.encoding == encoding_rs::UTF_8 {
            return bytes.to_vec();
        }
        self.encoding.encode(&String::from_utf8_lossy(bytes)).0.into_owned()
    }
}

/// Opens a listed serial port with the legacy settings (raw mode, 140 ms
/// reads, 2 s writes). Only ports the system lists can be opened.
pub fn serial(options: &crate::serial::SerialOptions) -> Result<Session, String> {
    use crate::serial::{FlowControl, Parity};
    options.validate()?;
    if !crate::serial::list_ports().contains(&options.path) {
        return Err("Serial port is not connected or was not found by the operating system".into());
    }
    let invalid = |e: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, e);
    let mut port = serial2::SerialPort::open(&options.path, |mut settings: serial2::Settings| {
        settings.set_raw();
        settings.set_baud_rate(options.baud_rate)?;
        settings.set_char_size(serial2::CharSize::try_from(options.data_bits).map_err(|e| invalid(e.to_string()))?);
        settings.set_stop_bits(serial2::StopBits::try_from(options.stop_bits).map_err(|e| invalid(e.to_string()))?);
        settings.set_parity(match options.parity {
            Parity::None => serial2::Parity::None,
            Parity::Odd => serial2::Parity::Odd,
            Parity::Even => serial2::Parity::Even,
        });
        settings.set_flow_control(match options.flow_control {
            FlowControl::RtsCts => serial2::FlowControl::RtsCts,
            FlowControl::XonXoff => serial2::FlowControl::XonXoff,
            FlowControl::None | FlowControl::DsrDtr => serial2::FlowControl::None,
        });
        Ok(settings)
    })
    .map_err(|e| format!("Unable to open serial port {}: {e}", options.path))?;
    port.set_read_timeout(std::time::Duration::from_millis(140)).map_err(|e| e.to_string())?;
    port.set_write_timeout(std::time::Duration::from_secs(2)).map_err(|e| e.to_string())?;
    let encoding = crate::serial::encoding(&options.charset).ok_or("unsupported charset")?;
    let port = std::sync::Arc::new(port);
    let (_, input_rx, events_tx, session) = channels();
    let closed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let (reader, stop) = (port.clone(), closed.clone());
    std::thread::spawn(move || {
        let mut charset = Charset::new(encoding);
        let mut buffer = [0u8; 4096];
        let reason = loop {
            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                break None;
            }
            match reader.read(&mut buffer) {
                Ok(0) => continue,
                Ok(n) => {
                    if events_tx.send(SessionEvent::Output(charset.decode(&buffer[..n]))).is_err() {
                        break None;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => break Some(format!("Connection closed with error: {e}")),
            }
        };
        let _ = events_tx.send(SessionEvent::Closed(reason));
    });
    std::thread::spawn(move || {
        let charset = Charset::new(encoding);
        for input in futures::executor::block_on_stream(input_rx) {
            match input {
                Input::Data(bytes) => {
                    if port.write_all(&charset.encode(&bytes)).is_err() {
                        break;
                    }
                }
                Input::Close => break,
                Input::Resize(..) | Input::Answer(_) => {}
            }
        }
        closed.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    Ok(session)
}

// -------------------------------------------------------------------- ssh --

#[derive(Clone, Debug)]
pub struct SshTarget {
    pub host: String,
    pub port: u16,
    /// Asked for in the terminal when missing.
    pub username: Option<String>,
    /// Trusted host keys; new keys are added after the user confirms.
    pub known_hosts: PathBuf,
    /// Private key files to offer, in order; missing files are skipped.
    pub identity_files: Vec<PathBuf>,
    /// Saved password, tried once before asking.
    pub password: Option<String>,
    /// Private key from the Keychain (and its passphrase), offered first.
    pub key: Option<LoginKey>,
    pub options: SshOptions,
}

/// A Keychain key as SSH login offers it: with its certificate, when it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginKey {
    pub private_key: String,
    pub passphrase: Option<String>,
    pub certificate: Option<String>,
}

impl LoginKey {
    pub fn new(private_key: impl Into<String>, passphrase: Option<String>) -> Self {
        Self { private_key: private_key.into(), passphrase, certificate: None }
    }

    /// `None` for a key without its private part.
    pub fn from_vault(key: &crate::vault::SshKey) -> Option<Self> {
        Some(Self { certificate: key.certificate.clone(), ..Self::new(key.private_key.clone()?, key.passphrase.clone()) })
    }
}

/// Settings → Terminal values a connection uses.
#[derive(Clone, Debug, PartialEq)]
pub struct SshOptions {
    /// «Terminal emulation type», sent with the PTY request.
    pub term: String,
    /// «Keepalive Interval» in seconds; 0 sends none, as in legacy.
    pub keepalive: u32,
    /// «Post-Quantum Key Exchange»: offer `mlkem768x25519-sha256` first.
    pub pqc_kex: bool,
    /// «Detect OS»: find the system of a saved host that has none yet.
    pub detect_os: bool,
    /// «Import shell history»: read existing bash/zsh/fish history after login.
    pub import_history: bool,
    /// «Verbose (-v)» on the failed connection screen: `debug1:` lines in the log.
    pub verbose: bool,
    /// The host's «Environment Variable» list, sent before the shell starts.
    pub env: Vec<(String, String)>,
    /// The host's «Agent Forwarding»: the server may use the local SSH agent.
    pub agent_forwarding: bool,
    /// The host's charset; `None` is UTF-8.
    pub charset: Option<String>,
}

impl Default for SshOptions {
    fn default() -> Self {
        Self { term: "xterm-256color".into(), keepalive: 0, pqc_kex: true, detect_os: false, import_history: false, verbose: false, env: Vec::new(), agent_forwarding: false, charset: None }
    }
}

impl SshOptions {
    pub fn from_vault(vault: &crate::vault::Vault) -> Self {
        let terminal = &vault.terminal;
        Self { term: terminal.emulation().to_owned(), keepalive: crate::terminal_settings::keep_alive(&vault.foreign), pqc_kex: terminal.pqc_kex, detect_os: false, import_history: terminal.shell_history_import, verbose: false, ..Self::default() }
    }

    /// Key exchanges to offer: russh's order, without the hybrid
    /// post-quantum one when it is turned off.
    pub fn kex(&self) -> Vec<russh::kex::Name> {
        russh::Preferred::default().kex.iter().copied().filter(|name| self.pqc_kex || *name != russh::kex::MLKEM768X25519_SHA256).collect()
    }

    fn config(&self) -> client::Config {
        let keepalive = (self.keepalive > 0).then(|| Duration::from_secs(self.keepalive as u64));
        client::Config {
            keepalive_interval: keepalive,
            keepalive_max: 3,
            preferred: russh::Preferred { kex: self.kex().into(), ..russh::Preferred::default() },
            ..Default::default()
        }
    }
}

impl SshTarget {
    /// Port 22, the user's default OpenSSH keys, and the native client's known_hosts.
    pub fn new(host: &str, username: Option<String>) -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let bracketed = host.strip_prefix('[').and_then(|s| s.split_once(']')).and_then(|(ip, suffix)| {
            ip.parse::<std::net::Ipv6Addr>().ok()?;
            let port = if suffix.is_empty() { 22 } else { suffix.strip_prefix(':')?.parse().ok()? };
            Some((ip.to_owned(), port))
        });
        let (host, port) = bracketed.unwrap_or_else(|| match host.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') && p.parse::<u16>().is_ok() => (h.to_owned(), p.parse().unwrap()),
            _ => (host.to_owned(), 22),
        });
        Self {
            host,
            port,
            username,
            known_hosts: crate::known_hosts::default_path(),
            identity_files: ["id_ed25519", "id_ecdsa", "id_rsa"].iter().map(|f| home.join(".ssh").join(f)).collect(),
            password: None,
            key: None,
            options: SshOptions::default(),
        }
    }
}

impl SshTarget {
    /// A saved host: its address and port, user and saved password.
    /// A saved host with the credentials it logs in with (its identity's, if set).
    pub fn for_host(host: &crate::vault::Host, vault: &crate::vault::Vault) -> Self {
        let (username, password, key) = vault.credentials(host);
        let mut target = Self {
            password,
            key: key.and_then(LoginKey::from_vault),
            ..Self::new(&host.address, username)
        };
        if let Some(port) = host.port { target.port = port; }
        let settings = &host.settings;
        target.options.env = settings.env.iter().map(|v| (v.name.clone(), v.value.clone())).collect();
        target.options.agent_forwarding = settings.agent_forwarding;
        target.options.charset = settings.charset.clone();
        target
    }
}

/// Parses what a user types into the quick-connect field:
/// `host`, `user@host`, `host:port`, optionally prefixed with `ssh `.
pub fn parse_destination(input: &str) -> Option<SshTarget> {
    let input = input.trim();
    let input = input.strip_prefix("ssh ").map(str::trim).unwrap_or(input);
    if input.is_empty() || input.contains(char::is_whitespace) {
        return None;
    }
    let (user, host) = match input.rsplit_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() => (Some(user.to_owned()), host),
        Some(_) => return None,
        None => (None, input),
    };
    Some(SshTarget::new(host, user))
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const PASSWORD_ATTEMPTS: usize = 3;

pub fn ssh(target: SshTarget, columns: u16, rows: u16) -> Session {
    let (_, input_rx, events_tx, session) = channels();
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ = events_tx.send(SessionEvent::Closed(Some(error.to_string())));
                return;
            }
        };
        let result = runtime.block_on(run_ssh(target, input_rx, events_tx.clone(), (columns, rows)));
        let _ = events_tx.send(SessionEvent::Closed(result.err()));
    });
    session
}

/// Where login questions go; the UI answers them on the connection screen.
pub(crate) trait Prompter: Send {
    fn event(&mut self, event: LoginEvent);
    fn ask<'a>(&'a mut self, prompt: &'a str, echo: bool) -> BoxFuture<'a, Option<String>>;
    fn host_key(&mut self, info: HostKeyInfo) -> BoxFuture<'_, HostKeyDecision>;

    /// Credentials of an identity picked on the Username question, once.
    fn take_identity(&mut self) -> Option<(Option<String>, Option<LoginKey>)> {
        None
    }

    fn log(&mut self, line: String) {
        self.event(LoginEvent::Log(line));
    }
}

pub(crate) type SharedPrompter = Arc<Mutex<dyn Prompter>>;

/// Forwards login events through `emit` and waits for answers on `answers`.
pub(crate) struct EventPrompter<F: FnMut(LoginEvent) + Send> {
    pub emit: F,
    pub answers: UnboundedReceiver<Answer>,
    /// Password and key of an identity picked for the username.
    pub picked: Option<(Option<String>, Option<LoginKey>)>,
}

impl<F: FnMut(LoginEvent) + Send> Prompter for EventPrompter<F> {
    fn event(&mut self, event: LoginEvent) {
        (self.emit)(event);
    }

    fn ask<'a>(&'a mut self, prompt: &'a str, echo: bool) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            (self.emit)(LoginEvent::Prompt { text: prompt.trim().to_owned(), echo });
            match self.answers.next().await {
                Some(Answer::Text(text)) => text,
                Some(Answer::Identity { username, password, key }) => {
                    self.picked = Some((password, key));
                    Some(username)
                }
                _ => None,
            }
        })
    }

    fn take_identity(&mut self) -> Option<(Option<String>, Option<LoginKey>)> {
        self.picked.take()
    }

    fn host_key(&mut self, info: HostKeyInfo) -> BoxFuture<'_, HostKeyDecision> {
        Box::pin(async move {
            (self.emit)(LoginEvent::HostKey(info));
            match self.answers.next().await {
                Some(Answer::HostKey(decision)) => decision,
                _ => HostKeyDecision::Reject,
            }
        })
    }
}

pub(crate) struct Verifier {
    host: String,
    port: u16,
    known_hosts: PathBuf,
    io: SharedPrompter,
    /// Remote port forwarding: where connections the server sends back go.
    pub(crate) forward_to: Option<(String, u16)>,
    /// The server's version line, recorded once a channel opens.
    server_id: Arc<std::sync::Mutex<Option<String>>>,
    /// «Agent Forwarding»: agent channels the server opens go to `SSH_AUTH_SOCK`.
    #[cfg(unix)]
    agent: bool,
}

fn key_label(key: &PublicKey) -> String {
    crate::vault::key_kind(key.algorithm().as_str())
}

impl client::Handler for Verifier {
    type Error = russh::Error;

    async fn channel_open_confirmation(&mut self, _: russh::ChannelId, _: u32, _: u32, session: &mut client::Session) -> Result<(), Self::Error> {
        let mut id = self.server_id.lock().unwrap();
        if id.is_none() {
            *id = Some(String::from_utf8_lossy(session.remote_sshid()).into_owned());
        }
        Ok(())
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        _: &str,
        _: u32,
        _: &str,
        _: u32,
        reply: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let Some((host, port)) = self.forward_to.clone() else {
            reply.reject(russh::ChannelOpenFailure::AdministrativelyProhibited).await;
            return Ok(());
        };
        reply.accept().await;
        tokio::spawn(async move {
            if let Ok(mut local) = tokio::net::TcpStream::connect((host.as_str(), port)).await {
                let mut remote = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
            }
        });
        Ok(())
    }

    async fn server_channel_open_agent_forward(&mut self, channel: russh::Channel<client::Msg>, reply: client::ChannelOpenHandle, _: &mut client::Session) -> Result<(), Self::Error> {
        #[cfg(unix)]
        if let Some(socket) = std::env::var_os("SSH_AUTH_SOCK").filter(|_| self.agent) {
            reply.accept().await;
            tokio::spawn(async move {
                if let Ok(mut local) = tokio::net::UnixStream::connect(socket).await {
                    let mut remote = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
                }
            });
            return Ok(());
        }
        // Without a local agent (or on Windows, where it is a named pipe) the request is refused.
        #[cfg(not(unix))]
        let _ = channel;
        reply.reject(russh::ChannelOpenFailure::AdministrativelyProhibited).await;
        Ok(())
    }

    async fn check_server_key(&mut self, server_key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let key: PublicKey = match server_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(certificate) => certificate.public_key().clone().into(),
        };
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        let info = HostKeyInfo {
            host: self.host.clone(),
            port: self.port,
            algorithm: key_label(&key),
            fingerprint: fingerprint.trim_start_matches("SHA256:").to_owned(),
        };
        let mut io = self.io.lock().await;
        io.event(LoginEvent::Stage(Stage::Verifying));
        io.log(format!("Server host key: {} {fingerprint}", info.algorithm));
        match crate::known_hosts::check(&self.host, self.port, &key, &self.known_hosts) {
            Ok(crate::known_hosts::Trust::Match) => {
                io.log("Host key matches a known_hosts entry".into());
                return Ok(true);
            }
            Ok(crate::known_hosts::Trust::Changed(line)) => {
                io.log(format!("Host key differs from known_hosts line {line}"));
                io.event(LoginEvent::HostKeyChanged(info));
                return Ok(false);
            }
            Ok(crate::known_hosts::Trust::Unknown) => {}
            Err(error) => {
                io.log(format!("Cannot read known_hosts: {error}"));
                return Err(std::io::Error::new(error.kind(), format!("Cannot read known_hosts: {error}")).into());
            }
        }
        match io.host_key(info).await {
            HostKeyDecision::Reject => Ok(false),
            HostKeyDecision::Once => {
                io.log("Host key accepted for this session".into());
                Ok(true)
            }
            HostKeyDecision::Save => {
                match crate::known_hosts::learn(&self.host, self.port, &key, &self.known_hosts) {
                    Ok(()) => io.log(format!("Permanently added '{}' to the list of known hosts", self.host)),
                    Err(error) => return Err(error.into()),
                }
                Ok(true)
            }
        }
    }
}

async fn run_ssh(
    target: SshTarget,
    mut input: UnboundedReceiver<Input>,
    events: EventTx,
    size: (u16, u16),
) -> Result<(), String> {
    // Answers go to the login; everything else waits for the shell.
    let (answer_tx, answers) = unbounded();
    let (shell_tx, shell_input) = unbounded();
    let latest_size = Arc::new(std::sync::Mutex::new(size));
    let router_size = latest_size.clone();
    tokio::spawn(async move {
        while let Some(item) = input.next().await {
            let sent = match item {
                Input::Answer(answer) => answer_tx.unbounded_send(answer).is_ok(),
                Input::Resize(cols, rows) => {
                    *router_size.lock().unwrap() = (cols, rows);
                    shell_tx.unbounded_send(Input::Resize(cols, rows)).is_ok()
                }
                Input::Close => {
                    let _ = shell_tx.unbounded_send(Input::Close);
                    break;
                }
                other => shell_tx.unbounded_send(other).is_ok(),
            };
            if !sent {
                break;
            }
        }
    });

    let emit_events = events.clone();
    let prompter = Arc::new(Mutex::new(EventPrompter {
        emit: move |event| {
            let _ = emit_events.send(SessionEvent::Login(event));
        },
        answers,
        picked: None,
    }));
    let server_id = Arc::<std::sync::Mutex<Option<String>>>::default();
    let handle = Arc::new(open(&target, prompter.clone(), None, server_id.clone()).await?);
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    let (cols, rows) = *latest_size.lock().unwrap();
    // Servers may refuse variables they do not accept (AcceptEnv); that is not an error.
    for (name, value) in &target.options.env {
        let _ = channel.set_env(false, name.as_str(), value.as_str()).await;
    }
    if target.options.agent_forwarding {
        let _ = channel.agent_forward(false).await;
    }
    channel.request_pty(true, &target.options.term, cols as u32, rows as u32, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    channel.request_shell(true).await.map_err(|e| e.to_string())?;
    prompter.lock().await.event(LoginEvent::Stage(Stage::Ready));
    if target.options.detect_os {
        let (handle, events) = (handle.clone(), events.clone());
        tokio::spawn(async move {
            let version = server_id.lock().unwrap().clone().unwrap_or_default();
            if let Some(os) = detect_os(&handle, &version).await {
                let _ = events.send(SessionEvent::Os(os.to_owned()));
            }
        });
    }
    let history = target.options.import_history.then(|| {
        let (handle, events) = (handle.clone(), events.clone());
        tokio::spawn(async move {
            if let Some(commands) = read_shell_history(&handle).await {
                let _ = events.send(SessionEvent::History(commands));
            }
        })
    });
    let charset = target.options.charset.as_deref().and_then(crate::serial::encoding).map(Charset::new);
    let result = pump(channel, shell_input, events, charset).await;
    if let Some(history) = history { history.abort(); }
    drop(handle);
    result
}

/// Legacy `osTypesMap`, in its order: the first name found wins.
const OS_TYPES: [(&str, &str); 28] = [
    ("raspbian", "raspbian"),
    ("centos", "centos"),
    ("darwin", "osx"),
    ("ubuntu", "ubuntu"),
    ("openbsd", "openbsd"),
    ("netbsd", "netbsd"),
    ("freebsd", "freebsd"),
    ("suse", "suse"),
    ("fedora", "fedora"),
    ("mageia", "mageia"),
    ("arch", "arch"),
    ("gentoo", "gentoo"),
    ("debian", "debian"),
    ("linux", "linux"),
    ("android", "android"),
    ("mikrotik", "routeros"),
    ("routeros", "routeros"),
    ("redhat", "redhat"),
    ("red", "redhat"),
    ("msys_nt", "windows"),
    ("win", "windows"),
    ("windows", "windows"),
    ("cisco", "cisco"),
    ("gloria-ops-agent", "gloria_ops_agent"),
    ("rocky", "rocky"),
    ("almalinux", "alma"),
    ("alpine", "alpine"),
    ("amazon", "amazon"),
];

const DETECT_OS_SCRIPT: &str = "HISTFILE=; SA_OS_TYPE=\"Linux\" REAL_OS_NAME=`uname` if [ \"$REAL_OS_NAME\" != \"$SA_OS_TYPE\" ] ; then echo `uname` else DISTRIB_ID=\\\"`cat /etc/*release`\\\" echo $DISTRIB_ID; fi; exit; ";
const DETECT_OS_FISH_SCRIPT: &str = "if set name (uname) = \"Linux\"     cat /etc/*release else     uname end ";
const DETECT_ROUTEROS_SCRIPT: &str = ":put [/system resource get platform] ";

/// The system named in the server's version line (`getOsFromSoftwareVersion`).
pub fn os_from_version(version: &str) -> Option<&'static str> {
    let version = version.to_lowercase();
    OS_TYPES.iter().find(|(name, _)| version.contains(name)).map(|(_, os)| *os)
}

/// The system in the detection script's output (`parseOS`): a bare name such
/// as `Darwin`, or `NAME=` of `/etc/os-release`.
pub fn parse_os(output: &str) -> Option<&'static str> {
    let output = output.trim();
    let exact = |name: &str| OS_TYPES.iter().find(|(n, _)| *n == name.to_lowercase()).map(|(_, os)| *os);
    if let Some(os) = exact(output) {
        return Some(os);
    }
    for word in output.replace("Linux\n", "").split_whitespace() {
        let word = word.replace('"', "");
        if let Some(("NAME", name)) = word.split_once('=') {
            if let Some(os) = exact(name) {
                return Some(os);
            }
        }
    }
    output.to_lowercase().contains("msys_nt").then_some("windows")
}

/// The detection script for a login shell (`getDetectOSScriptByShellType`);
/// a shell that cannot `echo` is taken for RouterOS.
fn detect_script(shell: &str) -> &'static str {
    let shell = shell.trim();
    if shell.is_empty() || !shell.contains('/') {
        DETECT_ROUTEROS_SCRIPT
    } else if shell.contains("fish") {
        DETECT_OS_FISH_SCRIPT
    } else {
        DETECT_OS_SCRIPT
    }
}

/// Runs `command` on its own channel. Failed, oversized or timed-out replies
/// are discarded, not forwarded into the terminal or session log.
async fn exec(handle: &Handle<Verifier>, command: &str) -> Option<String> {
    let deadline = tokio::time::Instant::now() + crate::shell_history::TIMEOUT;
    let mut channel = tokio::time::timeout_at(deadline, handle.channel_open_session()).await.ok()?.ok()?;
    let result = tokio::time::timeout_at(deadline, async {
        channel.exec(true, command).await.ok()?;
        let mut output = Vec::new();
        let mut success = false;
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => {
                    if output.len() + data.len() > crate::shell_history::MAX_OUTPUT { return None; }
                    output.extend_from_slice(&data);
                }
                ChannelMsg::ExitStatus { exit_status } => success = exit_status == 0,
                ChannelMsg::ExitSignal { .. } => return None,
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        success.then(|| String::from_utf8(output).ok()).flatten()
    }).await.ok().flatten();
    let _ = tokio::time::timeout(Duration::from_secs(1), channel.close()).await;
    result
}

async fn read_shell_history(handle: &Handle<Verifier>) -> Option<Vec<String>> {
    let path = exec(handle, "echo $SHELL").await?;
    let shell = crate::shell_history::Shell::from_path(&path)?;
    let command = shell.remote_command(&path)?;
    Some(shell.parse(&exec(handle, &command).await?))
}

/// Legacy `getOS`: the version line first, then the script for the login shell.
async fn detect_os(handle: &Handle<Verifier>, version: &str) -> Option<&'static str> {
    if let Some(os) = os_from_version(version) {
        return Some(os);
    }
    let shell = exec(handle, "echo $SHELL").await.unwrap_or_default();
    parse_os(&exec(handle, detect_script(&shell)).await?)
}

/// Opens an authenticated SSH connection, asking `io` whatever login needs.
pub(crate) async fn connect(target: &SshTarget, io: SharedPrompter) -> Result<Handle<Verifier>, String> {
    connect_forwarding(target, io, None).await
}

/// [`connect`] that also accepts connections the server forwards back (remote forwarding).
pub(crate) async fn connect_forwarding(target: &SshTarget, io: SharedPrompter, forward_to: Option<(String, u16)>) -> Result<Handle<Verifier>, String> {
    open(target, io, forward_to, Arc::default()).await
}

#[cfg(target_os = "macos")]
async fn connect_macos_tcp<T, F>(
    addresses: impl IntoIterator<Item = std::net::SocketAddr>,
    mut dial: impl FnMut(std::net::SocketAddr) -> F,
    waiting: impl std::future::Future<Output = ()>,
) -> std::io::Result<T>
where
    F: std::future::Future<Output = std::io::Result<T>>,
{
    use std::net::IpAddr;

    let mut waiting = Some(waiting);
    let mut addresses = addresses.into_iter().collect::<Vec<_>>();
    let mut last = std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "could not resolve to any addresses",
    );
    loop {
        let mut retry = Vec::new();
        for endpoint in addresses {
            match dial(endpoint).await {
                Ok(stream) => return Ok(stream),
                Err(error) => {
                    if error.raw_os_error() == Some(libc::EHOSTUNREACH)
                        && match endpoint.ip().to_canonical() {
                            IpAddr::V4(ip) => ip.is_private() || ip.is_link_local(),
                            IpAddr::V6(ip) => ip.is_unique_local() || ip.is_unicast_link_local(),
                        }
                    {
                        retry.push(endpoint);
                    }
                    last = error;
                }
            }
        }
        if retry.is_empty() {
            return Err(last);
        }
        if let Some(notify) = waiting.take() {
            notify.await;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        addresses = retry;
    }
}

async fn open(
    target: &SshTarget,
    io: SharedPrompter,
    forward_to: Option<(String, u16)>,
    server_id: Arc<std::sync::Mutex<Option<String>>>,
) -> Result<Handle<Verifier>, String> {
    let address = format!("{}:{}", target.host, target.port);
    {
        let mut io = io.lock().await;
        io.event(LoginEvent::Stage(Stage::Connecting));
        io.log(format!("Connecting to {address}"));
        if target.options.verbose {
            io.log(format!("debug1: Connecting to host {} port {} using russh", target.host, target.port));
            io.log(format!("debug1: Post-quantum key exchange {}", if target.options.pqc_kex { "offered first" } else { "off" }));
            io.log(format!("debug1: Identity files: {}", target.identity_files.len() + usize::from(target.key.is_some())));
        }
    }
    #[cfg(target_os = "macos")]
    let connect = async {
        let addresses = tokio::net::lookup_host((target.host.as_str(), target.port)).await?;
        connect_macos_tcp(addresses, tokio::net::TcpStream::connect, async {
            io.lock().await.log(crate::language::text(
                "macOS could not reach the local address. If prompted, allow Termian to access the local network. Check System Settings > Privacy & Security > Local Network.",
            ).to_owned());
        })
        .await
    };
    #[cfg(not(target_os = "macos"))]
    let connect = tokio::net::TcpStream::connect((target.host.as_str(), target.port));
    let stream = tokio::time::timeout(CONNECT_TIMEOUT, connect)
        .await
        .map_err(|_| format!("ssh: connect to host {address}: timed out"))?
        .map_err(|error| format!("ssh: connect to host {address}: {error}"))?;
    let _ = stream.set_nodelay(true);
    io.lock().await.log("TCP connection established".into());
    if target.options.verbose {
        io.lock().await.log(format!("debug1: Connection established to {address}, starting the SSH handshake"));
    }

    let config = target.options.config();
    let verifier = Verifier {
        host: target.host.clone(),
        port: target.port,
        known_hosts: target.known_hosts.clone(),
        io: io.clone(),
        forward_to,
        server_id,
        #[cfg(unix)]
        agent: target.options.agent_forwarding,
    };
    let mut handle = client::connect_stream(Arc::new(config), stream, verifier).await.map_err(|error| match error {
        russh::Error::UnknownKey => "Host key verification failed.".to_string(),
        other => format!("SSH handshake failed: {other}"),
    })?;

    let username = match target.username.clone().filter(|u| !u.is_empty()) {
        Some(user) => user,
        None => io.lock().await.ask("Username", true).await.ok_or("Authentication cancelled.")?,
    };
    {
        let mut io = io.lock().await;
        io.event(LoginEvent::Stage(Stage::Authenticating));
        io.log(format!("Authenticating as '{username}'"));
    }
    // An identity picked for the username brings its own credentials.
    let picked = io.lock().await.take_identity().map(|(password, key)| SshTarget { password, key, ..target.clone() });
    authenticate(&mut handle, &username, picked.as_ref().unwrap_or(target), &io).await?;
    io.lock().await.log("Authentication succeeded".into());
    Ok(handle)
}

async fn authenticate(
    handle: &mut Handle<Verifier>,
    user: &str,
    target: &SshTarget,
    io: &SharedPrompter,
) -> Result<(), String> {
    let failed = |e: russh::Error| e.to_string();
    if handle.authenticate_none(user).await.map_err(failed)?.success() {
        return Ok(());
    }
    if let Some(LoginKey { private_key, passphrase, certificate }) = &target.key {
        io.lock().await.log("Offering the key from the Keychain".into());
        let key = match crate::keys::decode(private_key, passphrase.as_deref()) {
            Ok(key) => Some(key),
            Err(crate::keys::KeyCheck::NeedsPassphrase) => {
                let typed = io.lock().await.ask("Passphrase for the Keychain key", false).await;
                typed.and_then(|p| crate::keys::decode(private_key, Some(&p)).ok())
            }
            Err(_) => None,
        };
        let certificate = certificate.as_deref().and_then(crate::keys::parse_certificate);
        if let (Some(key), Some(certificate)) = (&key, certificate) {
            io.lock().await.log("Offering the certificate of the Keychain key".into());
            if handle.authenticate_openssh_cert(user, Arc::new(key.clone()), certificate).await.map_err(failed)?.success() {
                io.lock().await.log("Authenticated with the Keychain certificate".into());
                return Ok(());
            }
        }
        if let Some(key) = key {
            let hash = handle.best_supported_rsa_hash().await.ok().flatten().flatten();
            if handle.authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash)).await.map_err(failed)?.success() {
                io.lock().await.log("Authenticated with the Keychain key".into());
                return Ok(());
            }
        }
    }
    for path in target.identity_files.iter().filter(|p| p.is_file()) {
        io.lock().await.log(format!("Offering public key {}", path.display()));
        let key = match load_secret_key(path, None) {
            Ok(key) => key,
            Err(_) => {
                let prompt = format!("Passphrase for key '{}'", path.display());
                let Some(passphrase) = io.lock().await.ask(&prompt, false).await else { continue };
                match load_secret_key(path, Some(&passphrase)) {
                    Ok(key) => key,
                    Err(_) => continue,
                }
            }
        };
        let hash = handle.best_supported_rsa_hash().await.ok().flatten().flatten();
        let result = handle.authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash)).await;
        if result.map_err(failed)?.success() {
            io.lock().await.log("Authenticated with public key".into());
            return Ok(());
        }
    }
    if let Some(password) = &target.password {
        io.lock().await.log("Trying the saved password".into());
        if handle.authenticate_password(user, password).await.map_err(failed)?.success() {
            return Ok(());
        }
        io.lock().await.log("The saved password was not accepted".into());
    }
    for _ in 0..PASSWORD_ATTEMPTS {
        let prompt = format!("{user}@{}'s password", target.host);
        let password = io.lock().await.ask(&prompt, false).await.ok_or("Authentication cancelled.")?;
        if handle.authenticate_password(user, &password).await.map_err(failed)?.success() {
            return Ok(());
        }
        if keyboard_interactive(handle, user, io).await? {
            return Ok(());
        }
        io.lock().await.log("Permission denied, please try again".into());
    }
    Err(format!("{user}@{}: Permission denied.", target.host))
}

async fn keyboard_interactive(handle: &mut Handle<Verifier>, user: &str, io: &SharedPrompter) -> Result<bool, String> {
    let mut response =
        handle.authenticate_keyboard_interactive_start(user, None).await.map_err(|e| e.to_string())?;
    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { instructions, prompts, .. } => {
                let mut io = io.lock().await;
                if !instructions.is_empty() {
                    io.log(instructions);
                }
                let mut answers = Vec::new();
                for prompt in prompts {
                    answers.push(io.ask(&prompt.prompt, prompt.echo).await.ok_or("Authentication cancelled.")?);
                }
                drop(io);
                response =
                    handle.authenticate_keyboard_interactive_respond(answers).await.map_err(|e| e.to_string())?;
            }
        }
    }
}

/// Copies shell output to the terminal and keystrokes to the shell until either side closes.
/// How a session that was running ends when the network or server goes away
/// without closing the shell; the tab reconnects after it («Autoreconnect»).
pub const CONNECTION_LOST: &str = "Connection lost";

/// When to reconnect a lost session, as the legacy reconnect queue paces it:
/// six attempts, then a 30-second pause before the next six.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backoff {
    left: u8,
}

impl Default for Backoff {
    fn default() -> Self {
        Self { left: Self::ATTEMPTS }
    }
}

impl Backoff {
    const ATTEMPTS: u8 = 6;

    /// The wait before the next attempt.
    pub fn next_delay(&mut self) -> Duration {
        if self.left == 0 {
            self.left = Self::ATTEMPTS - 1;
            Duration::from_secs(30)
        } else {
            self.left -= 1;
            Duration::from_secs(2)
        }
    }
}

async fn pump(
    channel: russh::Channel<client::Msg>,
    mut input: UnboundedReceiver<Input>,
    events: EventTx,
    mut charset: Option<Charset>,
) -> Result<(), String> {
    let (mut reader, writer) = channel.split();
    loop {
        tokio::select! {
            message = reader.wait() => match message {
                Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                    let data = match charset.as_mut() {
                        Some(charset) => charset.decode(&data),
                        None => data.to_vec(),
                    };
                    for block in data.chunks(32 * 1024) {
                        if events.0.clone().send(SessionEvent::Output(block.to_vec())).await.is_err() { return Ok(()); }
                    }
                }
                Some(ChannelMsg::ExitStatus { exit_status }) if exit_status != 0 => {
                    return Err(format!("Remote shell exited with code {exit_status}"));
                }
                Some(ChannelMsg::Close | ChannelMsg::Eof) => return Ok(()),
                // The channel went away without the shell closing it.
                None => return Err(CONNECTION_LOST.into()),
                _ => {}
            },
            item = input.next() => match item {
                Some(Input::Data(bytes)) => {
                    let bytes = match charset.as_ref() {
                        Some(charset) => charset.encode(&bytes).into(),
                        None => bytes,
                    };
                    writer.data_bytes(bytes).await.map_err(|e| format!("{CONNECTION_LOST}: {e}"))?
                }
                Some(Input::Resize(cols, rows)) => {
                    let _ = writer.window_change(cols as u32, rows as u32, 0, 0).await;
                }
                Some(Input::Answer(_)) => {}
                Some(Input::Close) | None => {
                    let _ = writer.close().await;
                    return Ok(());
                }
            },
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn bounded_output_delivers_bytes_in_order_and_closes() {
        let (_, _, events, mut session) = channels();
        let mut rx = session.take_events().unwrap();
        let producer = std::thread::spawn(move || {
            for n in 0..128 {
                events.send(SessionEvent::Output(vec![n; 4096])).unwrap();
            }
            events.send(SessionEvent::Closed(None)).unwrap();
        });
        futures::executor::block_on(async {
            for n in 0..128 {
                assert!(matches!(rx.next().await, Some(SessionEvent::Output(bytes)) if bytes == [n; 4096]));
            }
            assert!(matches!(rx.next().await, Some(SessionEvent::Closed(None))));
            assert!(rx.next().await.is_none());
        });
        producer.join().unwrap();
    }

    #[test]
    fn ipv6_keeps_the_configured_port() {
        let mut host = crate::vault::tests::host(1, "db", None);
        host.address = "2001:db8::1".into();
        host.port = Some(2222);
        for target in [SshTarget::for_host(&host, &Default::default()), SshTarget::new("[2001:db8::1]:2222", None)] {
            assert_eq!((target.host.as_str(), target.port), ("2001:db8::1", 2222));
        }
    }

    #[test]
    fn a_saved_host_carries_its_environment_agent_and_charset() {
        use crate::vault::{EnvVar, Host, HostSettings, Vault};
        let mut host = crate::vault::tests::host(1, "db", None);
        host.settings = HostSettings { agent_forwarding: true, charset: Some("windows-1251".into()), env: vec![EnvVar { name: "LANG".into(), value: "ru_RU.CP1251".into() }], ..HostSettings::default() };
        let target = SshTarget::for_host(&host, &Vault::default());
        assert_eq!(target.options.env, [("LANG".to_owned(), "ru_RU.CP1251".to_owned())]);
        assert!(target.options.agent_forwarding);
        assert_eq!(target.options.charset.as_deref(), Some("windows-1251"));
        let plain = SshTarget::for_host(&Host { settings: HostSettings::default(), ..host }, &Vault::default());
        assert_eq!((plain.options.env.len(), plain.options.agent_forwarding, plain.options.charset), (0, false, None));
    }

    fn collect_until(session: &mut Session, needle: &str) -> String {
        let mut seen = Vec::new();
        futures::executor::block_on(async {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while std::time::Instant::now() < deadline {
                let next = tokio_timeout(session.events.as_mut().unwrap()).await;
                match next {
                    Some(SessionEvent::Output(bytes)) => {
                        seen.extend(bytes);
                        if String::from_utf8_lossy(&seen).contains(needle) {
                            break;
                        }
                    }
                    Some(SessionEvent::Login(event)) => {
                        let text = match event {
                            LoginEvent::HostKey(info) => format!("<hostkey {} {}>", info.algorithm, info.fingerprint),
                            LoginEvent::Prompt { text, echo } => format!("<prompt {text} echo={echo}>"),
                            LoginEvent::HostKeyChanged(_) => "<changed>".into(),
                            LoginEvent::Stage(stage) => format!("<stage {stage:?}>"),
                            LoginEvent::Log(line) => format!("<log {line}>"),
                        };
                        seen.extend(text.bytes());
                        if String::from_utf8_lossy(&seen).contains(needle) {
                            break;
                        }
                    }
                    Some(SessionEvent::Closed(reason)) => {
                        seen.extend(format!("<closed {reason:?}>").bytes());
                        break;
                    }
                    Some(SessionEvent::Os(os)) => {
                        seen.extend(format!("<os {os}>").bytes());
                        if String::from_utf8_lossy(&seen).contains(needle) {
                            break;
                        }
                    }
                    Some(SessionEvent::History(_)) => {}
                    None => continue,
                }
            }
        });
        String::from_utf8_lossy(&seen).into_owned()
    }

    /// Waits up to 100 ms for the next event without needing a runtime.
    async fn tokio_timeout(events: &mut Receiver<SessionEvent>) -> Option<SessionEvent> {
        let timer = futures::FutureExt::fuse(async_sleep(Duration::from_millis(100)));
        let next = futures::FutureExt::fuse(events.next());
        futures::pin_mut!(timer, next);
        futures::select! { event = next => event, _ = timer => None }
    }

    async fn async_sleep(duration: Duration) {
        let (tx, rx) = futures::channel::oneshot::channel::<()>();
        std::thread::spawn(move || {
            std::thread::sleep(duration);
            let _ = tx.send(());
        });
        let _ = rx.await;
    }

    #[cfg(unix)]
    #[test]
    fn local_command_output_reaches_the_terminal() {
        let mut command = portable_pty::CommandBuilder::new("/bin/sh");
        command.args(["-c", "read line; echo got:$line"]);
        let mut session = local_command(command, 80, 24).unwrap();
        session.write(b"ping\r");
        let output = collect_until(&mut session, "got:ping");
        assert!(output.contains("got:ping"), "{output:?}");
    }

    pub(crate) mod server {
        use super::*;
        use std::io::{Read, Seek, SeekFrom, Write};
        use russh::keys::ssh_key::private::Ed25519Keypair;
        use russh::keys::PrivateKey;
        use russh::server::{self, Auth, Msg, Session as ServerSession};
        use russh::{Channel, ChannelId};
        use russh_sftp::protocol::{Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version};

        pub const PASSWORD: &str = "s3cret";

        pub fn host_key() -> PrivateKey {
            PrivateKey::from(Ed25519Keypair::from_seed(&[7; 32]))
        }

        /// Accepts `PASSWORD`, echoes shell input back with a prefix and,
        /// when `root` is set, serves that directory over SFTP.
        #[derive(Default)]
        struct Echo {
            fault: Option<SftpFault>,
            rsa_user_key: Option<PublicKey>,
            root: Option<std::path::PathBuf>,
            channel: Option<Channel<Msg>>,
            sftp: std::collections::HashSet<ChannelId>,
            #[cfg(windows)]
            modes: Modes,
        }

        /// SFTP over a local directory: listings, file reads and writes,
        /// folders, renames and deletes, enough for the SFTP tab.
        #[cfg(windows)]
        type Modes = Arc<std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, u32>>>;

        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum SftpFault {
            PublicDirectory,
            MissingAttributes,
            DenyMode,
            ReadWindow { start: u64, end: u64 },
        }

        struct Files {
            fault: Option<SftpFault>,
            #[cfg(windows)]
            modes: Modes,
            root: std::path::PathBuf,
            listed: std::collections::HashSet<String>,
            open: std::collections::HashMap<String, std::fs::File>,
            next: u32,
        }

        fn ok(id: u32) -> Status {
            Status { id, status_code: StatusCode::Ok, error_message: "Ok".into(), language_tag: "en".into() }
        }

        fn failed(_: std::io::Error) -> StatusCode {
            StatusCode::Failure
        }

        impl Files {
            fn local(&self, path: &str) -> std::path::PathBuf {
                self.root.join(path.trim_start_matches('/'))
            }

            fn attrs(&self, path: &std::path::Path) -> Result<FileAttributes, StatusCode> {
                let attrs = std::fs::symlink_metadata(path).map(|m| FileAttributes::from(&m)).map_err(|_| StatusCode::NoSuchFile)?;
                #[cfg(windows)]
                let attrs = {
                    let mut attrs = attrs;
                    if let Some(mode) = self.modes.lock().unwrap().get(path) {
                        attrs.permissions = attrs.permissions.map(|p| (p & !0o777) | mode);
                    }
                    attrs
                };
                if self.fault == Some(SftpFault::PublicDirectory) && attrs.is_dir() {
                    return Ok(FileAttributes { permissions: attrs.permissions.map(|p| p | 0o055), ..attrs });
                }
                Ok(attrs)
            }

            fn file(&mut self, handle: &str) -> Result<&mut std::fs::File, StatusCode> {
                self.open.get_mut(handle).ok_or(StatusCode::Failure)
            }
        }

        impl russh_sftp::server::Handler for Files {
            type Error = StatusCode;

            fn unimplemented(&self) -> StatusCode {
                StatusCode::OpUnsupported
            }

            async fn init(&mut self, _: u32, _: std::collections::HashMap<String, String>) -> Result<Version, StatusCode> {
                Ok(Version::new())
            }

            async fn realpath(&mut self, id: u32, _: String) -> Result<Name, StatusCode> {
                Ok(Name { id, files: vec![File::dummy("/")] })
            }

            async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, StatusCode> {
                if !self.local(&path).is_dir() {
                    return Err(StatusCode::NoSuchFile);
                }
                self.listed.remove(&path);
                Ok(Handle { id, handle: path })
            }

            async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, StatusCode> {
                if !self.listed.insert(handle.clone()) {
                    return Err(StatusCode::Eof);
                }
                let mut files = Vec::new();
                for item in std::fs::read_dir(self.local(&handle)).map_err(|_| StatusCode::NoSuchFile)?.flatten() {
                    files.push(File::new(item.file_name().to_string_lossy(), self.attrs(&item.path())?));
                }
                Ok(Name { id, files })
            }

            async fn open(&mut self, id: u32, path: String, flags: OpenFlags, attrs: FileAttributes) -> Result<Handle, StatusCode> {
                let mut options = std::fs::OpenOptions::new();
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut options, attrs.permissions.unwrap_or(0o666));
                #[cfg(windows)]
                if let Some(mode) = attrs.permissions { self.modes.lock().unwrap().insert(self.local(&path), mode); }
                let file = options
                    .create_new(flags.contains(OpenFlags::EXCLUDE))
                    .read(flags.contains(OpenFlags::READ))
                    .write(flags.contains(OpenFlags::WRITE))
                    .create(flags.contains(OpenFlags::CREATE))
                    .truncate(flags.contains(OpenFlags::TRUNCATE))
                    .open(self.local(&path))
                    .map_err(|_| StatusCode::NoSuchFile)?;
                self.next += 1;
                let handle = format!("file{}", self.next);
                self.open.insert(handle.clone(), file);
                Ok(Handle { id, handle })
            }

            async fn read(&mut self, id: u32, handle: String, offset: u64, len: u32) -> Result<Data, StatusCode> {
                if let Some(SftpFault::ReadWindow { start, end }) = self.fault {
                    if offset < start || offset.saturating_add(u64::from(len)) > end {
                        return Err(StatusCode::PermissionDenied);
                    }
                }
                let mut data = vec![0; len as usize];
                let file = self.file(&handle)?;
                file.seek(SeekFrom::Start(offset)).map_err(failed)?;
                let read = file.read(&mut data).map_err(failed)?;
                if read == 0 {
                    return Err(StatusCode::Eof);
                }
                data.truncate(read);
                Ok(Data { id, data })
            }

            async fn write(&mut self, id: u32, handle: String, offset: u64, data: Vec<u8>) -> Result<Status, StatusCode> {
                let file = self.file(&handle)?;
                file.seek(SeekFrom::Start(offset)).map_err(failed)?;
                file.write_all(&data).map_err(failed)?;
                Ok(ok(id))
            }

            async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, StatusCode> {
                let meta = self.file(&handle)?.metadata().map_err(failed)?;
                Ok(Attrs { id, attrs: FileAttributes::from(&meta) })
            }

            async fn setstat(&mut self, id: u32, path: String, attrs: FileAttributes) -> Result<Status, StatusCode> {
                if self.fault == Some(SftpFault::DenyMode) { return Err(StatusCode::PermissionDenied); }
                #[cfg(unix)]
                if let Some(mode) = attrs.permissions {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(self.local(&path), std::fs::Permissions::from_mode(mode & 0o7777)).map_err(failed)?;
                }
                #[cfg(windows)]
                if let Some(mode) = attrs.permissions { self.modes.lock().unwrap().insert(self.local(&path), mode); }
                Ok(ok(id))
            }

            async fn mkdir(&mut self, id: u32, path: String, attrs: FileAttributes) -> Result<Status, StatusCode> {
                let mut builder = std::fs::DirBuilder::new();
                #[cfg(unix)]
                std::os::unix::fs::DirBuilderExt::mode(&mut builder, attrs.permissions.unwrap_or(0o777));
                builder.create(self.local(&path)).map_err(failed)?;
                #[cfg(windows)]
                if let Some(mode) = attrs.permissions { self.modes.lock().unwrap().insert(self.local(&path), mode); }
                Ok(ok(id))
            }

            async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, StatusCode> {
                std::fs::remove_dir(self.local(&path)).map_err(failed)?;
                Ok(ok(id))
            }

            async fn remove(&mut self, id: u32, path: String) -> Result<Status, StatusCode> {
                std::fs::remove_file(self.local(&path)).map_err(failed)?;
                Ok(ok(id))
            }

            async fn rename(&mut self, id: u32, from: String, to: String) -> Result<Status, StatusCode> {
                std::fs::rename(self.local(&from), self.local(&to)).map_err(failed)?;
                Ok(ok(id))
            }

            async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
                let meta = std::fs::metadata(self.local(&path)).map_err(|_| StatusCode::NoSuchFile)?;
                Ok(Attrs { id, attrs: FileAttributes::from(&meta) })
            }

            async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
                if self.fault == Some(SftpFault::MissingAttributes) && path.ends_with("/keep") { return Err(StatusCode::PermissionDenied); }
                Ok(Attrs { id, attrs: self.attrs(&self.local(&path))? })
            }

            async fn close(&mut self, id: u32, handle: String) -> Result<Status, StatusCode> {
                self.open.remove(&handle);
                Ok(ok(id))
            }
        }

        impl server::Handler for Echo {
            type Error = russh::Error;

            async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
                Ok(if user == "root" && password == PASSWORD { Auth::Accept } else { Auth::reject() })
            }

            async fn auth_publickey(&mut self, user: &str, key: server::VerifiedPublicKey<'_>) -> Result<Auth, Self::Error> {
                let accepted = self.rsa_user_key.as_ref().is_some_and(|k| k.key_data() == key.public_key.key_data())
                    || key.public_key.to_openssh().is_ok_and(|k| k.starts_with(crate::keys::tests::PLAIN_PUBLIC));
                Ok(if user == "root" && accepted { Auth::Accept } else { Auth::reject() })
            }

            async fn auth_openssh_certificate(&mut self, user: &str, certificate: &russh::keys::Certificate) -> Result<Auth, Self::Error> {
                let accepted = certificate.valid_principals().iter().any(|p| p == user) && certificate.key_id() == "termian-test";
                Ok(if user == "root" && accepted { Auth::Accept } else { Auth::reject() })
            }

            async fn channel_open_session(
                &mut self,
                channel: Channel<Msg>,
                reply: server::ChannelOpenHandle,
                _: &mut ServerSession,
            ) -> Result<(), Self::Error> {
                self.channel = Some(channel);
                reply.accept().await;
                Ok(())
            }

            async fn subsystem_request(
                &mut self,
                channel: ChannelId,
                name: &str,
                session: &mut ServerSession,
            ) -> Result<(), Self::Error> {
                match (name, self.root.clone(), self.channel.take()) {
                    ("sftp", Some(root), Some(stream)) => {
                        session.channel_success(channel)?;
                        self.sftp.insert(channel);
                        let files = Files { root, fault: self.fault, #[cfg(windows)] modes: self.modes.clone(), listed: Default::default(), open: Default::default(), next: 0 };
                        tokio::spawn(russh_sftp::server::run(stream.into_stream(), files));
                    }
                    _ => session.channel_failure(channel)?,
                }
                Ok(())
            }

            #[allow(clippy::too_many_arguments)]
            async fn pty_request(
                &mut self,
                channel: ChannelId,
                term: &str,
                cols: u32,
                rows: u32,
                _: u32,
                _: u32,
                _: &[(russh::Pty, u32)],
                session: &mut ServerSession,
            ) -> Result<(), Self::Error> {
                session.data(channel, format!("pty {term} {cols}x{rows}\r\n").into_bytes())?;
                session.channel_success(channel)
            }

            async fn exec_request(&mut self, channel: ChannelId, data: &[u8], session: &mut ServerSession) -> Result<(), Self::Error> {
                let reply: &[u8] = match data {
                    b"echo $SHELL" => b"/bin/bash\n",
                    script if script.starts_with(b"env BASH_ENV=/dev/null '/bin/bash' ") => b"__TERMIAN_HISTORY_BEGIN__\n  1 __TERMIAN_HISTORY__ fixture-history-command\n__TERMIAN_HISTORY_END__\n",
                    script if script.starts_with(b"HISTFILE=;") => b"PRETTY_NAME=\"Ubuntu 24.04 LTS\" NAME=\"Ubuntu\" VERSION_ID=\"24.04\"\n",
                    _ => b"",
                };
                // `fail:` scripts exit 1 with the rest as output (key export errors).
                let (reply, status) = match data.strip_prefix(b"fail:") {
                    Some(message) => (message, 1),
                    None => (reply, 0),
                };
                session.channel_success(channel)?;
                session.data(channel, reply.to_vec())?;
                session.exit_status_request(channel, status)?;
                session.eof(channel)?;
                session.close(channel)
            }

            async fn shell_request(&mut self, channel: ChannelId, session: &mut ServerSession) -> Result<(), Self::Error> {
                session.data(channel, b"welcome\r\n".to_vec())?;
                session.channel_success(channel)
            }

            async fn channel_open_direct_tcpip(
                &mut self,
                channel: Channel<Msg>,
                host: &str,
                port: u32,
                _: &str,
                _: u32,
                reply: server::ChannelOpenHandle,
                _: &mut ServerSession,
            ) -> Result<(), Self::Error> {
                // Local and dynamic forwarding: connect to the asked address from here.
                self.sftp.insert(channel.id());
                let address = format!("{host}:{port}");
                reply.accept().await;
                tokio::spawn(async move {
                    if let Ok(mut target) = tokio::net::TcpStream::connect(address).await {
                        let mut stream = channel.into_stream();
                        let _ = tokio::io::copy_bidirectional(&mut target, &mut stream).await;
                    }
                });
                Ok(())
            }

            async fn tcpip_forward(&mut self, address: &str, port: &mut u32, session: &mut ServerSession) -> Result<bool, Self::Error> {
                // Remote forwarding: listen here and send each connection back to the client.
                let Ok(listener) = tokio::net::TcpListener::bind((address, *port as u16)).await else { return Ok(false) };
                *port = u32::from(listener.local_addr().map(|a| a.port()).unwrap_or(0));
                let (handle, address, bound) = (session.handle(), address.to_owned(), *port);
                tokio::spawn(async move {
                    while let Ok((mut socket, peer)) = listener.accept().await {
                        let Ok(channel) = handle.channel_open_forwarded_tcpip(address.clone(), bound, peer.ip().to_string(), u32::from(peer.port())).await else { break };
                        tokio::spawn(async move {
                            let mut stream = channel.into_stream();
                            let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
                        });
                    }
                });
                Ok(true)
            }

            async fn data(&mut self, channel: ChannelId, data: &[u8], session: &mut ServerSession) -> Result<(), Self::Error> {
                if self.sftp.contains(&channel) {
                    return Ok(()); // the SFTP task reads this channel itself
                }
                match data {
                    b"exit" => {
                        session.exit_status_request(channel, 0)?;
                        session.eof(channel)?;
                        session.close(channel)
                    }
                    b"drop" => session.disconnect(russh::Disconnect::ByApplication, "gone", ""),
                    _ => session.data(channel, [b"echo:".as_slice(), data].concat()),
                }
            }
        }

        /// Starts a one-connection server on a free port.
        pub fn start() -> u16 {
            start_with(None, None)
        }

        pub fn start_with_sftp(root: std::path::PathBuf) -> u16 {
            start_with(Some(root), None)
        }

        pub(super) fn start_with(root: Option<std::path::PathBuf>, rsa_user_key: Option<PublicKey>) -> u16 {
            start_with_fault(root, rsa_user_key, None)
        }

        pub fn start_with_fault(root: Option<std::path::PathBuf>, rsa_user_key: Option<PublicKey>, fault: Option<SftpFault>) -> u16 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            listener.set_nonblocking(true).unwrap();
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                runtime.block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                    let config = Arc::new(server::Config {
                        keys: vec![host_key()],
                        auth_rejection_time: Duration::from_millis(10),
                        ..Default::default()
                    });
                    let (socket, _) = listener.accept().await.unwrap();
                    let running = server::run_stream(config, socket, Echo { root, rsa_user_key, fault, #[cfg(windows)] modes: Default::default(), channel: None, sftp: Default::default() }).await.unwrap();
                    let _ = running.await;
                });
            });
            port
        }
    }

    fn target_for(port: u16, known_hosts: PathBuf) -> SshTarget {
        target(port, known_hosts)
    }

    fn target(port: u16, known_hosts: PathBuf) -> SshTarget {
        SshTarget {
            host: "127.0.0.1".into(),
            port,
            username: Some("root".into()),
            known_hosts,
            identity_files: vec![],
            password: None,
            key: None,
            options: SshOptions::default(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn local_shell_imports_fixture_history_only_when_enabled_without_executing_it() {
        use std::os::unix::fs::PermissionsExt;
        for enabled in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let home = dir.path().to_str().unwrap().replace('\'', "'\\''");
            let path = dir.path().join("bash");
            std::fs::write(&path, format!("#!/bin/sh\nexport HOME='{home}'\nif [ \"$1\" = -l ]; then\n  printf 'fixture-prompt\\n'\n  while IFS= read -r line; do printf 'echo:%s\\n' \"$line\"; done\nelse\n  touch \"$HOME/import-read\"\n  exec /bin/bash \"$@\"\nfi\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            let script = format!("touch '{home}/must-not-exist'");
            let history = dir.path().join(".bash_history");
            std::fs::write(&history, format!("{script}\n")).unwrap();
            let mut session = local_shell(path.to_str(), "xterm", 80, 24, enabled).unwrap();
            let mut commands = Vec::new();
            let mut output = String::new();
            futures::executor::block_on(async {
                for _ in 0..20 {
                    match tokio_timeout(session.events.as_mut().unwrap()).await {
                        Some(SessionEvent::History(batch)) => commands = batch,
                        Some(SessionEvent::Output(bytes)) => output.push_str(&String::from_utf8_lossy(&bytes)),
                        _ => {}
                    }
                    if enabled && !commands.is_empty() && output.contains("fixture-prompt") { break; }
                }
            });
            assert!(output.contains("fixture-prompt"));
            assert!(!output.contains("must-not-exist"));
            assert_eq!(commands, if enabled { vec![script.clone()] } else { vec![] });
            assert_eq!(dir.path().join("import-read").exists(), enabled);
            assert!(!dir.path().join("must-not-exist").exists());
            assert_eq!(std::fs::read_to_string(history).unwrap(), format!("{script}\n"));
            session.write(b"still-alive\n");
            assert!(collect_until(&mut session, "echo:still-alive").contains("echo:still-alive"));
        }
    }

    #[test]
    fn ssh_imports_history_separately_from_terminal_output_only_when_enabled() {
        for enabled in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut target = target(server::start(), dir.path().join("known_hosts"));
            target.password = Some(server::PASSWORD.into());
            target.options.import_history = enabled;
            let mut session = ssh(target, 80, 24);
            collect_until(&mut session, "<hostkey");
            session.answer(Answer::HostKey(HostKeyDecision::Once));
            let mut output = String::new();
            let mut commands = Vec::new();
            futures::executor::block_on(async {
                for _ in 0..20 {
                    match tokio_timeout(session.events.as_mut().unwrap()).await {
                        Some(SessionEvent::History(batch)) => commands = batch,
                        Some(SessionEvent::Output(bytes)) => output.push_str(&String::from_utf8_lossy(&bytes)),
                        _ => {}
                    }
                    if enabled && !commands.is_empty() && output.contains("welcome") { break; }
                }
            });
            assert!(output.contains("welcome"));
            assert!(!output.contains("fixture-history-command"));
            assert_eq!(commands, if enabled { vec!["fixture-history-command".to_owned()] } else { vec![] });
            session.write(b"still-alive");
            assert!(collect_until(&mut session, "echo:still-alive").contains("echo:still-alive"));
        }
    }

    #[test]
    fn terminal_settings_reach_the_connection() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.password = Some(server::PASSWORD.into());
        target.options = SshOptions { term: "xterm-16color".into(), keepalive: 5, pqc_kex: false, ..SshOptions::default() };
        assert!(!target.options.kex().contains(&russh::kex::MLKEM768X25519_SHA256));
        assert!(SshOptions::default().kex().contains(&russh::kex::MLKEM768X25519_SHA256));
        assert_eq!(target.options.config().keepalive_interval, Some(Duration::from_secs(5)));
        assert_eq!(SshOptions::default().config().keepalive_interval, None, "0 sends no keepalives");
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        let output = collect_until(&mut session, "welcome");
        assert!(output.contains("pty xterm-16color 80x24"), "without the post-quantum exchange too: {output:?}");
    }

    #[test]
    fn the_os_is_read_as_the_legacy_detector_reads_it() {
        assert_eq!(os_from_version("SSH-2.0-OpenSSH_9.2p1 Debian-2+deb12u3"), Some("debian"));
        assert_eq!(os_from_version("SSH-2.0-OpenSSH_9.6p1 Ubuntu-3ubuntu13"), Some("ubuntu"));
        assert_eq!(os_from_version("SSH-2.0-OpenSSH_9.8"), None);
        assert_eq!(parse_os("Darwin\n"), Some("osx"));
        assert_eq!(parse_os("FreeBSD"), Some("freebsd"));
        assert_eq!(parse_os("PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\" NAME=\"Debian GNU/Linux\" ID=debian"), Some("debian"));
        assert_eq!(parse_os("NAME=\"CentOS Linux\" VERSION=\"7\""), Some("centos"));
        assert_eq!(parse_os("MSYS_NT-10.0"), Some("windows"));
        assert_eq!(parse_os("something else"), None);
        assert_eq!(detect_script("/usr/bin/fish"), DETECT_OS_FISH_SCRIPT);
        assert_eq!(detect_script("/bin/bash"), DETECT_OS_SCRIPT);
        assert_eq!(detect_script("bad command \"echo\""), DETECT_ROUTEROS_SCRIPT);
    }

    #[test]
    fn an_identity_picked_for_the_username_supplies_its_password() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.username = None;
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        collect_until(&mut session, "<prompt Username");
        session.answer(Answer::Identity { username: "root".into(), password: Some(server::PASSWORD.into()), key: None });
        let output = collect_until(&mut session, "welcome");
        assert!(!output.contains("<prompt"), "no password question after picking an identity: {output:?}");
    }

    #[test]
    fn detects_the_os_of_a_host_after_login() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.password = Some(server::PASSWORD.into());
        target.options.detect_os = true;
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        let output = collect_until(&mut session, "<os ");
        assert!(output.contains("welcome") && output.contains("<os ubuntu>"), "{output:?}");
    }

    #[test]
    fn reconnects_six_times_then_pauses() {
        let mut backoff = Backoff::default();
        let delays: Vec<u64> = (0..14).map(|_| backoff.next_delay().as_secs()).collect();
        assert_eq!(delays, [2, 2, 2, 2, 2, 2, 30, 2, 2, 2, 2, 2, 30, 2]);
    }

    #[test]
    fn a_lost_connection_is_told_apart_from_exit() {
        for (command, expected) in [("exit", "<closed None>".to_owned()), ("drop", format!("<closed Some(\"{CONNECTION_LOST}\")>"))] {
            let dir = tempfile::tempdir().unwrap();
            let mut target = target(server::start(), dir.path().join("known_hosts"));
            target.password = Some(server::PASSWORD.into());
            let mut session = ssh(target, 80, 24);
            collect_until(&mut session, "<hostkey");
            session.answer(Answer::HostKey(HostKeyDecision::Once));
            collect_until(&mut session, "welcome");
            session.write(command.as_bytes());
            let output = collect_until(&mut session, "<closed");
            assert!(output.contains(&expected), "{command}: {output:?}");
        }
    }

    #[test]
    fn a_saved_password_logs_in_without_asking() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.password = Some(server::PASSWORD.into());
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        let output = collect_until(&mut session, "welcome");
        assert!(!output.contains("<prompt"), "no password question: {output:?}");
    }

    #[test]
    fn the_keychain_key_logs_in_and_its_passphrase_is_asked_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.key = Some(LoginKey::new(crate::keys::tests::PLAIN, None));
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        let output = collect_until(&mut session, "welcome");
        assert!(!output.contains("<prompt"), "{output:?}");

        // A key the server does not accept falls through to the password.
        let mut target = target_for(server::start(), dir.path().join("known_hosts2"));
        target.key = Some(LoginKey::new(crate::keys::tests::ENCRYPTED, None));
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        let output = collect_until(&mut session, "echo=false>");
        assert!(output.contains("<prompt Passphrase for the Keychain key"), "{output:?}");
        session.answer(Answer::Text(Some(crate::keys::tests::PASSPHRASE.into())));
        assert!(collect_until(&mut session, "echo=false>").contains("password"));
    }

    #[test]
    fn rsa_key_authenticates_over_ssh() {
        let dir = tempfile::tempdir().unwrap();
        let pair = crate::keys::generate(crate::keys::KeyAlgorithm::Rsa(2048), "").unwrap();
        let port = server::start_with(None, Some(PublicKey::from_openssh(&pair.public_key).unwrap()));
        let mut target = target(port, dir.path().join("known_hosts"));
        target.key = Some(LoginKey::new(pair.private_key, None));
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        assert!(collect_until(&mut session, "welcome").contains("welcome"));
    }

    #[test]
    fn the_keychain_certificate_is_offered_before_the_bare_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.key = Some(LoginKey { certificate: Some(crate::keys::tests::PLAIN_CERTIFICATE.into()), ..LoginKey::new(crate::keys::tests::PLAIN, None) });
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        let output = collect_until(&mut session, "welcome");
        assert!(output.contains("Authenticated with the Keychain certificate"), "{output:?}");
    }

    #[test]
    fn generated_encrypted_key_prompts_only_when_passphrase_was_not_saved() {
        let dir = tempfile::tempdir().unwrap();
        let key = crate::keys::generate(crate::keys::KeyAlgorithm::Ed25519, "generated passphrase").unwrap();
        for saved in [false, true] {
            let mut target = target_for(server::start(), dir.path().join(format!("known-{saved}")));
            target.key = Some(LoginKey::new(key.private_key.clone(), saved.then(|| "generated passphrase".into())));
            target.password = Some(server::PASSWORD.into());
            let mut session = ssh(target, 80, 24);
            collect_until(&mut session, "<hostkey");
            session.answer(Answer::HostKey(HostKeyDecision::Once));
            if !saved {
                let output = collect_until(&mut session, "echo=false>");
                assert!(output.contains("<prompt Passphrase for the Keychain key"), "{output:?}");
                session.answer(Answer::Text(Some("generated passphrase".into())));
            }
            let output = collect_until(&mut session, "welcome");
            assert!(!output.contains("generated passphrase"), "passphrase must not leak");
            if saved { assert!(!output.contains("<prompt"), "{output:?}"); }
        }
    }

    #[test]
    fn a_wrong_saved_password_falls_back_to_asking() {
        let dir = tempfile::tempdir().unwrap();
        let mut target = target(server::start(), dir.path().join("known_hosts"));
        target.password = Some("stale".into());
        let mut session = ssh(target, 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        assert!(collect_until(&mut session, "echo=false>").contains("<prompt root@127.0.0.1's password"));
    }

    #[test]
    fn ssh_asks_to_trust_new_host_then_password_then_runs_shell() {
        let dir = tempfile::tempdir().unwrap();
        let known_hosts = dir.path().join("known_hosts");
        let mut session = ssh(target(server::start(), known_hosts.clone()), 100, 30);

        let output = collect_until(&mut session, "<hostkey");
        assert!(output.contains("<stage Connecting>") && output.contains("<stage Verifying>"), "{output:?}");
        assert!(output.contains("<hostkey ED25519 "), "{output:?}");
        session.answer(Answer::HostKey(HostKeyDecision::Save));
        let output = collect_until(&mut session, "echo=false>");
        assert!(output.contains("<prompt root@127.0.0.1's password echo=false>"), "{output:?}");
        session.answer(Answer::Text(Some(server::PASSWORD.into())));
        let output = collect_until(&mut session, "welcome");
        assert!(output.contains("<stage Ready>"), "{output:?}");
        assert!(output.contains("pty xterm-256color 100x30"), "{output:?}");
        assert!(!output.contains(server::PASSWORD), "password must not leak");

        session.write(b"ls");
        assert!(collect_until(&mut session, "echo:ls").contains("echo:ls"));
        assert!(std::fs::read_to_string(&known_hosts).unwrap().contains("ssh-ed25519"));
    }

    #[test]
    fn ssh_refuses_a_changed_host_key() {
        let dir = tempfile::tempdir().unwrap();
        let known_hosts = dir.path().join("known_hosts");
        let port = server::start();
        let other = russh::keys::PrivateKey::from(russh::keys::ssh_key::private::Ed25519Keypair::from_seed(&[8; 32]));
        crate::known_hosts::learn("127.0.0.1", port, other.public_key(), &known_hosts).unwrap();

        let mut session = ssh(target(port, known_hosts), 80, 24);
        let output = collect_until(&mut session, "<closed");
        assert!(output.contains("<changed>"), "{output:?}");
        assert!(output.contains("Host key verification failed"), "{output:?}");
        assert!(!output.contains("<hostkey"), "a changed key is never offered for trust: {output:?}");
    }

    #[test]
    fn declining_the_host_key_closes_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = ssh(target(server::start(), dir.path().join("known_hosts")), 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Reject));
        assert!(collect_until(&mut session, "<closed").contains("Host key verification failed"));
        assert!(!dir.path().join("known_hosts").exists());
    }

    #[test]
    fn continue_trusts_the_key_without_saving_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = ssh(target(server::start(), dir.path().join("known_hosts")), 80, 24);
        collect_until(&mut session, "<hostkey");
        session.answer(Answer::HostKey(HostKeyDecision::Once));
        collect_until(&mut session, "echo=false>");
        session.answer(Answer::Text(Some(server::PASSWORD.into())));
        assert!(collect_until(&mut session, "welcome").contains("welcome"));
        assert!(!dir.path().join("known_hosts").exists());
    }

    #[test]
    fn unreachable_host_reports_the_error() {
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let dir = tempfile::tempdir().unwrap();
        let mut session = ssh(target(port, dir.path().join("kh")), 80, 24);
        let output = collect_until(&mut session, "<closed");
        assert!(output.contains(&format!("connect to host 127.0.0.1:{port}")), "{output:?}");
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn retries_only_the_unreachable_local_address() {
        use std::{cell::Cell, future::ready, io::Error, net::SocketAddr};

        let local: SocketAddr = "10.0.0.1:22".parse().unwrap();
        let refused: SocketAddr = "10.0.0.2:22".parse().unwrap();
        let public: SocketAddr = "192.0.2.1:22".parse().unwrap();
        let notice = Cell::new(0);
        let mut attempts = Vec::new();
        let result = connect_macos_tcp(
            [local, refused, public],
            |endpoint| {
                let allowed = endpoint == local && attempts.contains(&local);
                attempts.push(endpoint);
                ready(if allowed {
                    Ok(endpoint)
                } else {
                    Err(Error::from_raw_os_error(if endpoint == refused {
                        libc::ECONNREFUSED
                    } else {
                        libc::EHOSTUNREACH
                    }))
                })
            },
            async { notice.set(notice.get() + 1) },
        )
        .await
        .unwrap();
        assert_eq!(result, local);
        assert_eq!(attempts, [local, refused, public, local]);
        assert_eq!(notice.get(), 1);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn unreachable_local_address_respects_the_connection_deadline() {
        use std::{cell::Cell, future::ready, io::Error};

        let attempts = Cell::new(0);
        let result = tokio::time::timeout(
            Duration::from_millis(20),
            connect_macos_tcp(
                ["[fd00::1]:22".parse().unwrap()],
                |_| {
                    attempts.set(attempts.get() + 1);
                    ready(Err::<(), _>(Error::from_raw_os_error(libc::EHOSTUNREACH)))
                },
                async {},
            ),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(attempts.get(), 1);
    }

    #[test]
    fn parses_quick_connect_input() {
        let t = parse_destination("ssh root@10.0.0.1:2001").unwrap();
        assert_eq!((t.username.as_deref(), t.host.as_str(), t.port), (Some("root"), "10.0.0.1", 2001));
        assert_eq!(parse_destination("example.com").unwrap().username, None);
        assert!(parse_destination("").is_none());
        assert!(parse_destination("two words").is_none());
        assert!(parse_destination("@host").is_none());
    }

    #[test]
    fn parses_host_with_port() {
        let t = SshTarget::new("example.com:2001", None);
        assert_eq!((t.host.as_str(), t.port), ("example.com", 2001));
        assert_eq!(SshTarget::new("example.com", None).port, 22);
    }

    #[test]
    fn serial_charsets_translate_both_ways() {
        let mut cp1251 = Charset::new(crate::serial::encoding("windows-1251").unwrap());
        assert_eq!(cp1251.decode(&[0xcf, 0xf0, 0xe8]), "При".as_bytes());
        assert_eq!(cp1251.encode("да".as_bytes()), [0xe4, 0xe0]);
        let mut sjis = Charset::new(crate::serial::encoding("Shift_JIS").unwrap());
        let mut out = sjis.decode(&[0x82]);
        out.extend(sjis.decode(&[0xa0]));
        assert_eq!(out, "あ".as_bytes(), "a character split between reads");
        let mut utf8 = Charset::new(encoding_rs::UTF_8);
        assert_eq!(utf8.decode(b"\x1b[0m ok"), b"\x1b[0m ok");
    }

    #[test]
    fn a_port_the_system_does_not_list_is_refused() {
        let options = crate::serial::SerialOptions { path: "/dev/not-a-port".into(), ..Default::default() };
        assert!(serial(&options).err().unwrap().contains("not found"));
    }
}