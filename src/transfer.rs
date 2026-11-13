//! Copying between the local disk and an SFTP server: "Copy to target
//! directory" and drag and drop between the SFTP panels. Name clashes are
//! settled first ([`Resolver`]), then each item is copied as one transfer
//! that reports [`Progress`] and waits for a [`Control`] when a file fails.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use futures::StreamExt;
use russh_sftp::client::fs::Metadata;
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::files::{format_size, join, list_local, Entry};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Upload,
    Download,
}

/// Answer to "File already exists".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conflict {
    Skip,
    Replace,
    /// Copy under a free `Copy of …` name.
    Duplicate,
    /// Folders only: copy into the existing folder.
    Merge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    pub is_dir: bool,
}

/// One item to copy once clashes are settled.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub item: Item,
    /// Name in the destination folder.
    pub target: String,
    /// Delete what is already at the target first.
    pub replace: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConflictQuestion {
    pub name: String,
    pub is_dir: bool,
    /// Last clash left: the dialog has no "Apply to All".
    pub single: bool,
}

#[derive(Debug, PartialEq)]
pub enum Step {
    Ask(ConflictQuestion),
    Done(Vec<Planned>),
}

/// Walks the items against the destination listing, asking about each clash
/// the way the legacy transfer dialog does (names compare case-insensitively).
pub struct Resolver {
    items: Vec<Item>,
    next: usize,
    taken: Vec<String>,
    item_names: Vec<String>,
    clashes: Vec<String>,
    remaining: usize,
    for_all: Option<Conflict>,
    planned: Vec<Planned>,
}

impl Resolver {
    pub fn new(items: Vec<Item>, destination: &[String]) -> Self {
        let taken: Vec<String> = destination.iter().map(|n| n.to_lowercase()).collect();
        let item_names: Vec<String> = items.iter().map(|i| i.name.to_lowercase()).collect();
        let clashes: Vec<String> = taken.iter().filter(|n| item_names.contains(n)).cloned().collect();
        Self {
            remaining: clashes.len(),
            items,
            next: 0,
            taken,
            item_names,
            clashes,
            for_all: None,
            planned: Vec::new(),
        }
    }

    /// Plans items up to the next clash that needs an answer.
    pub fn step(&mut self) -> Step {
        while let Some(item) = self.items.get(self.next).cloned() {
            if self.clashes.contains(&item.name.to_lowercase()) {
                let Some(conflict) = self.for_all else {
                    return Step::Ask(ConflictQuestion { name: item.name, is_dir: item.is_dir, single: self.remaining == 1 });
                };
                self.plan(item, conflict);
            } else {
                self.planned.push(Planned { target: item.name.clone(), item, replace: false });
            }
            self.next += 1;
        }
        Step::Done(std::mem::take(&mut self.planned))
    }

    /// Answers the clash [`Resolver::step`] asked about.
    pub fn answer(&mut self, conflict: Conflict, for_all: bool) {
        let Some(item) = self.items.get(self.next).cloned() else { return };
        if for_all {
            self.for_all = Some(conflict);
        }
        self.remaining = self.remaining.saturating_sub(1);
        self.plan(item, conflict);
        self.next += 1;
    }

    fn plan(&mut self, item: Item, conflict: Conflict) {
        match conflict {
            Conflict::Skip => {}
            Conflict::Replace => self.planned.push(Planned { target: item.name.clone(), item, replace: true }),
            Conflict::Merge => self.planned.push(Planned { target: item.name.clone(), item, replace: false }),
            Conflict::Duplicate => {
                let target = self.free_name(&item.name);
                let lower = target.to_lowercase();
                self.taken.push(lower.clone());
                // A new name can clash with a later item: that one is asked about too.
                if self.item_names.contains(&lower) {
                    self.clashes.push(lower);
                    self.remaining += 1;
                }
                self.planned.push(Planned { item, target, replace: false });
            }
        }
    }

    fn free_name(&self, name: &str) -> String {
        let mut name = copy_of(name);
        while self.taken.contains(&name.to_lowercase()) {
            name = copy_of(&name);
        }
        name
    }
}

