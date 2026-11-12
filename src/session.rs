//! Byte streams behind terminal tabs: a local shell (PTY) or an SSH shell.
//! Interactive questions during SSH login (unknown host key, password,
//! keyboard-interactive) are asked inside the terminal, like OpenSSH does.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use russh::client::{self, Handle, KeyboardInteractiveAuthResponse};
use russh::keys::{
    check_known_hosts_path, known_hosts::learn_known_hosts_path, load_secret_key, HashAlg, PrivateKeyWithHashAlg, PublicKey,
    PublicKeyOrCertificate,
};
use russh::ChannelMsg;
use tokio::sync::Mutex;

#[derive(Debug, PartialEq)]
pub enum SessionEvent {
    Output(Vec<u8>),
    /// The session ended; `Some` carries the reason when it failed.
    Closed(Option<String>),
}

enum Input {
    Data(Vec<u8>),
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
        let io = Arc::new(Mutex::new(Terminal { input: input_rx, output: events_tx.clone(), size: (columns, rows) }));
        let result = runtime.block_on(run_ssh(target, io.clone()));
        let reason = result.err();
        if let Some(reason) = &reason {
            let _ = events_tx.unbounded_send(SessionEvent::Output(format!("\r\n{reason}\r\n").into_bytes()));
        }
        let _ = events_tx.unbounded_send(SessionEvent::Closed(reason));
    });
    session
}

/// The terminal as seen by the SSH task: prints text and reads answers.
struct Terminal {
    input: UnboundedReceiver<Input>,
    output: UnboundedSender<SessionEvent>,
    size: (u16, u16),
}

impl Terminal {
    fn print(&self, text: &str) {
        let _ = self.output.unbounded_send(SessionEvent::Output(text.replace('\n', "\r\n").into_bytes()));
    }

