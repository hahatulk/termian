//! Remote side of the SFTP tab. Runs on its own thread; the UI sends
//! commands and receives events. Login questions (host key, password) arrive
//! as `Prompt` events and are answered with [`RemoteFs::answer`].

use std::sync::Arc;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::future::BoxFuture;
use futures::StreamExt;
use russh_sftp::client::SftpSession;
use tokio::sync::Mutex;

use crate::files::{join, Entry};
use crate::session::{connect, Prompter, SshTarget};

#[derive(Debug, PartialEq)]
pub enum RemoteEvent {
    /// Text to show before the next prompt (host key fingerprint, warnings).
    Message(String),
    Prompt { text: String, echo: bool },
    Connected { home: String },
    Listed { path: String, result: Result<Vec<Entry>, String> },
    Closed(Option<String>),
}

enum Command {
    Answer(Option<String>),
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

    /// Answers the pending prompt; `None` cancels the login.
    pub fn answer(&self, answer: Option<String>) {
        let _ = self.commands.unbounded_send(Command::Answer(answer));
    }

    pub fn list(&self, path: &str) {
        let _ = self.commands.unbounded_send(Command::List(path.to_owned()));
    }
}

struct FormPrompter {
    events: UnboundedSender<RemoteEvent>,
    answers: UnboundedReceiver<Option<String>>,
}

impl Prompter for FormPrompter {
    fn print(&mut self, text: &str) {
        let text = text.trim();
        if !text.is_empty() && !text.starts_with("Connecting to ") {
            let _ = self.events.unbounded_send(RemoteEvent::Message(text.to_owned()));
        }
    }

    fn ask<'a>(&'a mut self, prompt: &'a str, echo: bool) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let _ = self.events.unbounded_send(RemoteEvent::Prompt { text: prompt.trim().to_owned(), echo });
            self.answers.next().await.flatten()
        })
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

    let prompter = Arc::new(Mutex::new(FormPrompter { events: events.clone(), answers }));
    let handle = connect(&target, prompter).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
    let sftp = SftpSession::new(channel.into_stream()).await.map_err(|e| format!("SFTP is not available: {e}"))?;
    let home = sftp.canonicalize(".").await.unwrap_or_else(|_| "/".into());
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

        let RemoteEvent::Prompt { text, echo } = next(&mut events) else { panic!("expected host key prompt") };
        assert!(text.contains("SHA256:") && text.ends_with("(yes/no)?") && echo, "{text}");
        remote.answer(Some("yes".into()));
        assert!(matches!(next(&mut events), RemoteEvent::Message(m) if m.contains("Permanently added")));
        let RemoteEvent::Prompt { text, echo } = next(&mut events) else { panic!("expected password prompt") };
        assert_eq!((text.as_str(), echo), ("root@127.0.0.1's password:", false));
        remote.answer(Some(server::PASSWORD.into()));

        let event = next(&mut events);
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
        assert!(matches!(next(&mut events), RemoteEvent::Prompt { .. }));
        remote.answer(None);
        loop {
            match next(&mut events) {
                RemoteEvent::Closed(reason) => {
                    assert!(reason.unwrap().contains("Host key verification failed"));
                    break;
                }
                RemoteEvent::Message(_) => continue,
                other => panic!("unexpected {other:?}"),
            }
        }
    }
}
