//! Running a port forwarding rule: log in to its host (questions go to the
//! UI as [`LoginEvent`]s), then
//! - Local: listen on bind:port and send each connection to destination:port through SSH;
//! - Dynamic: listen on bind:port as a SOCKS5 proxy;
//! - Remote: ask the server to listen on bind:port and send its connections to destination:port here.
//!
//! Dropping the [`Forwarding`] stops it.

use std::sync::Arc;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use russh::client::Handle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::login::{Answer, LoginEvent, Stage};
use crate::session::{connect_forwarding, EventPrompter, Prompter, SshTarget, Verifier};
use crate::vault::RuleKind;

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub kind: RuleKind,
    pub bind_address: String,
    pub port: u16,
    pub destination: String,
    pub destination_port: u16,
}

#[derive(Debug, PartialEq)]
pub enum ForwardEvent {
    Login(LoginEvent),
    /// Listening; for a local port 0, the port that was picked.
    Active { port: u16 },
    Closed(Option<String>),
}

pub struct Forwarding {
    answers: UnboundedSender<Answer>,
    events: Option<UnboundedReceiver<ForwardEvent>>,
    _stop: oneshot::Sender<()>,
}

impl Forwarding {
    pub fn start(target: SshTarget, spec: Spec) -> Self {
        let (answers, answer_rx) = unbounded();
        let (events_tx, events) = unbounded();
        let (stop, stopped) = oneshot::channel::<()>();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = events_tx.unbounded_send(ForwardEvent::Closed(Some(error.to_string())));
                    return;
                }
            };
            let result = runtime.block_on(async {
                tokio::select! {
                    result = run(target, spec, answer_rx, events_tx.clone()) => result,
                    _ = stopped => Ok(()),
                }
            });
            let _ = events_tx.unbounded_send(ForwardEvent::Closed(result.err()));
        });
        Self { answers, events: Some(events), _stop: stop }
    }

    pub fn take_events(&mut self) -> Option<UnboundedReceiver<ForwardEvent>> {
        self.events.take()
    }

    pub fn answer(&self, answer: Answer) {
        let _ = self.answers.unbounded_send(answer);
    }
}