/// `Copy of notes.txt`; dotfiles keep the dot in front: `.Copy of bashrc`.
fn copy_of(name: &str) -> String {
    match name.strip_prefix('.') {
        Some(rest) => format!(".Copy of {rest}"),
        None => format!("Copy of {name}"),
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub elapsed: Duration,
}

impl Progress {
    pub fn percentage(&self) -> f32 {
        if self.bytes_total > 0 {
            self.bytes_done as f32 * 100. / self.bytes_total as f32
        } else if self.files_total > 0 {
            self.files_done as f32 * 100. / self.files_total as f32
        } else {
            0.
        }
    }

    /// Bytes per second so far.
    pub fn speed(&self) -> u64 {
        let seconds = self.elapsed.as_secs_f64();
        if seconds > 0. { (self.bytes_done as f64 / seconds) as u64 } else { 0 }
    }

    pub fn remaining(&self) -> Option<Duration> {
        let speed = self.speed();
        let left = self.bytes_total.saturating_sub(self.bytes_done);
        (speed > 0 && left > 0).then(|| Duration::from_secs(left.div_ceil(speed)))
    }

    /// The info line of a transfer row: `2 of 5 files, 1.20 MB/5.00 MB, 300.00 kB/s, 13s`.
    pub fn summary(&self, is_dir: bool) -> String {
        let mut parts = Vec::new();
        if is_dir {
            parts.push(format!("{} of {} files", self.files_done, self.files_total));
        }
        parts.push(format!("{}/{}", format_size(self.bytes_done), format_size(self.bytes_total)));
        parts.push(format!("{}/s", format_size(self.speed())));
        if let Some(left) = self.remaining() {
            parts.push(format_duration(left));
        }
        parts.join(", ")
    }
}

/// `13s`, `2m 5s`, `1h 2m 3s`.
pub fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds}s"),
        (0, _) => format!("{minutes}m {seconds}s"),
        _ => format!("{hours}h {minutes}m {seconds}s"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TransferEvent {
    Progress(Progress),
    /// Waiting for a [`Control`] about this path.
    Failed { path: String, error: String },
    Finished,
    Cancelled,
}

/// What the user decided about a failed file, or Discard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Retry,
    Skip,
    SkipAll,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub direction: Direction,
    pub source_dir: String,
    pub destination_dir: String,
    pub planned: Planned,
}

/// One side of a transfer.
#[derive(Clone)]
pub(crate) enum Fs {
    Local,
    Remote(Arc<SftpSession>),
}

type Reader = Box<dyn AsyncRead + Unpin + Send>;
type Writer = Box<dyn AsyncWrite + Unpin + Send>;

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// `path: reason`. SFTP statuses often repeat their code as the message
/// (`Failure: Failure`); the repeat is dropped.
fn at(path: &str, error: impl std::fmt::Display) -> String {
    let error = error.to_string();
    let reason = match error.split_once(": ") {
        Some((code, message)) if code == message => code.to_owned(),
        _ => error,
    };
    format!("{path}: {reason}")
}

impl Fs {
    pub(crate) async fn list(&self, path: &str) -> Result<Vec<Entry>, String> {
        match self {
            Fs::Local => {
                let path = path.to_owned();
                tokio::task::spawn_blocking(move || list_local(&path)).await.map_err(text)?
            }
            Fs::Remote(sftp) => crate::remote_fs::list(sftp, path).await,
        }
    }

    async fn size(&self, path: &str) -> Result<(u64, Option<u32>), String> {
        match self {
            Fs::Local => {
                let meta = tokio::fs::metadata(path).await.map_err(|e| at(path, e))?;
                #[cfg(unix)]
                let mode = Some(std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o7777);
                #[cfg(not(unix))]
                let mode = None;
                Ok((meta.len(), mode))
            }
            Fs::Remote(sftp) => {
                let meta = sftp.metadata(path).await.map_err(|e| at(path, e))?;
                Ok((meta.size.unwrap_or(0), meta.permissions.map(|p| p & 0o7777)))
            }
        }
    }

    async fn exists(&self, path: &str) -> bool {
        match self {
            Fs::Local => tokio::fs::symlink_metadata(path).await.is_ok(),
            Fs::Remote(sftp) => sftp.symlink_metadata(path).await.is_ok(),
        }
    }

    async fn open(&self, path: &str) -> Result<Reader, String> {
        match self {
            Fs::Local => Ok(Box::new(tokio::fs::File::open(path).await.map_err(|e| at(path, e))?)),
            Fs::Remote(sftp) => Ok(Box::new(sftp.open(path).await.map_err(|e| at(path, e))?)),
        }
    }

    /// Creates or truncates a file.
    async fn create(&self, path: &str) -> Result<Writer, String> {
        match self {
            Fs::Local => Ok(Box::new(tokio::fs::File::create(path).await.map_err(|e| at(path, e))?)),
            Fs::Remote(sftp) => Ok(Box::new(sftp.create(path).await.map_err(|e| at(path, e))?)),
        }
    }

