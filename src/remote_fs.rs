//! Remote side of the SFTP tab. Runs on its own thread; the UI sends
//! commands and receives events. Login questions (host key, password) arrive
//! as `Login` events and are answered with [`RemoteFs::answer`].

use std::sync::Arc;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use russh_sftp::client::SftpSession;
use tokio::sync::Mutex;

use crate::files::{join, Entry};
use crate::login::{Answer, LoginEvent, Stage};
use crate::session::{connect, EventPrompter, Prompter, SshTarget};

#[derive(Debug, PartialEq)]
pub enum RemoteEvent {
    Login(LoginEvent),
    Connected { home: String },
    Listed { path: String, result: Result<Vec<Entry>, String> },
    Closed(Option<String>),
}

enum Command {
    Answer(Answer),
    List(String),
}

/// A connection to one host. Dropping it disconnects.
pub struct RemoteFs {
    commands: UnboundedSender<Command>,
    events: Option<UnboundedReceiver<RemoteEvent>>,
}

impl RemoteFs {
    pub fn connect(target: SshTarget) -> Self {
        let (commands, command_rx) = unbounded();
        let (events_tx, events) = unbounded();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = events_tx.unbounded_send(RemoteEvent::Closed(Some(error.to_string())));
                    return;
                }
            };
            let result = runtime.block_on(run(target, command_rx, events_tx.clone()));
            let _ = events_tx.unbounded_send(RemoteEvent::Closed(result.err()));
        });
        Self { commands, events: Some(events) }
    }

    pub fn take_events(&mut self) -> Option<UnboundedReceiver<RemoteEvent>> {
        self.events.take()
    }

    /// Answers the pending login question.
    pub fn answer(&self, answer: Answer) {
        let _ = self.commands.unbounded_send(Command::Answer(answer));
    }

    pub fn list(&self, path: &str) {
        let _ = self.commands.unbounded_send(Command::List(path.to_owned()));
    }
}

async fn run(
    target: SshTarget,
    mut commands: UnboundedReceiver<Command>,
    events: UnboundedSender<RemoteEvent>,
) -> Result<(), String> {
    // Answers go to the prompter while logging in; listings queue up meanwhile.
    let (answer_tx, answers) = unbounded();
    let (list_tx, mut lists) = unbounded::<String>();
    tokio::spawn(async move {
        while let Some(command) = commands.next().await {
            let sent = match command {
                Command::Answer(answer) => answer_tx.unbounded_send(answer).is_ok(),
                Command::List(path) => list_tx.unbounded_send(path).is_ok(),
            };
            if !sent {
                break;
            }
        }
    });

    let emit_events = events.clone();
    let prompter = Arc::new(Mutex::new(EventPrompter {
        emit: move |event| {
            let _ = emit_events.unbounded_send(RemoteEvent::Login(event));
        },
        answers,
    }));
    let handle = connect(&target, prompter.clone()).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
    let sftp = SftpSession::new(channel.into_stream()).await.map_err(|e| format!("SFTP is not available: {e}"))?;
    let home = sftp.canonicalize(".").await.unwrap_or_else(|_| "/".into());
    prompter.lock().await.event(LoginEvent::Stage(Stage::Ready));
    let _ = events.unbounded_send(RemoteEvent::Connected { home });

    while let Some(path) = lists.next().await {
        let result = list(&sftp, &path).await;
        if events.unbounded_send(RemoteEvent::Listed { path, result }).is_err() {
            break;
        }
    }
    Ok(())
}

