//! Remote side of the SFTP tab. Runs on its own thread; the UI sends
//! commands and receives events. Login questions (host key, password) arrive
//! as `Login` events and are answered with [`RemoteFs::answer`]. Transfers
//! and file operations run concurrently with listings.

use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use russh_sftp::client::{RawSftpSession, SftpSession};
use tokio::sync::Mutex;

use crate::files::{join, parse_owner, Entry};
use crate::login::{Answer, LoginEvent, Stage};
use crate::session::{connect, EventPrompter, Prompter, SshTarget};
use crate::transfer::{self, Control, Direction, Fs, Job, TransferEvent};

pub type TransferId = u64;

#[derive(Debug, PartialEq)]
pub enum RemoteEvent {
    Login(LoginEvent),
    Connected { home: String },
    Listed { path: String, result: Result<Vec<Entry>, String> },
    Transfer { id: TransferId, event: TransferEvent },
    /// A rename, delete or new folder finished.
    Changed(Result<(), String>),
    /// Fresh names in a folder, asked for with [`RemoteFs::names`].
    Names { request: u64, result: Result<Vec<String>, String> },
    Closed(Option<String>),
}

/// File operations on the remote side.
#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Rename { from: String, to: String },
    Remove { path: String, is_dir: bool },
    MakeDir(String),
    /// Edit Permissions: the new permission bits.
    Chmod { path: String, mode: u32 },
}

enum Work {
    List(String),
    Names { request: u64, path: String },
    Transfer { id: TransferId, job: Job, controls: UnboundedReceiver<Control> },
    Operate(Vec<Operation>),
}

enum Command {
    Answer(Answer),
    Work(Work),
}

/// A connection to one host. Dropping it disconnects and stops its transfers.
pub struct RemoteFs {
    commands: UnboundedSender<Command>,
    events: Option<UnboundedReceiver<RemoteEvent>>,
    controls: HashMap<TransferId, UnboundedSender<Control>>,
    next_transfer: TransferId,
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
        Self { commands, events: Some(events), controls: HashMap::new(), next_transfer: 0 }
    }

    pub fn take_events(&mut self) -> Option<UnboundedReceiver<RemoteEvent>> {
        self.events.take()
    }

    /// Answers the pending login question.
    pub fn answer(&self, answer: Answer) {
        let _ = self.commands.unbounded_send(Command::Answer(answer));
    }

    pub fn list(&self, path: &str) {
        let _ = self.commands.unbounded_send(Command::Work(Work::List(path.to_owned())));
    }

    /// Reads the names in a folder again, e.g. right before a copy into it.
    pub fn names(&self, request: u64, path: &str) {
        let _ = self.commands.unbounded_send(Command::Work(Work::Names { request, path: path.to_owned() }));
    }

    /// Starts copying between the local disk and this host.
    pub fn transfer(&mut self, job: Job) -> TransferId {
        self.next_transfer += 1;
        let id = self.next_transfer;
        let (control, controls) = unbounded();
        self.controls.insert(id, control);
        let _ = self.commands.unbounded_send(Command::Work(Work::Transfer { id, job, controls }));
        id
    }

    /// Answers a failed transfer, or discards it with [`Control::Cancel`].
    pub fn control(&mut self, id: TransferId, control: Control) {
        if let Some(sender) = self.controls.get(&id) {
            let _ = sender.unbounded_send(control);
        }
    }

    /// Forgets a transfer that ended.
    pub fn finished(&mut self, id: TransferId) {
        self.controls.remove(&id);
    }

    /// Runs the operations in order; one `Changed` event reports the outcome.
    pub fn operate(&self, operations: Vec<Operation>) {
        let _ = self.commands.unbounded_send(Command::Work(Work::Operate(operations)));
    }
}