    async fn set_mode(&self, path: &str, mode: u32) {
        match self {
            #[cfg(unix)]
            Fs::Local => {
                let _ = tokio::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode)).await;
            }
            #[cfg(not(unix))]
            Fs::Local => {}
            Fs::Remote(sftp) => {
                let _ = sftp.set_metadata(path, Metadata { permissions: Some(mode), ..Default::default() }).await;
            }
        }
    }

    /// Creates a folder; an existing one is fine.
    pub(crate) async fn make_dir(&self, path: &str) -> Result<(), String> {
        let created = match self {
            Fs::Local => tokio::fs::create_dir(path).await.map_err(text),
            Fs::Remote(sftp) => sftp.create_dir(path).await.map_err(text),
        };
        match created {
            Err(_) if self.is_dir(path).await => Ok(()),
            other => other.map_err(|e| at(path, e)),
        }
    }

    async fn is_dir(&self, path: &str) -> bool {
        match self {
            Fs::Local => tokio::fs::metadata(path).await.is_ok_and(|m| m.is_dir()),
            Fs::Remote(sftp) => sftp.metadata(path).await.is_ok_and(|m| m.is_dir()),
        }
    }

    /// Deletes a file, link or whole folder. Links are removed, not followed.
    pub(crate) async fn remove(&self, path: &str, is_dir: bool) -> Result<(), String> {
        match self {
            Fs::Local => {
                let meta = tokio::fs::symlink_metadata(path).await.map_err(|e| at(path, e))?;
                let removed = if meta.is_dir() { tokio::fs::remove_dir_all(path).await } else { tokio::fs::remove_file(path).await };
                removed.map_err(|e| at(path, e))
            }
            Fs::Remote(sftp) => {
                let link = sftp.symlink_metadata(path).await.is_ok_and(|m| m.is_symlink());
                if is_dir && !link {
                    // Depth first, iteratively: deep trees must not overflow the stack.
                    let mut stack = vec![(path.to_owned(), false)];
                    while let Some((dir, emptied)) = stack.pop() {
                        if emptied {
                            sftp.remove_dir(&dir).await.map_err(|e| at(&dir, e))?;
                            continue;
                        }
                        stack.push((dir.clone(), true));
                        for entry in crate::remote_fs::list(sftp, &dir).await? {
                            let child = join(&dir, &entry.name);
                            if entry.is_dir && !entry.is_link {
                                stack.push((child, false));
                            } else {
                                sftp.remove_file(&child).await.map_err(|e| at(&child, e))?;
                            }
                        }
                    }
                    Ok(())
                } else {
                    sftp.remove_file(path).await.map_err(|e| at(path, e))
                }
            }
        }
    }

    pub(crate) async fn chmod(&self, path: &str, mode: u32) -> Result<(), String> {
        match self {
            #[cfg(unix)]
            Fs::Local => tokio::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode)).await.map_err(|e| at(path, e)),
            #[cfg(not(unix))]
            Fs::Local => Err(at(path, "permissions are not supported here")),
            Fs::Remote(sftp) => {
                sftp.set_metadata(path, Metadata { permissions: Some(mode), ..Default::default() }).await.map_err(|e| at(path, e))
            }
        }
    }

    pub(crate) async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        match self {
            Fs::Local => tokio::fs::rename(from, to).await.map_err(|e| at(from, e)),
            Fs::Remote(sftp) => sftp.rename(from, to).await.map_err(|e| at(from, e)),
        }
    }
}

/// A file or folder of a transfer, relative to the copied item.
struct Part {
    relative: String,
    is_dir: bool,
    size: u64,
    mode: Option<u32>,
}

/// Everything under the item, folders before their contents. Links to
/// folders inside the tree are skipped so a link loop cannot recurse forever.
async fn scan(fs: &Fs, root: &str, is_dir: bool) -> Result<Vec<Part>, String> {
    if !is_dir {
        let (size, mode) = fs.size(root).await?;
        return Ok(vec![Part { relative: String::new(), is_dir: false, size, mode }]);
    }
    let mut parts = vec![Part { relative: String::new(), is_dir: true, size: 0, mode: None }];
    let mut pending = vec![String::new()];
    while let Some(relative) = pending.pop() {
        let dir = if relative.is_empty() { root.to_owned() } else { join(root, &relative) };
        for entry in fs.list(&dir).await? {
            if entry.is_dir && entry.is_link {
                continue;
            }
            let child = if relative.is_empty() { entry.name.clone() } else { join(&relative, &entry.name) };
            if entry.is_dir {
                pending.push(child.clone());
            }
            parts.push(Part { relative: child, is_dir: entry.is_dir, size: if entry.is_dir { 0 } else { entry.size }, mode: entry.mode });
        }
    }
    Ok(parts)
}