async fn list(sftp: &SftpSession, path: &str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for item in sftp.read_dir(path).await.map_err(|e| format!("{path}: {e}"))? {
        let meta = item.metadata();
        let name = item.file_name();
        let is_link = meta.is_symlink();
        // Links count as folders when they point at one, so they can be opened.
        let is_dir = if is_link {
            sftp.metadata(join(path, &name)).await.map(|target| target.is_dir()).unwrap_or(false)
        } else {
            meta.is_dir()
        };
        entries.push(Entry {
            name,
            is_dir,
            is_link,
            size: meta.size.unwrap_or(0),
            modified: meta.mtime.map(i64::from),
            mode: meta.permissions.map(|p| p & 0o777),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::HostKeyDecision;
    use crate::session::tests::server;
    use std::time::Duration;

    fn next(events: &mut UnboundedReceiver<RemoteEvent>) -> RemoteEvent {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut taken = std::mem::replace(events, unbounded().1);
        std::thread::spawn(move || {
            let event = futures::executor::block_on(taken.next());
            let _ = tx.send((event, taken));
        });
        let (event, back) = rx.recv_timeout(Duration::from_secs(10)).expect("no event within 10 s");
        *events = back;
        event.expect("event stream ended")
    }

    #[test]
    fn logs_in_through_prompts_and_lists_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("etc")).unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"12345").unwrap();
        let port = server::start_with_sftp(dir.path().to_path_buf());
        let state = tempfile::tempdir().unwrap();
        let target = SshTarget {
            host: "127.0.0.1".into(),
            port,
            username: Some("root".into()),
            known_hosts: state.path().join("known_hosts"),
            identity_files: vec![],
        };
        let mut remote = RemoteFs::connect(target);
        let mut events = remote.take_events().unwrap();

        let info = loop {
            match next(&mut events) {
                RemoteEvent::Login(LoginEvent::HostKey(info)) => break info,
                RemoteEvent::Login(_) => continue,
                other => panic!("expected host key question, got {other:?}"),
            }
        };
        assert_eq!((info.host.as_str(), info.algorithm.as_str()), ("127.0.0.1", "ED25519"));
        remote.answer(Answer::HostKey(HostKeyDecision::Save));
        let (text, echo) = loop {
            match next(&mut events) {
                RemoteEvent::Login(LoginEvent::Prompt { text, echo }) => break (text, echo),
                RemoteEvent::Login(_) => continue,
                other => panic!("expected password prompt, got {other:?}"),
            }
        };
        assert_eq!((text.as_str(), echo), ("root@127.0.0.1's password", false));
        remote.answer(Answer::Text(Some(server::PASSWORD.into())));
        let event = loop {
            match next(&mut events) {
                RemoteEvent::Login(_) => continue,
                other => break other,
            }
        };

        let RemoteEvent::Connected { home } = event else { panic!("expected connection, got {event:?}") };
        assert_eq!(home, "/");
        remote.list("/");
        let RemoteEvent::Listed { path, result } = next(&mut events) else { panic!("expected listing") };
        assert_eq!(path, "/");
        let mut entries = result.unwrap();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(entries.iter().map(|e| (e.name.as_str(), e.is_dir)).collect::<Vec<_>>(), [("etc", true), ("notes.txt", false)]);
        assert_eq!(entries[1].size, 5);
        assert!(entries[1].modified.is_some() && entries[1].mode.is_some());
    }

    #[test]
    fn cancelling_the_login_closes() {
        let dir = tempfile::tempdir().unwrap();
        let port = server::start_with_sftp(dir.path().to_path_buf());
        let target = SshTarget {
            host: "127.0.0.1".into(),
            port,
            username: Some("root".into()),
            known_hosts: dir.path().join("known_hosts"),
            identity_files: vec![],
        };
        let mut remote = RemoteFs::connect(target);
        let mut events = remote.take_events().unwrap();
        while !matches!(next(&mut events), RemoteEvent::Login(LoginEvent::HostKey(_))) {}
        remote.answer(Answer::HostKey(HostKeyDecision::Reject));
        loop {
            match next(&mut events) {
                RemoteEvent::Closed(reason) => {
                    assert!(reason.unwrap().contains("Host key verification failed"));
                    break;
                }
                RemoteEvent::Login(_) => continue,
                other => panic!("unexpected {other:?}"),
            }
        }
    }
}