async fn run(
    target: SshTarget,
    mut commands: UnboundedReceiver<Command>,
    events: UnboundedSender<RemoteEvent>,
) -> Result<(), String> {
    // Answers go to the prompter while logging in; other work queues up meanwhile.
    let (answer_tx, answers) = unbounded();
    let (work_tx, mut work) = unbounded::<Work>();
    tokio::spawn(async move {
        while let Some(command) = commands.next().await {
            let sent = match command {
                Command::Answer(answer) => answer_tx.unbounded_send(answer).is_ok(),
                Command::Work(item) => work_tx.unbounded_send(item).is_ok(),
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
        picked: None,
    }));
    let handle = connect(&target, prompter.clone()).await?;
    let channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.request_subsystem(true, "sftp").await.map_err(|e| e.to_string())?;
    let sftp = SftpSession::new(channel.into_stream()).await.map_err(|e| format!("SFTP is not available: {e}"))?;
    let sftp = Arc::new(sftp);
    // A second, raw session reads directory long names, the only place SFTP
    // v3 names the owner and group. Listings work without it.
    let raw = raw_session(&handle).await;
    let home = sftp.canonicalize(".").await.unwrap_or_else(|_| "/".into());
    prompter.lock().await.event(LoginEvent::Stage(Stage::Ready));
    let _ = events.unbounded_send(RemoteEvent::Connected { home });

    while let Some(item) = work.next().await {
        match item {
            Work::List(path) => {
                let mut result = list(&sftp, &path).await;
                if let (Ok(entries), Some(raw)) = (&mut result, &raw) {
                    add_owners(raw, &path, entries).await;
                }
                if events.unbounded_send(RemoteEvent::Listed { path, result }).is_err() {
                    break;
                }
            }
            Work::Names { request, path } => {
                let result = sftp.read_dir(&path).await.map(|dir| dir.map(|e| e.file_name()).collect()).map_err(|e| format!("{path}: {e}"));
                if events.unbounded_send(RemoteEvent::Names { request, result }).is_err() {
                    break;
                }
            }
            Work::Transfer { id, job, controls } => {
                let remote = Fs::Remote(sftp.clone());
                let (source, target) = match job.direction {
                    Direction::Upload => (Fs::Local, remote),
                    Direction::Download => (remote, Fs::Local),
                };
                let events = events.clone();
                tokio::spawn(transfer::transfer(source, target, job, controls, move |event| {
                    let _ = events.unbounded_send(RemoteEvent::Transfer { id, event });
                }));
            }
            Work::Operate(operations) => {
                let (fs, events) = (Fs::Remote(sftp.clone()), events.clone());
                tokio::spawn(async move {
                    let _ = events.unbounded_send(RemoteEvent::Changed(operate(&fs, operations).await));
                });
            }
        }
    }
    Ok(())
}

/// [`RemoteFs::operate`] for the local panel. Blocks: call it off the UI thread.
pub fn operate_locally(operations: Vec<Operation>) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
    runtime.block_on(operate(&Fs::Local, operations))
}

/// Runs file operations in order, stopping at the first failure.
pub(crate) async fn operate(fs: &Fs, operations: Vec<Operation>) -> Result<(), String> {
    for operation in operations {
        match operation {
            Operation::Rename { from, to } => fs.rename(&from, &to).await?,
            Operation::Remove { path, is_dir } => fs.remove(&path, is_dir).await?,
            Operation::MakeDir(path) => fs.make_dir(&path).await?,
            Operation::Chmod { path, mode } => fs.chmod(&path, mode).await?,
        }
    }
    Ok(())
}

async fn raw_session(handle: &russh::client::Handle<crate::session::Verifier>) -> Option<RawSftpSession> {
    let channel = handle.channel_open_session().await.ok()?;
    channel.request_subsystem(true, "sftp").await.ok()?;
    let raw = RawSftpSession::new(channel.into_stream());
    raw.init().await.ok()?;
    Some(raw)
}