enum Stop {
    Cancelled,
    Error(String),
}

/// How often progress is reported while bytes flow.
const REPORT_EVERY: Duration = Duration::from_millis(100);
const CHUNK: usize = 256 * 1024;

struct Run<'a, E: Fn(TransferEvent)> {
    source: &'a Fs,
    target: &'a Fs,
    controls: &'a mut UnboundedReceiver<Control>,
    emit: &'a E,
    progress: Progress,
    started: Instant,
    reported: Instant,
}

impl<E: Fn(TransferEvent)> Run<'_, E> {
    fn report(&mut self, force: bool) {
        if force || self.reported.elapsed() >= REPORT_EVERY {
            self.progress.elapsed = self.started.elapsed();
            self.reported = Instant::now();
            (self.emit)(TransferEvent::Progress(self.progress.clone()));
        }
    }

    fn cancelled(&mut self) -> bool {
        // A closed channel means the connection was dropped.
        matches!(self.controls.try_recv(), Ok(Control::Cancel) | Err(TryRecvError::Closed))
    }

    async fn copy_file(&mut self, from: &str, to: &str, mode: Option<u32>) -> Result<(), Stop> {
        let mut reader = self.source.open(from).await.map_err(Stop::Error)?;
        let mut writer = self.target.create(to).await.map_err(Stop::Error)?;
        let mut buffer = vec![0; CHUNK];
        let mut copied = 0;
        let result = async {
            loop {
                if self.cancelled() {
                    return Err(Stop::Cancelled);
                }
                let read = reader.read(&mut buffer).await.map_err(|e| Stop::Error(format!("{from}: {e}")))?;
                if read == 0 {
                    break;
                }
                writer.write_all(&buffer[..read]).await.map_err(|e| Stop::Error(format!("{to}: {e}")))?;
                copied += read as u64;
                self.progress.bytes_done += read as u64;
                self.report(false);
            }
            writer.shutdown().await.map_err(|e| Stop::Error(format!("{to}: {e}")))
        }
        .await;
        if result.is_err() {
            // A retried file starts over, so its bytes are counted again.
            self.progress.bytes_done -= copied;
        }
        result?;
        if let Some(mode) = mode {
            self.target.set_mode(to, mode).await;
        }
        Ok(())
    }

    /// Reports a failure and waits for the user's decision.
    async fn decide(&mut self, path: &str, error: String) -> Control {
        (self.emit)(TransferEvent::Failed { path: path.to_owned(), error });
        self.controls.next().await.unwrap_or(Control::Cancel)
    }
}