async fn run(target: SshTarget, spec: Spec, answers: UnboundedReceiver<Answer>, events: UnboundedSender<ForwardEvent>) -> Result<(), String> {
    let emit = events.clone();
    let prompter = Arc::new(Mutex::new(EventPrompter {
        emit: move |event| {
            let _ = emit.unbounded_send(ForwardEvent::Login(event));
        },
        answers,
    }));
    let forward_to = (spec.kind == RuleKind::Remote).then(|| (spec.destination.clone(), spec.destination_port));
    let handle = Arc::new(connect_forwarding(&target, prompter.clone(), forward_to).await?);
    prompter.lock().await.event(LoginEvent::Stage(Stage::Ready));

    match spec.kind {
        RuleKind::Remote => {
            let port = handle
                .tcpip_forward(spec.bind_address.clone(), u32::from(spec.port))
                .await
                .map_err(|e| format!("The server refused to listen on {}:{}: {e}", spec.bind_address, spec.port))?;
            let _ = events.unbounded_send(ForwardEvent::Active { port: if spec.port == 0 { port as u16 } else { spec.port } });
            // Connections arrive through the handler; stay until stopped or disconnected.
            while !handle.is_closed() {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err("The connection was closed.".into())
        }
        RuleKind::Local | RuleKind::Dynamic => {
            let listener = TcpListener::bind((spec.bind_address.as_str(), spec.port))
                .await
                .map_err(|e| format!("Cannot listen on {}:{}: {e}", spec.bind_address, spec.port))?;
            let port = listener.local_addr().map(|a| a.port()).unwrap_or(spec.port);
            let _ = events.unbounded_send(ForwardEvent::Active { port });
            loop {
                let (socket, peer) = listener.accept().await.map_err(|e| e.to_string())?;
                if handle.is_closed() {
                    return Err("The connection was closed.".into());
                }
                let (handle, spec) = (handle.clone(), spec.clone());
                tokio::spawn(async move {
                    let origin = (peer.ip().to_string(), u32::from(peer.port()));
                    let _ = match spec.kind {
                        RuleKind::Dynamic => socks(socket, &handle, origin).await,
                        _ => tunnel(socket, &handle, &spec.destination, spec.destination_port, origin).await,
                    };
                });
            }
        }
    }
}

async fn tunnel(mut socket: TcpStream, handle: &Handle<Verifier>, host: &str, port: u16, origin: (String, u32)) -> Result<(), String> {
    let channel = handle.channel_open_direct_tcpip(host, u32::from(port), origin.0, origin.1).await.map_err(|e| e.to_string())?;
    let mut stream = channel.into_stream();
    tokio::io::copy_bidirectional(&mut socket, &mut stream).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// A SOCKS5 CONNECT without authentication, then a tunnel to the asked address.
async fn socks(mut socket: TcpStream, handle: &Handle<Verifier>, origin: (String, u32)) -> Result<(), String> {
    let failed = |e: std::io::Error| e.to_string();
    let mut head = [0u8; 2];
    socket.read_exact(&mut head).await.map_err(failed)?;
    if head[0] != 5 {
        return Err("only SOCKS5 is supported".into());
    }
    let mut methods = vec![0u8; usize::from(head[1])];
    socket.read_exact(&mut methods).await.map_err(failed)?;
    socket.write_all(&[5, 0]).await.map_err(failed)?;

    let mut request = [0u8; 4];
    socket.read_exact(&mut request).await.map_err(failed)?;
    let host = match request[3] {
        1 => {
            let mut ip = [0u8; 4];
            socket.read_exact(&mut ip).await.map_err(failed)?;
            std::net::Ipv4Addr::from(ip).to_string()
        }
        3 => {
            let mut len = [0u8; 1];
            socket.read_exact(&mut len).await.map_err(failed)?;
            let mut name = vec![0u8; usize::from(len[0])];
            socket.read_exact(&mut name).await.map_err(failed)?;
            String::from_utf8_lossy(&name).into_owned()
        }
        4 => {
            let mut ip = [0u8; 16];
            socket.read_exact(&mut ip).await.map_err(failed)?;
            std::net::Ipv6Addr::from(ip).to_string()
        }
        _ => return Err("unsupported SOCKS address".into()),
    };
    let mut port = [0u8; 2];
    socket.read_exact(&mut port).await.map_err(failed)?;
    let port = u16::from_be_bytes(port);
    // Only CONNECT; anything else is "command not supported".
    if request[1] != 1 {
        let _ = socket.write_all(&[5, 7, 0, 1, 0, 0, 0, 0, 0, 0]).await;
        return Err("only SOCKS CONNECT is supported".into());
    }
    match handle.channel_open_direct_tcpip(host, u32::from(port), origin.0, origin.1).await {
        Ok(channel) => {
            socket.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await.map_err(failed)?;
            let mut stream = channel.into_stream();
            tokio::io::copy_bidirectional(&mut socket, &mut stream).await.map_err(failed)?;
            Ok(())
        }
        Err(error) => {
            let _ = socket.write_all(&[5, 5, 0, 1, 0, 0, 0, 0, 0, 0]).await;
            Err(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::HostKeyDecision;
    use crate::session::tests::server;
    use futures::StreamExt;
    use std::time::Duration;

    /// A TCP server that answers each line with `pong:` + the line.
    fn echo_server() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || {
                    use std::io::{BufRead, Write};
                    let mut writer = stream.try_clone().unwrap();
                    for line in std::io::BufReader::new(stream).lines().map_while(Result::ok) {
                        let _ = writeln!(writer, "pong:{line}");
                    }
                });
            }
        });
        port
    }

    fn target(port: u16, dir: &std::path::Path) -> SshTarget {
        let mut target = SshTarget::new(&format!("127.0.0.1:{port}"), Some("root".into()));
        target.known_hosts = dir.join("known_hosts");
        target.identity_files.clear();
        target.password = Some(server::PASSWORD.into());
        target
    }

    /// Answers the host key question and returns the listening port.
    fn wait_active(forwarding: &mut Forwarding) -> u16 {
        let mut events = forwarding.take_events().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let answers = forwarding.answers.clone();
        std::thread::spawn(move || {
            futures::executor::block_on(async {
                while let Some(event) = events.next().await {
                    match event {
                        ForwardEvent::Login(LoginEvent::HostKey(_)) => {
                            let _ = answers.unbounded_send(Answer::HostKey(HostKeyDecision::Once));
                        }
                        ForwardEvent::Active { port } => {
                            let _ = tx.send(Ok(port));
                        }
                        ForwardEvent::Closed(reason) => {
                            let _ = tx.send(Err(reason));
                        }
                        ForwardEvent::Login(_) => {}
                    }
                }
            });
        });
        rx.recv_timeout(Duration::from_secs(10)).expect("no answer").expect("forwarding failed")
    }

    fn ask(port: u16, line: &str) -> String {
        use std::io::{BufRead, Write};
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        writeln!(stream, "{line}").unwrap();
        let mut reply = String::new();
        std::io::BufReader::new(stream).read_line(&mut reply).unwrap();
        reply.trim().to_owned()
    }

    #[test]
    fn local_forwarding_reaches_the_destination_through_ssh() {
        let dir = tempfile::tempdir().unwrap();
        let destination = echo_server();
        let spec = Spec { kind: RuleKind::Local, bind_address: "127.0.0.1".into(), port: 0, destination: "127.0.0.1".into(), destination_port: destination };
        let mut forwarding = Forwarding::start(target(server::start(), dir.path()), spec);
        let port = wait_active(&mut forwarding);
        assert_eq!(ask(port, "hi"), "pong:hi");
        assert_eq!(ask(port, "again"), "pong:again", "every connection gets its own channel");
    }

    #[test]
    fn dynamic_forwarding_is_a_socks5_proxy() {
        use std::io::{BufRead, Read, Write};
        let dir = tempfile::tempdir().unwrap();
        let destination = echo_server();
        let spec = Spec { kind: RuleKind::Dynamic, bind_address: "127.0.0.1".into(), port: 0, destination: String::new(), destination_port: 0 };
        let mut forwarding = Forwarding::start(target(server::start(), dir.path()), spec);
        let port = wait_active(&mut forwarding);

        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        stream.write_all(&[5, 1, 0]).unwrap();
        let mut greeting = [0u8; 2];
        stream.read_exact(&mut greeting).unwrap();
        assert_eq!(greeting, [5, 0]);
        let name = b"localhost";
        let mut request = vec![5, 1, 0, 3, name.len() as u8];
        request.extend_from_slice(name);
        request.extend_from_slice(&destination.to_be_bytes());
        stream.write_all(&request).unwrap();
        let mut reply = [0u8; 10];
        stream.read_exact(&mut reply).unwrap();
        assert_eq!(reply[1], 0, "SOCKS success");
        writeln!(stream, "via socks").unwrap();
        let mut line = String::new();
        std::io::BufReader::new(stream).read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "pong:via socks");
    }

    #[test]
    fn remote_forwarding_brings_server_connections_here() {
        let dir = tempfile::tempdir().unwrap();
        let destination = echo_server();
        let spec = Spec { kind: RuleKind::Remote, bind_address: "127.0.0.1".into(), port: 0, destination: "127.0.0.1".into(), destination_port: destination };
        let mut forwarding = Forwarding::start(target(server::start(), dir.path()), spec);
        let port = wait_active(&mut forwarding);
        // The test server listens on the port it picked, on this machine.
        assert_eq!(ask(port, "from server"), "pong:from server");
    }

    #[test]
    fn a_busy_port_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let busy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let spec = Spec { kind: RuleKind::Local, bind_address: "127.0.0.1".into(), port: busy.local_addr().unwrap().port(), destination: "x".into(), destination_port: 1 };
        let mut forwarding = Forwarding::start(target(server::start(), dir.path()), spec);
        let mut events = forwarding.take_events().unwrap();
        let answers = forwarding.answers.clone();
        let reason = futures::executor::block_on(async {
            while let Some(event) = events.next().await {
                match event {
                    ForwardEvent::Login(LoginEvent::HostKey(_)) => {
                        let _ = answers.unbounded_send(Answer::HostKey(HostKeyDecision::Once));
                    }
                    ForwardEvent::Closed(reason) => return reason,
                    _ => {}
                }
            }
            None
        });
        assert!(reason.unwrap().starts_with("Cannot listen on 127.0.0.1:"));
    }
}
