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
use futures::StreamExt;
use russh::client::{self, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{
    check_known_hosts_path, known_hosts::learn_known_hosts_path, load_secret_key, HashAlg, PrivateKeyWithHashAlg, PublicKey,
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
    events: Option<UnboundedReceiver<SessionEvent>>,
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
    pub fn take_events(&mut self) -> Option<UnboundedReceiver<SessionEvent>> {
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

fn channels() -> (UnboundedSender<Input>, UnboundedReceiver<Input>, UnboundedSender<SessionEvent>, Session) {
    let (input, input_rx) = unbounded();
    let (events_tx, events) = unbounded();
    (input.clone(), input_rx, events_tx, Session { input, events: Some(events) })
}

// ------------------------------------------------------------------ local --

/// Starts the user's login shell.
pub fn local_shell(columns: u16, rows: u16) -> Result<Session, String> {
    local_command(portable_pty::CommandBuilder::new_default_prog(), columns, rows)
}

pub fn local_command(command: portable_pty::CommandBuilder, columns: u16, rows: u16) -> Result<Session, String> {
    use portable_pty::{native_pty_system, PtySize};
    let size = |cols: u16, rows: u16| PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
    let pair = native_pty_system().openpty(size(columns, rows)).map_err(|e| e.to_string())?;
    let mut command = command;
    command.env("TERM", "xterm-256color");
    let mut child = pair.slave.spawn_command(command).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let mut writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let (_, input_rx, events_tx, session) = channels();

    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if events_tx.unbounded_send(SessionEvent::Output(buffer[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
        let status = child.wait().ok();
        let reason = status.filter(|s| !s.success()).map(|s| format!("Process exited with code {}", s.exit_code()));
        let _ = events_tx.unbounded_send(SessionEvent::Closed(reason));
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
}

impl SshTarget {
    /// Port 22, the user's default OpenSSH keys, and the native client's known_hosts.
    pub fn new(host: &str, username: Option<String>) -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let (host, port) = match host.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') && p.parse::<u16>().is_ok() => (h.to_owned(), p.parse().unwrap()),
            _ => (host.to_owned(), 22),
        };
        Self {
            host,
            port,
            username,
            known_hosts: dirs::data_dir().unwrap_or_default().join("termian-native").join("known_hosts"),
            identity_files: ["id_ed25519", "id_ecdsa", "id_rsa"].iter().map(|f| home.join(".ssh").join(f)).collect(),
        }
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
                let _ = events_tx.unbounded_send(SessionEvent::Closed(Some(error.to_string())));
                return;
            }
        };
        let result = runtime.block_on(run_ssh(target, input_rx, events_tx.clone(), (columns, rows)));
        let _ = events_tx.unbounded_send(SessionEvent::Closed(result.err()));
    });
    session
}

/// Where login questions go; the UI answers them on the connection screen.
pub(crate) trait Prompter: Send {
    fn event(&mut self, event: LoginEvent);
    fn ask<'a>(&'a mut self, prompt: &'a str, echo: bool) -> BoxFuture<'a, Option<String>>;
    fn host_key(&mut self, info: HostKeyInfo) -> BoxFuture<'_, HostKeyDecision>;

    fn log(&mut self, line: String) {
        self.event(LoginEvent::Log(line));
    }
}

pub(crate) type SharedPrompter = Arc<Mutex<dyn Prompter>>;

/// Forwards login events through `emit` and waits for answers on `answers`.
pub(crate) struct EventPrompter<F: FnMut(LoginEvent) + Send> {
    pub emit: F,
    pub answers: UnboundedReceiver<Answer>,
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
                _ => None,
            }
        })
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
}

fn key_label(key: &PublicKey) -> String {
    crate::vault::key_kind(key.algorithm().as_str())
}

impl client::Handler for Verifier {
    type Error = russh::Error;

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
        match check_known_hosts_path(&self.host, self.port, &key, &self.known_hosts) {
            Ok(true) => {
                io.log("Host key matches a known_hosts entry".into());
                return Ok(true);
            }
            Err(russh::keys::Error::KeyChanged { line }) => {
                io.log(format!("Host key differs from known_hosts line {line}"));
                io.event(LoginEvent::HostKeyChanged(info));
                return Ok(false);
            }
            _ => {}
        }
        match io.host_key(info).await {
            HostKeyDecision::Reject => Ok(false),
            HostKeyDecision::Once => {
                io.log("Host key accepted for this session".into());
                Ok(true)
            }
            HostKeyDecision::Save => {
                match learn_known_hosts_path(&self.host, self.port, &key, &self.known_hosts) {
                    Ok(()) => io.log(format!("Permanently added '{}' to the list of known hosts", self.host)),
                    Err(error) => io.log(format!("Could not save the host key: {error}")),
                }
                Ok(true)
            }
        }
    }
}

async fn run_ssh(
    target: SshTarget,
    mut input: UnboundedReceiver<Input>,
    events: UnboundedSender<SessionEvent>,
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
            let _ = emit_events.unbounded_send(SessionEvent::Login(event));
        },
        answers,
    }));
    let handle = connect(&target, prompter.clone()).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    let (cols, rows) = *latest_size.lock().unwrap();
    channel.request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    channel.request_shell(true).await.map_err(|e| e.to_string())?;
    prompter.lock().await.event(LoginEvent::Stage(Stage::Ready));
    pump(channel, shell_input, events).await
}