/// Runs one transfer to the end. Every outcome is reported through `emit`.
pub(crate) async fn transfer(source: Fs, target: Fs, job: Job, mut controls: UnboundedReceiver<Control>, emit: impl Fn(TransferEvent)) {
    let from_root = join(&job.source_dir, &job.planned.item.name);
    let to_root = join(&job.destination_dir, &job.planned.target);
    let now = Instant::now();
    let mut run = Run { source: &source, target: &target, controls: &mut controls, emit: &emit, progress: Progress::default(), started: now, reported: now };

    let parts = loop {
        match scan(&source, &from_root, job.planned.item.is_dir).await {
            Ok(parts) => break parts,
            Err(error) => match run.decide(&from_root, error).await {
                Control::Retry => continue,
                Control::Skip | Control::SkipAll => return emit(TransferEvent::Finished),
                Control::Cancel => return emit(TransferEvent::Cancelled),
            },
        }
    };
    run.progress.files_total = parts.iter().filter(|p| !p.is_dir).count();
    run.progress.bytes_total = parts.iter().map(|p| p.size).sum();
    run.report(true);

    while job.planned.replace && target.exists(&to_root).await {
        let Err(error) = target.remove(&to_root, job.planned.item.is_dir).await else { break };
        match run.decide(&to_root, error).await {
            Control::Retry => continue,
            Control::Skip | Control::SkipAll => return emit(TransferEvent::Finished),
            Control::Cancel => return emit(TransferEvent::Cancelled),
        }
    }

    let mut skip_all = false;
    for part in parts {
        let path = |root: &str| if part.relative.is_empty() { root.to_owned() } else { join(root, &part.relative) };
        let (from, to) = (path(&from_root), path(&to_root));
        loop {
            let result = if part.is_dir { target.make_dir(&to).await.map_err(Stop::Error) } else { run.copy_file(&from, &to, part.mode).await };
            let error = match result {
                Ok(()) => break,
                Err(Stop::Cancelled) => return emit(TransferEvent::Cancelled),
                Err(Stop::Error(error)) => error,
            };
            if skip_all {
                break;
            }
            match run.decide(&from, error).await {
                Control::Retry => continue,
                Control::Skip => break,
                Control::SkipAll => {
                    skip_all = true;
                    break;
                }
                Control::Cancel => return emit(TransferEvent::Cancelled),
            }
        }
        if !part.is_dir {
            // Skipped files count as handled so the bar still reaches the end.
            run.progress.files_done += 1;
            run.report(true);
        }
    }
    emit(TransferEvent::Finished);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(names: &[(&str, bool)]) -> Vec<Item> {
        names.iter().map(|(name, is_dir)| Item { name: (*name).into(), is_dir: *is_dir }).collect()
    }

    fn targets(step: Step) -> Vec<(String, bool)> {
        match step {
            Step::Done(planned) => planned.into_iter().map(|p| (p.target, p.replace)).collect(),
            Step::Ask(question) => panic!("unexpected question about {}", question.name),
        }
    }

    #[test]
    fn items_without_clashes_are_planned_as_is() {
        let mut resolver = Resolver::new(items(&[("a.txt", false), ("docs", true)]), &["b.txt".into()]);
        assert_eq!(targets(resolver.step()), [("a.txt".into(), false), ("docs".into(), false)]);
    }

    #[test]
    fn each_clash_is_asked_about_case_insensitively() {
        let mut resolver = Resolver::new(items(&[("Notes.txt", false), ("docs", true), ("new", false)]), &["notes.txt".into(), "DOCS".into()]);
        assert_eq!(resolver.step(), Step::Ask(ConflictQuestion { name: "Notes.txt".into(), is_dir: false, single: false }));
        resolver.answer(Conflict::Replace, false);
        assert_eq!(resolver.step(), Step::Ask(ConflictQuestion { name: "docs".into(), is_dir: true, single: true }));
        resolver.answer(Conflict::Skip, false);
        assert_eq!(targets(resolver.step()), [("Notes.txt".into(), true), ("new".into(), false)]);
    }

    #[test]
    fn apply_to_all_answers_the_remaining_clashes() {
        let mut resolver = Resolver::new(items(&[("a", false), ("b", false)]), &["a".into(), "b".into()]);
        assert!(matches!(resolver.step(), Step::Ask(_)));
        resolver.answer(Conflict::Replace, true);
        assert_eq!(targets(resolver.step()), [("a".into(), true), ("b".into(), true)]);
    }

    #[test]
    fn duplicate_picks_a_free_copy_name() {
        let mut resolver = Resolver::new(
            items(&[("notes.txt", false), (".bashrc", false)]),
            &["notes.txt".into(), "copy of notes.txt".into(), ".bashrc".into()],
        );
        resolver.step();
        resolver.answer(Conflict::Duplicate, true);
        assert_eq!(targets(resolver.step()), [("Copy of Copy of notes.txt".into(), false), (".Copy of bashrc".into(), false)]);
    }

    #[test]
    fn a_duplicate_name_that_hits_a_later_item_is_asked_about() {
        let mut resolver = Resolver::new(items(&[("a", false), ("Copy of a", false)]), &["a".into()]);
        assert!(matches!(resolver.step(), Step::Ask(ConflictQuestion { single: true, .. })));
        resolver.answer(Conflict::Duplicate, false);
        assert_eq!(resolver.step(), Step::Ask(ConflictQuestion { name: "Copy of a".into(), is_dir: false, single: true }));
        resolver.answer(Conflict::Duplicate, false);
        assert_eq!(targets(resolver.step()), [("Copy of a".into(), false), ("Copy of Copy of a".into(), false)]);
    }

    #[test]
    fn repeated_status_text_is_shown_once() {
        assert_eq!(at("/tmp/x", "Failure: Failure"), "/tmp/x: Failure");
        assert_eq!(at("/tmp/x", "No such file: gone"), "/tmp/x: No such file: gone");
    }

    #[test]
    fn progress_summary_matches_the_legacy_row() {
        let progress = Progress { files_done: 2, files_total: 5, bytes_done: 1_200_000, bytes_total: 5_000_000, elapsed: Duration::from_secs(4) };
        assert_eq!(progress.summary(true), "2 of 5 files, 1.20 MB/5.00 MB, 300.00 kB/s, 13s");
        assert_eq!(progress.summary(false), "1.20 MB/5.00 MB, 300.00 kB/s, 13s");
        assert_eq!(progress.percentage(), 24.);
        assert_eq!(format_duration(Duration::from_secs(3723)), "1h 2m 3s");
        assert_eq!(format_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(Progress::default().summary(false), "0.00 Bytes/0.00 Bytes, 0.00 Bytes/s");
    }
}