/// Fills `owner` from the long names of a raw directory read.
async fn add_owners(raw: &RawSftpSession, path: &str, entries: &mut [Entry]) {
    let Ok(dir) = raw.opendir(path).await else { return };
    while let Ok(name) = raw.readdir(dir.handle.as_str()).await {
        for file in name.files {
            if let Some(entry) = entries.iter_mut().find(|e| e.name == file.filename) {
                entry.owner = parse_owner(&file.longname);
            }
        }
    }
    let _ = raw.close(dir.handle).await;
}

pub(crate) async fn list(sftp: &SftpSession, path: &str) -> Result<Vec<Entry>, String> {
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
            mode: meta.permissions.map(|p| p & 0o7777),
            owner: None,
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
            password: None,
            key: None,
            options: crate::session::SshOptions::default(),
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
        assert!(entries[1].owner.is_some(), "owner from the long name");
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
            password: None,
            key: None,
            options: crate::session::SshOptions::default(),
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
    use crate::transfer::{Item, Planned};

    /// Logs in to a fresh test server that serves `root`.
    fn connected(root: &std::path::Path, state: &std::path::Path) -> (RemoteFs, UnboundedReceiver<RemoteEvent>) {
        let port = server::start_with_sftp(root.to_path_buf());
        let target = SshTarget {
            host: "127.0.0.1".into(),
            port,
            username: Some("root".into()),
            known_hosts: state.join("known_hosts"),
            identity_files: vec![],
            password: None,
            key: None,
            options: crate::session::SshOptions::default(),
        };
        let mut remote = RemoteFs::connect(target);
        let mut events = remote.take_events().unwrap();
        loop {
            match next(&mut events) {
                RemoteEvent::Login(LoginEvent::HostKey(_)) => remote.answer(Answer::HostKey(HostKeyDecision::Once)),
                RemoteEvent::Login(LoginEvent::Prompt { .. }) => remote.answer(Answer::Text(Some(server::PASSWORD.into()))),
                RemoteEvent::Login(_) => {}
                RemoteEvent::Connected { .. } => return (remote, events),
                other => panic!("login failed: {other:?}"),
            }
        }
    }

    fn job(direction: Direction, source_dir: &str, destination_dir: &str, name: &str, is_dir: bool) -> Job {
        let item = Item { name: name.into(), is_dir };
        Job { direction, source_dir: source_dir.into(), destination_dir: destination_dir.into(), planned: Planned { target: name.into(), item, replace: false } }
    }

    /// Transfer events until it ends; failures are answered with `on_failure`.
    fn finish(remote: &mut RemoteFs, events: &mut UnboundedReceiver<RemoteEvent>, id: TransferId, on_failure: Control) -> Vec<TransferEvent> {
        let mut seen = Vec::new();
        loop {
            let RemoteEvent::Transfer { id: got, event } = next(events) else { continue };
            assert_eq!(got, id);
            if matches!(event, TransferEvent::Failed { .. }) {
                remote.control(id, on_failure);
            }
            let done = matches!(event, TransferEvent::Finished | TransferEvent::Cancelled);
            seen.push(event);
            if done {
                remote.finished(id);
                return seen;
            }
        }
    }

    fn last_progress(events: &[TransferEvent]) -> crate::transfer::Progress {
        events.iter().rev().find_map(|e| match e { TransferEvent::Progress(p) => Some(p.clone()), _ => None }).expect("no progress")
    }

    #[test]
    fn uploads_and_downloads_a_folder_tree() {
        let (served, local, state) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let project = local.path().join("project");
        std::fs::create_dir_all(project.join("src/deep")).unwrap();
        std::fs::write(project.join("README.md"), b"hello").unwrap();
        let big: Vec<u8> = (0..700_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(project.join("src/deep/data.bin"), &big).unwrap();
        std::fs::create_dir(project.join("empty")).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(project.join("README.md"), std::os::unix::fs::PermissionsExt::from_mode(0o751)).unwrap();
        let (mut remote, mut events) = connected(served.path(), state.path());

        let id = remote.transfer(job(Direction::Upload, local.path().to_str().unwrap(), "/", "project", true));
        let seen = finish(&mut remote, &mut events, id, Control::Cancel);
        assert_eq!(seen.last(), Some(&TransferEvent::Finished));
        let progress = last_progress(&seen);
        assert_eq!((progress.files_done, progress.files_total, progress.bytes_done, progress.bytes_total), (2, 2, 700_005, 700_005));
        let uploaded = served.path().join("project");
        assert_eq!(std::fs::read(uploaded.join("src/deep/data.bin")).unwrap(), big);
        assert_eq!(std::fs::read(uploaded.join("README.md")).unwrap(), b"hello");
        assert!(uploaded.join("empty").is_dir());
        #[cfg(unix)]
        assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(uploaded.join("README.md")).unwrap().permissions()) & 0o777, 0o751);

        let back = tempfile::tempdir().unwrap();
        let id = remote.transfer(job(Direction::Download, "/project/src", back.path().to_str().unwrap(), "deep", true));
        assert_eq!(finish(&mut remote, &mut events, id, Control::Cancel).last(), Some(&TransferEvent::Finished));
        assert_eq!(std::fs::read(back.path().join("deep/data.bin")).unwrap(), big);
    }

    #[test]
    fn discard_stops_a_transfer() {
        let (served, local, state) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        std::fs::write(local.path().join("big.bin"), vec![1u8; 4_000_000]).unwrap();
        let (mut remote, mut events) = connected(served.path(), state.path());

        let id = remote.transfer(job(Direction::Upload, local.path().to_str().unwrap(), "/", "big.bin", false));
        remote.control(id, Control::Cancel);
        let seen = finish(&mut remote, &mut events, id, Control::Cancel);
        assert_eq!(seen.last(), Some(&TransferEvent::Cancelled));
        assert!(std::fs::metadata(served.path().join("big.bin")).map_or(true, |m| m.len() < 4_000_000));
    }

    #[test]
    fn replace_deletes_the_old_item_first() {
        let (served, local, state) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        std::fs::create_dir_all(served.path().join("site/old")).unwrap();
        std::fs::write(served.path().join("site/old/stale.txt"), b"stale").unwrap();
        std::fs::create_dir(local.path().join("site")).unwrap();
        std::fs::write(local.path().join("site/index.html"), b"new").unwrap();
        let (mut remote, mut events) = connected(served.path(), state.path());

        let mut replacing = job(Direction::Upload, local.path().to_str().unwrap(), "/", "site", true);
        replacing.planned.replace = true;
        let id = remote.transfer(replacing);
        assert_eq!(finish(&mut remote, &mut events, id, Control::Cancel).last(), Some(&TransferEvent::Finished));
        assert!(!served.path().join("site/old").exists());
        assert_eq!(std::fs::read(served.path().join("site/index.html")).unwrap(), b"new");
    }

    #[test]
    fn a_failed_file_waits_for_a_decision() {
        let (served, local, state) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (mut remote, mut events) = connected(served.path(), state.path());

        let id = remote.transfer(job(Direction::Download, "/", local.path().to_str().unwrap(), "missing.txt", false));
        let seen = finish(&mut remote, &mut events, id, Control::Skip);
        assert!(matches!(&seen[0], TransferEvent::Failed { path, .. } if path == "/missing.txt"), "{seen:?}");
        assert_eq!(seen.last(), Some(&TransferEvent::Finished));

        let id = remote.transfer(job(Direction::Download, "/", local.path().to_str().unwrap(), "missing.txt", false));
        assert_eq!(finish(&mut remote, &mut events, id, Control::Cancel).last(), Some(&TransferEvent::Cancelled));
    }

    #[test]
    fn renames_creates_and_deletes_remote_items() {
        let (served, state) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        std::fs::create_dir_all(served.path().join("logs/2026")).unwrap();
        std::fs::write(served.path().join("logs/2026/a.log"), b"x").unwrap();
        std::fs::write(served.path().join("notes.txt"), b"x").unwrap();
        let (remote, mut events) = connected(served.path(), state.path());

        remote.operate(vec![
            Operation::Rename { from: "/notes.txt".into(), to: "/todo.txt".into() },
            Operation::MakeDir("/new".into()),
            Operation::Remove { path: "/logs".into(), is_dir: true },
            #[cfg(unix)]
            Operation::Chmod { path: "/todo.txt".into(), mode: 0o640 },
        ]);
        assert_eq!(next(&mut events), RemoteEvent::Changed(Ok(())));
        #[cfg(unix)]
        assert_eq!(std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(served.path().join("todo.txt")).unwrap().permissions()) & 0o777, 0o640);
        assert!(served.path().join("todo.txt").exists() && served.path().join("new").is_dir());
        assert!(!served.path().join("logs").exists());

        remote.operate(vec![Operation::Remove { path: "/nothing".into(), is_dir: false }]);
        assert!(matches!(next(&mut events), RemoteEvent::Changed(Err(_))));

        remote.names(7, "/");
        let RemoteEvent::Names { request: 7, result: Ok(mut names) } = next(&mut events) else { panic!("expected names") };
        names.sort();
        assert_eq!(names, ["new", "todo.txt"]);
        remote.names(8, "/missing");
        assert!(matches!(next(&mut events), RemoteEvent::Names { request: 8, result: Err(_) }));
    }

    #[test]
    fn local_operations_use_the_same_rules() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        std::fs::create_dir_all(dir.path().join("tree/sub")).unwrap();
        std::fs::write(dir.path().join("tree/sub/f"), b"x").unwrap();
        operate_locally(vec![
            Operation::MakeDir(join(root, "made")),
            Operation::MakeDir(join(root, "made")),
            Operation::Rename { from: join(root, "made"), to: join(root, "renamed") },
            Operation::Remove { path: join(root, "tree"), is_dir: true },
        ])
        .unwrap();
        assert!(dir.path().join("renamed").is_dir() && !dir.path().join("tree").exists());
        assert!(operate_locally(vec![Operation::Remove { path: join(root, "tree"), is_dir: true }]).is_err());
    }

    /// Host to host as the SFTP tab does it: down from one server into a
    /// temporary folder, then up to the other one.
    #[test]
    fn copies_between_two_hosts_through_a_temporary_folder() {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        std::fs::write(a.path().join("report.txt"), b"from host a").unwrap();
        std::fs::create_dir_all(a.path().join("site/css")).unwrap();
        std::fs::write(a.path().join("site/css/main.css"), b"body{}").unwrap();
        let state = tempfile::tempdir().unwrap();
        let (mut source, mut source_events) = connected(a.path(), state.path());
        let (mut target, mut target_events) = connected(b.path(), state.path());
        for (name, is_dir) in [("report.txt", false), ("site", true)] {
            let temp = tempfile::tempdir().unwrap();
            let temp_dir = temp.path().to_string_lossy().into_owned();
            let id = source.transfer(job(Direction::Download, "/", &temp_dir, name, is_dir));
            assert_eq!(finish(&mut source, &mut source_events, id, Control::Cancel).last(), Some(&TransferEvent::Finished));
            let id = target.transfer(job(Direction::Upload, &temp_dir, "/", name, is_dir));
            assert_eq!(finish(&mut target, &mut target_events, id, Control::Cancel).last(), Some(&TransferEvent::Finished));
        }
        assert_eq!(std::fs::read(b.path().join("report.txt")).unwrap(), b"from host a");
        assert_eq!(std::fs::read(b.path().join("site/css/main.css")).unwrap(), b"body{}");
    }
}