    /// Reads one line; `None` when the user pressed Ctrl-C/Ctrl-D or closed the tab.
    async fn read_line(&mut self, echo: bool) -> Option<String> {
        let mut line = String::new();
        loop {
            match self.input.next().await? {
                Input::Close => return None,
                Input::Resize(cols, rows) => self.size = (cols, rows),
                Input::Data(bytes) => {
                    for ch in String::from_utf8_lossy(&bytes).chars() {
                        match ch {
                            '\r' | '\n' => {
                                self.print("\n");
                                return Some(line);
                            }
                            '\u{3}' | '\u{4}' => return None,
                            '\u{7f}' | '\u{8}' => {
                                if line.pop().is_some() && echo {
                                    self.print("\u{8} \u{8}");
                                }
                            }
                            c if !c.is_control() => {
                                line.push(c);
                                if echo {
                                    self.print(&c.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    async fn ask(&mut self, prompt: &str, echo: bool) -> Option<String> {
        self.print(prompt);
        self.read_line(echo).await
    }
}

struct Verifier {
    host: String,
    port: u16,
    known_hosts: PathBuf,
    io: Arc<Mutex<Terminal>>,
}

impl client::Handler for Verifier {
    type Error = russh::Error;

    async fn check_server_key(&mut self, server_key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let key: PublicKey = match server_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(certificate) => certificate.public_key().clone().into(),
        };
        let fingerprint = key.fingerprint(HashAlg::Sha256);
        let mut io = self.io.lock().await;
        match check_known_hosts_path(&self.host, self.port, &key, &self.known_hosts) {
            Ok(true) => return Ok(true),
            Err(russh::keys::Error::KeyChanged { line }) => {
                io.print(&format!(
                    "WARNING: the host key for {} has changed (known_hosts line {line}).\n\
                     Offending key fingerprint: {fingerprint}\nConnection refused to protect you from \
                     a possible man-in-the-middle attack.\n",
                    self.host
                ));
                return Ok(false);
            }
            _ => {}
        }
        let question = format!(
            "The authenticity of host '{}' can't be established.\n{} key fingerprint is {fingerprint}.\n\
             Are you sure you want to continue connecting (yes/no)? ",
            self.host,
            key.algorithm().as_str()
        );
        loop {
            match io.ask(&question, true).await.as_deref().map(str::trim) {
                Some("yes") => break,
                Some("no") | None => return Ok(false),
                _ => continue,
            }
        }
        if let Err(error) = learn_known_hosts_path(&self.host, self.port, &key, &self.known_hosts) {
            io.print(&format!("Could not save the host key: {error}\n"));
        } else {
            io.print(&format!("Permanently added '{}' to the list of known hosts.\n", self.host));
        }
        Ok(true)
    }
}

async fn run_ssh(target: SshTarget, io: Arc<Mutex<Terminal>>) -> Result<(), String> {
    let address = format!("{}:{}", target.host, target.port);
    io.lock().await.print(&format!("Connecting to {address}...\n"));
    let stream = tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::TcpStream::connect((target.host.as_str(), target.port)))
        .await
        .map_err(|_| format!("ssh: connect to host {address}: timed out"))?
        .map_err(|error| format!("ssh: connect to host {address}: {error}"))?;
    let _ = stream.set_nodelay(true);

    let config = client::Config { keepalive_interval: Some(Duration::from_secs(30)), keepalive_max: 3, ..Default::default() };
    let verifier =
        Verifier { host: target.host.clone(), port: target.port, known_hosts: target.known_hosts.clone(), io: io.clone() };
    let mut handle = client::connect_stream(Arc::new(config), stream, verifier).await.map_err(|error| match error {
        russh::Error::UnknownKey => "Host key verification failed.".to_string(),
        other => format!("SSH handshake failed: {other}"),
    })?;

    let username = match target.username.clone().filter(|u| !u.is_empty()) {
        Some(user) => user,
        None => io.lock().await.ask("login as: ", true).await.ok_or("Authentication cancelled.")?,
    };
    authenticate(&mut handle, &username, &target, &io).await?;

    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    let (cols, rows) = io.lock().await.size;
    channel.request_pty(true, "xterm-256color", cols as u32, rows as u32, 0, 0, &[]).await.map_err(|e| e.to_string())?;
    channel.request_shell(true).await.map_err(|e| e.to_string())?;
    pump(channel, io).await
}

async fn authenticate(
    handle: &mut Handle<Verifier>,
    user: &str,
    target: &SshTarget,
    io: &Arc<Mutex<Terminal>>,
) -> Result<(), String> {
    let failed = |e: russh::Error| e.to_string();
    if handle.authenticate_none(user).await.map_err(failed)?.success() {
        return Ok(());
    }
    for path in target.identity_files.iter().filter(|p| p.is_file()) {
        let key = match load_secret_key(path, None) {
            Ok(key) => key,
            Err(_) => {
                let prompt = format!("Enter passphrase for key '{}': ", path.display());
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
            return Ok(());
        }
    }
    for _ in 0..PASSWORD_ATTEMPTS {
        let prompt = format!("{user}@{}'s password: ", target.host);
        let password = io.lock().await.ask(&prompt, false).await.ok_or("Authentication cancelled.")?;
        if handle.authenticate_password(user, &password).await.map_err(failed)?.success() {
            return Ok(());
        }
        if keyboard_interactive(handle, user, io).await? {
            return Ok(());
        }
        io.lock().await.print("Permission denied, please try again.\n");
    }
    Err(format!("{user}@{}: Permission denied.", target.host))
}

async fn keyboard_interactive(handle: &mut Handle<Verifier>, user: &str, io: &Arc<Mutex<Terminal>>) -> Result<bool, String> {
    let mut response =
        handle.authenticate_keyboard_interactive_start(user, None).await.map_err(|e| e.to_string())?;
    loop {
        match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { instructions, prompts, .. } => {
                let mut io = io.lock().await;
                if !instructions.is_empty() {
                    io.print(&format!("{instructions}\n"));
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
async fn pump(channel: russh::Channel<client::Msg>, io: Arc<Mutex<Terminal>>) -> Result<(), String> {
    let (mut reader, writer) = channel.split();
    let mut io = io.lock().await;
    loop {
        tokio::select! {
            message = reader.wait() => match message {
                Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => {
                    let _ = io.output.unbounded_send(SessionEvent::Output(data.to_vec()));
                }
                Some(ChannelMsg::ExitStatus { exit_status }) if exit_status != 0 => {
                    return Err(format!("Remote shell exited with code {exit_status}"));
                }
                Some(ChannelMsg::Close | ChannelMsg::Eof) | None => return Ok(()),
                _ => {}
            },
            input = io.input.next() => match input {
                Some(Input::Data(bytes)) => writer.data_bytes(bytes).await.map_err(|e| e.to_string())?,
                Some(Input::Resize(cols, rows)) => {
                    io.size = (cols, rows);
                    let _ = writer.window_change(cols as u32, rows as u32, 0, 0).await;
                }
                Some(Input::Close) | None => {
                    let _ = writer.close().await;
                    return Ok(());
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
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

    mod server {
        use super::*;
        use russh::keys::ssh_key::private::Ed25519Keypair;
        use russh::keys::PrivateKey;
        use russh::server::{self, Auth, Msg, Session as ServerSession};
        use russh::{Channel, ChannelId};

        pub const PASSWORD: &str = "s3cret";

        pub fn host_key() -> PrivateKey {
            PrivateKey::from(Ed25519Keypair::from_seed(&[7; 32]))
        }

        /// Accepts `PASSWORD` and echoes shell input back with a prefix.
        #[derive(Clone)]
        struct Echo;

        impl server::Handler for Echo {
            type Error = russh::Error;

            async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
                Ok(if user == "root" && password == PASSWORD { Auth::Accept } else { Auth::reject() })
            }

            async fn channel_open_session(
                &mut self,
                _: Channel<Msg>,
                reply: server::ChannelOpenHandle,
                _: &mut ServerSession,
            ) -> Result<(), Self::Error> {
                reply.accept().await;
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
                session.data(channel, [b"echo:".as_slice(), data].concat())
            }
        }

        /// Starts a one-connection server on a free port.
        pub fn start() -> u16 {
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
                    let running = server::run_stream(config, socket, Echo).await.unwrap();
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

        let output = collect_until(&mut session, "(yes/no)? ");
        assert!(output.contains("SHA256:"), "{output:?}");
        session.write(b"yes\r");
        let output = collect_until(&mut session, "password: ");
        assert!(output.contains("root@127.0.0.1's password: "), "{output:?}");
        session.write(server::PASSWORD.as_bytes());
        session.write(b"\r");
        let output = collect_until(&mut session, "welcome");
        assert!(output.contains("pty xterm-256color 100x30"), "{output:?}");
        assert!(!output.contains(server::PASSWORD), "password must not be echoed");

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
        assert!(output.contains("has changed"), "{output:?}");
        assert!(output.contains("Host key verification failed"), "{output:?}");
    }

    #[test]
    fn declining_the_host_key_closes_the_session() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = ssh(target(server::start(), dir.path().join("known_hosts")), 80, 24);
        collect_until(&mut session, "(yes/no)? ");
        session.write(b"no\r");
        assert!(collect_until(&mut session, "<closed").contains("Host key verification failed"));
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
    fn parses_host_with_port() {
        let t = SshTarget::new("example.com:2001", None);
        assert_eq!((t.host.as_str(), t.port), ("example.com", 2001));
        assert_eq!(SshTarget::new("example.com", None).port, 22);
    }
}