/// Opens an authenticated SSH connection, asking `io` whatever login needs.
pub(crate) async fn connect(target: &SshTarget, io: SharedPrompter) -> Result<Handle<Verifier>, String> {
    let address = format!("{}:{}", target.host, target.port);
    {
        let mut io = io.lock().await;
        io.event(LoginEvent::Stage(Stage::Connecting));
        io.log(format!("Connecting to {address}"));
    }
    let stream = tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::TcpStream::connect((target.host.as_str(), target.port)))
        .await
        .map_err(|_| format!("ssh: connect to host {address}: timed out"))?
        .map_err(|error| format!("ssh: connect to host {address}: {error}"))?;
    let _ = stream.set_nodelay(true);
    io.lock().await.log("TCP connection established".into());

    let config = client::Config { keepalive_interval: Some(Duration::from_secs(30)), keepalive_max: 3, ..Default::default() };
    let verifier =
        Verifier { host: target.host.clone(), port: target.port, known_hosts: target.known_hosts.clone(), io: io.clone() };
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
    authenticate(&mut handle, &username, target, &io).await?;
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
async fn pump(
    channel: russh::Channel<client::Msg>,
    mut input: UnboundedReceiver<Input>,
    events: UnboundedSender<SessionEvent>,
) -> Result<(), String> {
    let (mut reader, writer) = channel.split();
    loop {
        tokio::select! {
            message = reader.wait() => match message {
                Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                    let _ = events.unbounded_send(SessionEvent::Output(data.to_vec()));
                }
                Some(ChannelMsg::ExitStatus { exit_status }) if exit_status != 0 => {
                    return Err(format!("Remote shell exited with code {exit_status}"));
                }
                Some(ChannelMsg::Close | ChannelMsg::Eof) | None => return Ok(()),
                _ => {}
            },
            item = input.next() => match item {
                Some(Input::Data(bytes)) => writer.data_bytes(bytes).await.map_err(|e| e.to_string())?,
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
                    None => continue,
                }
            }
        });
        String::from_utf8_lossy(&seen).into_owned()
    }

    /// Waits up to 100 ms for the next event without needing a runtime.
    async fn tokio_timeout(events: &mut UnboundedReceiver<SessionEvent>) -> Option<SessionEvent> {
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
        use russh::keys::ssh_key::private::Ed25519Keypair;
        use russh::keys::PrivateKey;
        use russh::server::{self, Auth, Msg, Session as ServerSession};
        use russh::{Channel, ChannelId};
        use russh_sftp::protocol::{Attrs, File, FileAttributes, Handle, Name, Status, StatusCode, Version};

        pub const PASSWORD: &str = "s3cret";

        pub fn host_key() -> PrivateKey {
            PrivateKey::from(Ed25519Keypair::from_seed(&[7; 32]))
        }

        /// Accepts `PASSWORD`, echoes shell input back with a prefix and,
        /// when `root` is set, serves that directory over SFTP.
        #[derive(Default)]
        struct Echo {
            root: Option<std::path::PathBuf>,
            channel: Option<Channel<Msg>>,
            sftp: Option<ChannelId>,
        }

        /// Read-only SFTP over a local directory, enough for listings.
        struct Files {
            root: std::path::PathBuf,
            listed: std::collections::HashSet<String>,
        }

        impl Files {
            fn local(&self, path: &str) -> std::path::PathBuf {
                self.root.join(path.trim_start_matches('/'))
            }

            fn attrs(&self, path: &std::path::Path) -> Result<FileAttributes, StatusCode> {
                std::fs::symlink_metadata(path).map(|m| FileAttributes::from(&m)).map_err(|_| StatusCode::NoSuchFile)
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

            async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
                Ok(Attrs { id, attrs: self.attrs(&self.local(&path))? })
            }

            async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
                Ok(Attrs { id, attrs: self.attrs(&self.local(&path))? })
            }

            async fn close(&mut self, id: u32, _: String) -> Result<Status, StatusCode> {
                Ok(Status { id, status_code: StatusCode::Ok, error_message: "Ok".into(), language_tag: "en".into() })
            }
        }

        impl server::Handler for Echo {
            type Error = russh::Error;

            async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
                Ok(if user == "root" && password == PASSWORD { Auth::Accept } else { Auth::reject() })
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
                        self.sftp = Some(channel);
                        let files = Files { root, listed: Default::default() };
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

            async fn shell_request(&mut self, channel: ChannelId, session: &mut ServerSession) -> Result<(), Self::Error> {
                session.data(channel, b"welcome\r\n".to_vec())?;
                session.channel_success(channel)
            }

            async fn data(&mut self, channel: ChannelId, data: &[u8], session: &mut ServerSession) -> Result<(), Self::Error> {
                if self.sftp == Some(channel) {
                    return Ok(()); // the SFTP task reads this channel itself
                }
                session.data(channel, [b"echo:".as_slice(), data].concat())
            }
        }

        /// Starts a one-connection server on a free port.
        pub fn start() -> u16 {
            start_with(None)
        }

        pub fn start_with_sftp(root: std::path::PathBuf) -> u16 {
            start_with(Some(root))
        }

        fn start_with(root: Option<std::path::PathBuf>) -> u16 {
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
                    let running = server::run_stream(config, socket, Echo { root, channel: None, sftp: None }).await.unwrap();
                    let _ = running.await;
                });
            });
            port
        }
    }

    fn target(port: u16, known_hosts: PathBuf) -> SshTarget {
        SshTarget {
            host: "127.0.0.1".into(),
            port,
            username: Some("root".into()),
            known_hosts,
            identity_files: vec![],
        }
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
        learn_known_hosts_path("127.0.0.1", port, other.public_key(), &known_hosts).unwrap();

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
}
