//! Copying between the local disk and an SFTP server: "Copy to target
//! directory" and drag and drop between the SFTP panels. Name clashes are
//! settled first ([`Resolver`]), then each item is copied as one transfer
//! that reports [`Progress`] and waits for a [`Control`] when a file fails.

use std::io::{self, SeekFrom};
use std::path::{Component, Path};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::channel::mpsc::{TryRecvError, UnboundedReceiver};
use futures::StreamExt;
use russh_sftp::client::fs::Metadata;
use russh_sftp::client::{RawSftpSession, SftpSession};
use russh_sftp::protocol::OpenFlags;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use crate::files::{format_size, join, list_local, valid_entry_name, valid_local_name, Entry};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Create,
    Merge,
    Replace,
}

/// One item to copy once clashes are settled.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub item: Item,
    /// Name in the destination folder.
    pub target: String,
    pub destination: Destination,
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
                self.planned.push(Planned { target: item.name.clone(), item, destination: Destination::Create });
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
            Conflict::Replace => self.planned.push(Planned { target: item.name.clone(), item, destination: Destination::Replace }),
            Conflict::Merge => self.planned.push(Planned { target: item.name.clone(), item, destination: Destination::Merge }),
            Conflict::Duplicate => {
                let target = self.free_name(&item.name);
                let lower = target.to_lowercase();
                self.taken.push(lower.clone());
                // A new name can clash with a later item: that one is asked about too.
                if self.item_names.contains(&lower) {
                    self.clashes.push(lower);
                    self.remaining += 1;
                }
                self.planned.push(Planned { item, target, destination: Destination::Create });
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
    Incomplete,
    CommitFailed { error: String },
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
    Remote(Arc<SftpSession>, Option<Arc<RawSftpSession>>),
}

trait ReadSeek: AsyncRead + AsyncSeek + Unpin + Send {}
impl<T: AsyncRead + AsyncSeek + Unpin + Send> ReadSeek for T {}
trait WriteSeek: AsyncWrite + AsyncSeek + Unpin + Send {}
impl<T: AsyncWrite + AsyncSeek + Unpin + Send> WriteSeek for T {}
type Reader = Box<dyn ReadSeek>;
type Writer = Box<dyn WriteSeek>;

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
            Fs::Remote(sftp, _) => crate::remote_fs::list(sftp, path).await,
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
            Fs::Remote(sftp, _) => {
                let meta = sftp.metadata(path).await.map_err(|e| at(path, e))?;
                Ok((meta.size.unwrap_or(0), meta.permissions.map(|p| p & 0o7777)))
            }
        }
    }

    async fn kind(&self, path: &str) -> Result<Option<bool>, String> {
        match self {
            Fs::Local => match tokio::fs::symlink_metadata(path).await {
                Ok(meta) => Ok(Some(meta.is_dir() && !meta.file_type().is_symlink()
                    && !is_reparse_point(&meta))),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(at(path, e)),
            },
            Fs::Remote(sftp, _) => match sftp.symlink_metadata(path).await {
                Ok(meta) => Ok(Some(meta.is_dir() && !meta.is_symlink())),
                Err(russh_sftp::client::error::Error::Status(status))
                    if status.status_code == russh_sftp::protocol::StatusCode::NoSuchFile => Ok(None),
                Err(e) => Err(at(path, e)),
            },
        }
    }

    fn valid_name(&self, name: &str) -> bool {
        match self {
            Fs::Local => valid_local_name(name),
            Fs::Remote(..) => valid_entry_name(name),
        }
    }

    async fn stage(&self, dir: &str) -> Result<String, String> {
        match self {
            Fs::Local => tempfile::Builder::new().prefix(".termian-copy-").tempdir_in(dir)
                .map(|dir| dir.keep().to_string_lossy().into_owned()).map_err(|e| at(dir, e)),
            Fs::Remote(..) => {
                let path = join(dir, &format!(".termian-copy-{}", uuid::Uuid::new_v4()));
                self.private_dir(&path).await?;
                Ok(path)
            }
        }
    }

    async fn open(&self, path: &str) -> Result<Reader, String> {
        match self {
            Fs::Local => Ok(Box::new(tokio::fs::File::open(path).await.map_err(|e| at(path, e))?)),
            Fs::Remote(sftp, _) => Ok(Box::new(sftp.open(path).await.map_err(|e| at(path, e))?)),
        }
    }

    async fn create(&self, path: &str) -> Result<Writer, String> {
        match self {
            Fs::Local => Ok(Box::new(tokio::fs::OpenOptions::new().write(true).create_new(true).open(path).await.map_err(|e| at(path, e))?)),
            Fs::Remote(sftp, _) => Ok(Box::new(sftp.open_with_flags_and_attributes(path, OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCLUDE, Metadata { permissions: Some(0o600), ..Default::default() }).await.map_err(|e| at(path, e))?)),
        }
    }

    /// Opens an existing partial file without truncating it.
    async fn open_write(&self, path: &str) -> Result<Writer, String> {
        match self {
            Fs::Local => Ok(Box::new(tokio::fs::OpenOptions::new().write(true).open(path).await.map_err(|e| at(path, e))?)),
            Fs::Remote(sftp, _) => Ok(Box::new(sftp.open_with_flags(path, OpenFlags::WRITE).await.map_err(|e| at(path, e))?)),
        }
    }

    async fn set_mode(&self, path: &str, mode: u32) -> Result<(), String> {
        #[cfg(not(unix))]
        if matches!(self, Fs::Local) { return Ok(()); }
        self.chmod(path, mode & 0o777).await
    }

    async fn private_dir(&self, path: &str) -> Result<(), String> {
        match self {
            Fs::Remote(sftp, Some(raw)) => {
                raw.mkdir(path, Metadata { permissions: Some(0o700), ..Default::default() }).await.map_err(|e| at(path, e))?;
                let check = sftp.symlink_metadata(path).await.map_err(|e| at(path, e)).and_then(|meta| {
                    if meta.is_dir() && meta.permissions.map(|p| p & 0o777) == Some(0o700) { Ok(()) }
                    else { Err(at(path, "SFTP server did not create a private directory")) }
                });
                if check.is_err() { let _ = sftp.remove_dir(path).await; }
                check
            }
            Fs::Remote(_, None) => Err(at(path, "cannot create directory with private permissions")),
            Fs::Local => {
                let mut builder = tokio::fs::DirBuilder::new();
                #[cfg(unix)]
                builder.mode(0o700);
                builder.create(path).await.map_err(|e| at(path, e))
            }
        }
    }

    /// Creates a folder; an existing one is fine.
    pub(crate) async fn make_dir(&self, path: &str) -> Result<(), String> {
        match self.create_dir(path).await {
            Err(_) if self.is_dir(path).await => Ok(()),
            other => other,
        }
    }

    async fn create_dir(&self, path: &str) -> Result<(), String> {
        match self {
            Fs::Local => tokio::fs::create_dir(path).await.map_err(text),
            Fs::Remote(sftp, _) => sftp.create_dir(path).await.map_err(text),
        }.map_err(|e| at(path, e))
    }

    async fn is_dir(&self, path: &str) -> bool {
        self.kind(path).await == Ok(Some(true))
    }

    /// Deletes a file, link or whole folder. Links are removed, not followed.
    pub(crate) async fn remove(&self, path: &str, is_dir: bool) -> Result<(), String> {
        match self {
            Fs::Local => {
                let meta = tokio::fs::symlink_metadata(path).await.map_err(|e| at(path, e))?;
                let removed = if meta.is_dir() || is_reparse_point(&meta) { tokio::fs::remove_dir_all(path).await } else { tokio::fs::remove_file(path).await };
                removed.map_err(|e| at(path, e))
            }
            Fs::Remote(sftp, _) => {
                let meta = sftp.symlink_metadata(path).await.map_err(|e| at(path, e))?;
                if !is_dir || !meta.is_dir() || meta.is_symlink() {
                    return sftp.remove_file(path).await.map_err(|e| at(path, e));
                }
                let parent = crate::files::parent(path).ok_or_else(|| at(path, "cannot remove a root directory"))?;
                let scratch = self.stage(&parent).await?;
                let mut moved: Vec<(String, String)> = Vec::new();
                let result = async {
                    let mut stack = vec![(path.to_owned(), false)];
                    while let Some((dir, finish)) = stack.pop() {
                        if finish {
                            sftp.remove_dir(&dir).await.map_err(|e| at(&dir, e))?;
                            moved.pop();
                            continue;
                        }
                        let meta = sftp.symlink_metadata(&dir).await.map_err(|e| at(&dir, e))?;
                        if !meta.is_dir() || meta.is_symlink() {
                            sftp.remove_file(&dir).await.map_err(|e| at(&dir, e))?;
                            continue;
                        }
                        let isolated = join(&scratch, &uuid::Uuid::new_v4().to_string());
                        sftp.rename(&dir, &isolated).await.map_err(|e| at(&dir, e))?;
                        moved.push((isolated.clone(), dir));
                        let meta = sftp.symlink_metadata(&isolated).await.map_err(|e| at(&isolated, e))?;
                        if !meta.is_dir() || meta.is_symlink() {
                            return Err(at(&isolated, "directory changed during removal"));
                        }
                        stack.push((isolated.clone(), true));
                        for entry in crate::remote_fs::list(sftp, &isolated).await? {
                            stack.push((join(&isolated, &entry.name), false));
                        }
                    }
                    Ok(())
                }.await;
                if result.is_err() {
                    for (isolated, original) in moved.into_iter().rev() {
                        if self.kind(&original).await != Ok(None) || sftp.rename(&isolated, &original).await.is_err() {
                            return Err(at(&scratch, "removal stopped; remaining files were kept here"));
                        }
                    }
                }
                sftp.remove_dir(&scratch).await.map_err(|e| at(&scratch, e))?;
                result
            }
        }
    }

    pub(crate) async fn chmod(&self, path: &str, mode: u32) -> Result<(), String> {
        match self {
            #[cfg(unix)]
            Fs::Local => tokio::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode)).await.map_err(|e| at(path, e)),
            #[cfg(not(unix))]
            Fs::Local => Err(at(path, "permissions are not supported here")),
            Fs::Remote(sftp, _) => {
                sftp.set_metadata(path, Metadata { permissions: Some(mode), ..Default::default() }).await.map_err(|e| at(path, e))
            }
        }
    }

    pub(crate) async fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        match self {
            Fs::Local => tokio::fs::rename(from, to).await.map_err(|e| at(from, e)),
            Fs::Remote(sftp, _) => sftp.rename(from, to).await.map_err(|e| at(from, e)),
        }
    }
}

fn is_reparse_point(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = meta;
        false
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
async fn scan(fs: &Fs, target: &Fs, root: &str, is_dir: bool) -> Result<Vec<Part>, String> {
    if !is_dir {
        let (size, mode) = fs.size(root).await?;
        return Ok(vec![Part { relative: String::new(), is_dir: false, size, mode }]);
    }
    let mut parts = vec![Part { relative: String::new(), is_dir: true, size: 0, mode: fs.size(root).await?.1 }];
    let mut pending = vec![String::new()];
    while let Some(relative) = pending.pop() {
        let dir = if relative.is_empty() { root.to_owned() } else { join(root, &relative) };
        if fs.kind(&dir).await? != Some(true) {
            return Err(at(&dir, "source is not a regular directory"));
        }
        for entry in fs.list(&dir).await? {
            if !target.valid_name(&entry.name) {
                return Err(at(&dir, "invalid destination file name"));
            }
            if entry.is_link && (entry.is_dir || matches!(fs, Fs::Local)) {
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

struct LocalSource {
    base: std::path::PathBuf,
    #[cfg(unix)]
    dir: std::fs::File,
    #[cfg(windows)]
    canonical: std::path::PathBuf,
}

impl LocalSource {
    fn new(base: &Path) -> io::Result<Self> {
        #[cfg(unix)]
        use std::os::unix::fs::OpenOptionsExt;
        Ok(Self {
            base: base.into(),
            #[cfg(unix)]
            dir: std::fs::OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY).open(base)?,
            #[cfg(windows)]
            canonical: base.canonicalize()?,
        })
    }

    fn open(&self, path: &Path) -> io::Result<std::fs::File> {
        let invalid = || io::Error::new(io::ErrorKind::InvalidInput, "source is not a regular file inside the selected directory");
        let relative = path.strip_prefix(&self.base).map_err(|_| invalid())?;
        if relative.as_os_str().is_empty() || relative.components().any(|p| !matches!(p, Component::Normal(_))) {
            return Err(invalid());
        }

        #[cfg(unix)]
        let file = {
            use std::ffi::CString;
            use std::os::fd::{AsRawFd, FromRawFd};
            use std::os::unix::ffi::OsStrExt;
            let mut dir = self.dir.try_clone()?;
            let mut parts = relative.iter().peekable();
            loop {
                let part = parts.next().ok_or_else(invalid)?;
                let name = CString::new(part.as_bytes()).map_err(|_| invalid())?;
                let leaf = parts.peek().is_none();
                let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
                    | if leaf { 0 } else { libc::O_DIRECTORY };
                let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
                if fd < 0 {
                    return Err(io::Error::last_os_error());
                }
                let opened = unsafe { std::fs::File::from_raw_fd(fd) };
                if leaf {
                    break opened;
                }
                dir = opened;
            }
        };

        #[cfg(windows)]
        let file = {
            use std::ffi::OsString;
            use std::os::windows::{ffi::OsStringExt, fs::OpenOptionsExt, io::AsRawHandle};
            use windows_sys::Win32::Storage::FileSystem::{GetFinalPathNameByHandleW, FILE_FLAG_OPEN_REPARSE_POINT};
            let mut expected = self.canonical.clone();
            for part in relative {
                expected.push(part);
                if is_reparse_point(&std::fs::symlink_metadata(&expected)?) {
                    return Err(invalid());
                }
            }
            let file = std::fs::OpenOptions::new().read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).open(&expected)?;
            let handle = file.as_raw_handle();
            let count = unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, 0) };
            if count == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut buf = vec![0u16; count as usize];
            let written = unsafe { GetFinalPathNameByHandleW(handle, buf.as_mut_ptr(), count, 0) };
            if written == 0 || written >= count {
                return Err(invalid());
            }
            if expected != Path::new(&OsString::from_wide(&buf[..written as usize]))
                || is_reparse_point(&file.metadata()?)
            {
                return Err(invalid());
            }
            file
        };

        if !file.metadata()?.is_file() {
            return Err(invalid());
        }
        Ok(file)
    }
}

async fn open_source(source: &Fs, local: Option<&LocalSource>, path: &str) -> Result<Reader, String> {
    if let Some(local) = local {
        let file = local.open(Path::new(path)).map_err(|e| at(path, e))?;
        Ok(Box::new(tokio::fs::File::from_std(file)))
    } else {
        source.open(path).await
    }
}

/// Do not append to a partial that no longer matches the source.
async fn resume_offset(original: &mut Reader, source_size: u64, target: &Fs, from: &str, to: &str, expected: u64) -> Result<u64, String> {
    let offset = target.size(to).await?.0;
    if source_size != expected || offset > source_size {
        return Err(format!("{to}: source or partial file size changed"));
    }
    let mut partial = target.open(to).await?;
    let mut left = offset;
    let mut a = vec![0; CHUNK];
    let mut b = vec![0; CHUNK];
    while left > 0 {
        let n = left.min(CHUNK as u64) as usize;
        original.read_exact(&mut a[..n]).await.map_err(|e| at(from, e))?;
        partial.read_exact(&mut b[..n]).await.map_err(|e| at(to, e))?;
        if a[..n] != b[..n] {
            return Err(format!("{to}: partial file does not match source"));
        }
        left -= n as u64;
    }
    Ok(offset)
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
    local: Option<LocalSource>,
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

    async fn copy_file(&mut self, from: &str, to: &str, size: u64, mode: Option<u32>, started: &mut bool) -> Result<(), Stop> {
        let mut reader = open_source(self.source, self.local.as_ref(), from).await.map_err(Stop::Error)?;
        let offset = if *started {
            let source_size = self.source.size(from).await.map_err(Stop::Error)?.0;
            resume_offset(&mut reader, source_size, self.target, from, to, size).await.map_err(Stop::Error)?
        } else { 0 };
        reader.seek(SeekFrom::Start(offset)).await.map_err(|e| Stop::Error(at(from, e)))?;
        let mut writer = if *started {
            self.target.open_write(to).await.map_err(Stop::Error)?
        } else {
            let writer = self.target.create(to).await.map_err(Stop::Error)?;
            *started = true;
            writer
        };
        writer.seek(SeekFrom::Start(offset)).await.map_err(|e| Stop::Error(at(to, e)))?;
        let mut buffer = vec![0; CHUNK];
        let mut copied = offset;
        self.progress.bytes_done += offset;
        let result = async {
            loop {
                if self.cancelled() {
                    return Err(Stop::Cancelled);
                }
                let read = reader.read(&mut buffer).await.map_err(|e| Stop::Error(format!("{from}: {e}")))?;
                if read == 0 {
                    break;
                }
                if read as u64 > size.saturating_sub(copied) {
                    return Err(Stop::Error(at(from, "source exceeds the declared file size")));
                }
                writer.write_all(&buffer[..read]).await.map_err(|e| Stop::Error(format!("{to}: {e}")))?;
                copied += read as u64;
                self.progress.bytes_done += read as u64;
                self.report(false);
            }
            if copied != size {
                return Err(Stop::Error(at(from, "source size changed during the transfer")));
            }
            writer.shutdown().await.map_err(|e| Stop::Error(format!("{to}: {e}")))?;
            if let Some(mode) = mode {
                self.target.set_mode(to, mode).await.map_err(Stop::Error)?;
            }
            Ok(())
        }
        .await;
        if result.is_err() {
            // The next attempt will recount the validated partial file.
            self.progress.bytes_done -= copied;
        }
        result
    }

    /// Reports a failure and waits for the user's decision.
    async fn decide(&mut self, path: &str, error: String) -> Control {
        (self.emit)(TransferEvent::Failed { path: path.to_owned(), error });
        self.controls.next().await.unwrap_or(Control::Cancel)
    }
}

fn move_new(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "android"))]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let from = CString::new(from.as_os_str().as_bytes())?;
        let to = CString::new(to.as_os_str().as_bytes())?;
        #[cfg(target_os = "macos")]
        let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let result = unsafe { libc::renameat2(libc::AT_FDCWD, from.as_ptr(), libc::AT_FDCWD, to.as_ptr(), libc::RENAME_NOREPLACE) };
        #[cfg(target_os = "android")]
        let result = unsafe { libc::syscall(libc::SYS_renameat2, libc::AT_FDCWD, from.as_ptr(), libc::AT_FDCWD, to.as_ptr(), libc::RENAME_NOREPLACE) };
        if result == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
        let wide = |path: &Path| -> io::Result<Vec<u16>> {
            let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
            if value.contains(&0) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid path")); }
            value.push(0);
            Ok(value)
        };
        let (from, to) = (wide(from)?, wide(to)?);
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0) } != 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "android", windows)))]
    { let _ = (from, to); Err(io::Error::new(io::ErrorKind::Unsupported, "exclusive rename unavailable")) }
}

async fn publish_new(target: &Fs, from: &str, to: &str) -> Result<(), String> {
    match target {
        Fs::Local => move_new(Path::new(from), Path::new(to)).map_err(|e| at(to, e)),
        Fs::Remote(..) => {
            if target.kind(to).await?.is_some() {
                return Err(at(to, "destination appeared during the transfer"));
            }
            target.rename(from, to).await
        }
    }
}

async fn publish_replace(target: &Fs, from: &str, to: &str, backup: &str) -> Result<(), String> {
    let Some(is_dir) = target.kind(to).await? else {
        return publish_new(target, from, to).await;
    };
    publish_new(target, to, backup).await?;
    if let Err(error) = publish_new(target, from, to).await {
        if let Err(rollback) = publish_new(target, backup, to).await {
            return Err(format!("{error}; previous destination is at {backup}: {rollback}"));
        }
        return Err(error);
    }
    target.remove(backup, is_dir).await.map_err(|e| format!("Copied to {to}, but could not remove the backup at {backup}: {e}"))
}

async fn commit(target: &Fs, stage: &str, from: &str, to: &str, parts: &[Part], destination: Destination) -> Result<(), String> {
    let backup = join(stage, "previous");
    match destination {
        Destination::Create => return publish_new(target, from, to).await,
        Destination::Replace => return publish_replace(target, from, to, &backup).await,
        Destination::Merge if !parts[0].is_dir || target.kind(to).await? != Some(true) => {
            return Err(at(to, "cannot merge without a destination directory"));
        }
        Destination::Merge => {}
    }
    let mut created = Vec::new();
    for part in &parts[1..] {
        let dst = join(to, &part.relative);
        if part.is_dir {
            match target.kind(&dst).await? {
                None => { target.private_dir(&dst).await?; created.push((dst, part.mode)); }
                Some(true) => {}
                Some(false) => return Err(at(&dst, "not a directory")),
            }
        } else {
            if target.kind(&dst).await? == Some(true) {
                return Err(at(&dst, "cannot replace a directory with a file during merge"));
            }
            publish_replace(target, &join(from, &part.relative), &dst, &backup).await?;
        }
    }
    for (dir, mode) in created.into_iter().rev() {
        if let Some(mode) = mode { target.set_mode(&dir, mode).await?; }
    }
    Ok(())
}

async fn copy_parts<E: Fn(TransferEvent)>(run: &mut Run<'_, E>, stage: &str, from: &str, to: &str, parts: &[Part], destination: Destination) -> TransferEvent {
    let staged = join(stage, "item");
    for part in parts {
        if run.cancelled() {
            return TransferEvent::Cancelled;
        }
        let path = |root: &str| if part.relative.is_empty() { root.to_owned() } else { join(root, &part.relative) };
        let (from, to) = (path(from), path(&staged));
        let mut started = false;
        loop {
            let result = if part.is_dir { run.target.private_dir(&to).await.map_err(Stop::Error) } else { run.copy_file(&from, &to, part.size, part.mode, &mut started).await };
            let error = match result {
                Ok(()) => break,
                Err(Stop::Cancelled) => return TransferEvent::Cancelled,
                Err(Stop::Error(error)) => error,
            };
            match run.decide(&from, error).await {
                Control::Retry => continue,
                Control::Skip | Control::SkipAll => return TransferEvent::Incomplete,
                Control::Cancel => return TransferEvent::Cancelled,
            }
        }
        if !part.is_dir {
            run.progress.files_done += 1;
            run.report(true);
        }
    }
    if run.cancelled() {
        return TransferEvent::Cancelled;
    }
    if destination != Destination::Merge {
        for part in parts.iter().rev().filter(|p| p.is_dir) {
            let path = if part.relative.is_empty() { staged.clone() } else { join(&staged, &part.relative) };
            if let Some(mode) = part.mode {
                if let Err(error) = run.target.set_mode(&path, mode).await { return TransferEvent::CommitFailed { error }; }
            }
        }
    }
    match commit(run.target, stage, &staged, to, parts, destination).await {
        Ok(()) => TransferEvent::Finished,
        Err(error) => TransferEvent::CommitFailed { error },
    }
}

pub(crate) async fn transfer(source: Fs, target: Fs, job: Job, mut controls: UnboundedReceiver<Control>, emit: impl Fn(TransferEvent)) {
    if !source.valid_name(&job.planned.item.name) || !target.valid_name(&job.planned.target) {
        return emit(TransferEvent::CommitFailed { error: "Invalid transfer file name".into() });
    }
    let from_root = join(&job.source_dir, &job.planned.item.name);
    let to_root = join(&job.destination_dir, &job.planned.target);
    let now = Instant::now();
    let local = match source {
        Fs::Local => match LocalSource::new(Path::new(&job.source_dir)) {
            Ok(local) => Some(local),
            Err(error) => return emit(TransferEvent::CommitFailed { error: at(&job.source_dir, error) }),
        },
        Fs::Remote(..) => None,
    };
    let mut run = Run { source: &source, local, target: &target, controls: &mut controls, emit: &emit, progress: Progress::default(), started: now, reported: now };

    let parts = loop {
        match scan(&source, &target, &from_root, job.planned.item.is_dir).await {
            Ok(parts) => break parts,
            Err(error) => match run.decide(&from_root, error).await {
                Control::Retry => continue,
                Control::Skip | Control::SkipAll => return emit(TransferEvent::Incomplete),
                Control::Cancel => return emit(TransferEvent::Cancelled),
            },
        }
    };
    if run.cancelled() {
        return emit(TransferEvent::Cancelled);
    }
    run.progress.files_total = parts.iter().filter(|p| !p.is_dir).count();
    run.progress.bytes_total = parts.iter().fold(0u64, |total, part| total.saturating_add(part.size));
    run.report(true);
    let stage = match target.stage(&job.destination_dir).await {
        Ok(path) => path,
        Err(error) => return emit(TransferEvent::CommitFailed { error }),
    };
    let end = copy_parts(&mut run, &stage, &from_root, &to_root, &parts, job.planned.destination).await;
    if matches!(target.kind(&join(&stage, "previous")).await, Ok(None)) {
        let _ = target.remove(&stage, true).await;
    }
    emit(end);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn staging_rejects_colliding_entries() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("Report");
        let alias = dir.path().join("report");
        std::fs::write(&existing, b"first").unwrap();
        let collision = if alias.exists() { &alias } else { &existing };
        assert!(Fs::Local.create(collision.to_str().unwrap()).await.is_err());
        assert_eq!(std::fs::read(existing).unwrap(), b"first");
        assert!(Fs::Local.create_dir(dir.path().to_str().unwrap()).await.is_err());
    }

    #[tokio::test]
    async fn create_preserves_a_destination_created_while_copying() {
        let dir = tempfile::tempdir().unwrap();
        let stage = dir.path().join("stage");
        let from = stage.join("item");
        let to = dir.path().join("destination");
        for is_dir in [false, true] {
            std::fs::create_dir_all(&stage).unwrap();
            if is_dir {
                std::fs::create_dir(&from).unwrap();
                std::fs::create_dir(&to).unwrap();
                std::fs::write(from.join("child"), "new").unwrap();
                std::fs::write(to.join("child"), "original").unwrap();
            } else {
                std::fs::write(&from, "new").unwrap();
                std::fs::write(&to, "original").unwrap();
            }
            let parts = scan(&Fs::Local, &Fs::Local, from.to_str().unwrap(), is_dir).await.unwrap();
            let publish = |policy| commit(&Fs::Local, stage.to_str().unwrap(), from.to_str().unwrap(), to.to_str().unwrap(), &parts, policy);
            assert!(publish(Destination::Create).await.is_err());
            if is_dir {
                assert_eq!(std::fs::read(to.join("child")).unwrap(), b"original");
                publish(Destination::Merge).await.unwrap();
                assert_eq!(std::fs::read(to.join("child")).unwrap(), b"new");
            } else {
                assert_eq!(std::fs::read(&to).unwrap(), b"original");
                std::fs::remove_file(&to).unwrap();
                std::fs::remove_file(&from).unwrap();
            }
        }
    }

    #[tokio::test]
    async fn cancelled_replacement_keeps_existing_destination() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        tokio::fs::write(source.path().join("config"), b"new").await.unwrap();
        tokio::fs::write(target.path().join("config"), b"old").await.unwrap();
        let job = Job {
            direction: Direction::Download,
            source_dir: source.path().to_str().unwrap().into(),
            destination_dir: target.path().to_str().unwrap().into(),
            planned: Planned { item: Item { name: "config".into(), is_dir: false }, target: "config".into(), destination: Destination::Replace },
        };
        let (control, controls) = futures::channel::mpsc::unbounded();
        let last = std::cell::RefCell::new(None);
        transfer(Fs::Local, Fs::Local, job, controls, |event| {
            if matches!(event, TransferEvent::Progress(Progress { files_done: 1, .. })) {
                control.unbounded_send(Control::Cancel).unwrap();
            }
            last.replace(Some(event));
        }).await;
        assert_eq!(last.into_inner(), Some(TransferEvent::Cancelled));
        assert_eq!(tokio::fs::read(target.path().join("config")).await.unwrap(), b"old");
        assert_eq!(std::fs::read_dir(target.path()).unwrap().count(), 1);
    }

    async fn local_resume(source: &str, target: &str) -> Result<u64, String> {
        let mut original = Fs::Local.open(source).await?;
        resume_offset(&mut original, Fs::Local.size(source).await?.0, &Fs::Local, source, target, 8).await
    }

    #[tokio::test]
    async fn resume_validates_partial_contents_and_size() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        tokio::fs::write(&source, b"abcdefgh").await.unwrap();
        tokio::fs::write(&target, b"abcd").await.unwrap();
        let (source, target) = (source.to_str().unwrap(), target.to_str().unwrap());
        assert_eq!(local_resume(source, target).await.unwrap(), 4);
        tokio::fs::write(target, b"abXd").await.unwrap();
        assert!(local_resume(source, target).await.is_err());
        tokio::fs::write(target, b"abcdefghi").await.unwrap();
        assert!(local_resume(source, target).await.is_err());
        tokio::fs::write(source, b"changed").await.unwrap();
        tokio::fs::write(target, b"ch").await.unwrap();
        assert!(local_resume(source, target).await.is_err());
    }

    #[tokio::test]
    async fn retry_continues_only_its_own_partial_and_counts_bytes_once() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        tokio::fs::write(&source, b"abcdefgh").await.unwrap();
        tokio::fs::write(&target, b"old contents").await.unwrap();
        let (source, target) = (source.to_str().unwrap(), target.to_str().unwrap());
        let (control, mut controls) = futures::channel::mpsc::unbounded();
        control.unbounded_send(Control::Cancel).unwrap();
        let now = Instant::now();
        let mut run = Run { source: &Fs::Local, local: Some(LocalSource::new(dir.path()).unwrap()), target: &Fs::Local, controls: &mut controls, emit: &|_| {}, progress: Progress::default(), started: now, reported: now };
        let mut started = false;
        assert!(matches!(run.copy_file(source, target, 8, None, &mut started).await, Err(Stop::Error(_))));
        assert!(!started);
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"old contents");
        tokio::fs::remove_file(target).await.unwrap();
        assert!(matches!(run.copy_file(source, target, 8, None, &mut started).await, Err(Stop::Cancelled)));
        assert!(started);
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"");
        tokio::fs::write(target, b"abcd").await.unwrap();
        control.unbounded_send(Control::Cancel).unwrap();
        assert!(matches!(run.copy_file(source, target, 8, None, &mut started).await, Err(Stop::Cancelled)));
        assert_eq!(run.progress.bytes_done, 0);
        assert!(run.copy_file(source, target, 8, None, &mut started).await.is_ok());
        assert_eq!(tokio::fs::read(target).await.unwrap(), b"abcdefgh");
        assert_eq!(run.progress.bytes_done, 8);
        tokio::fs::remove_file(target).await.unwrap();
        let mut started = false;
        assert!(matches!(run.copy_file(source, target, 4, None, &mut started).await, Err(Stop::Error(_))));
        assert!(tokio::fs::metadata(target).await.unwrap().len() <= 4);

        #[cfg(unix)]
        {
            let link = dir.path().join("linked-source");
            std::os::unix::fs::symlink(source, &link).unwrap();
            assert!(run.local.as_ref().unwrap().open(&link).is_err());
        }
    }

    #[tokio::test]
    async fn resumes_upload_and_download_over_sftp() {
        use crate::login::{Answer, HostKeyDecision};
        use crate::session::{connect, EventPrompter, SshOptions, SshTarget};
        use crate::session::tests::server;
        use tokio::sync::Mutex;

        let (served, local, state) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let port = server::start_with_sftp(served.path().to_path_buf());
        let target = SshTarget { host: "127.0.0.1".into(), port, username: Some("root".into()), known_hosts: state.path().join("known_hosts"), identity_files: vec![], password: Some(server::PASSWORD.into()), key: None, options: SshOptions::default() };
        let (answers, rx) = futures::channel::mpsc::unbounded();
        answers.unbounded_send(Answer::HostKey(HostKeyDecision::Once)).unwrap();
        let prompter = Arc::new(Mutex::new(EventPrompter { emit: |_| {}, answers: rx, picked: None }));
        let handle = connect(&target, prompter).await.unwrap();
        let channel = handle.channel_open_session().await.unwrap();
        channel.request_subsystem(true, "sftp").await.unwrap();
        let remote = Fs::Remote(Arc::new(SftpSession::new(channel.into_stream()).await.unwrap()), None);
        let local_fs = Fs::Local;
        let (control, mut controls) = futures::channel::mpsc::unbounded();
        let now = Instant::now();
        for upload in [true, false] {
            let local_path = local.path().join(if upload { "source" } else { "partial" });
            let remote_path = if upload { "/partial" } else { "/source" };
            let served_path = served.path().join(remote_path.trim_start_matches('/'));
            let (source_path, partial_path) = if upload { (&local_path, &served_path) } else { (&served_path, &local_path) };
            tokio::fs::write(source_path, b"abcdefgh").await.unwrap();
            tokio::fs::write(partial_path, b"abcd").await.unwrap();
            let (source, target, from, to) = if upload {
                (&local_fs, &remote, local_path.to_str().unwrap(), remote_path)
            } else {
                (&remote, &local_fs, remote_path, local_path.to_str().unwrap())
            };
            let mut run = Run { source, local: None, target, controls: &mut controls, emit: &|_| {}, progress: Progress::default(), started: now, reported: now };
            let mut started = true;
            assert!(run.copy_file(from, to, 8, None, &mut started).await.is_ok());
            assert_eq!(tokio::fs::read(partial_path).await.unwrap(), b"abcdefgh");
            assert_eq!(run.progress.bytes_done, 8);
        }
        drop(control);
    }

    fn items(names: &[(&str, bool)]) -> Vec<Item> {
        names.iter().map(|(name, is_dir)| Item { name: (*name).into(), is_dir: *is_dir }).collect()
    }

    fn targets(step: Step) -> Vec<(String, Destination)> {
        match step {
            Step::Done(planned) => planned.into_iter().map(|p| (p.target, p.destination)).collect(),
            Step::Ask(question) => panic!("unexpected question about {}", question.name),
        }
    }

    #[test]
    fn items_without_clashes_are_planned_as_is() {
        let mut resolver = Resolver::new(items(&[("a.txt", false), ("docs", true)]), &["b.txt".into()]);
        assert_eq!(targets(resolver.step()), [("a.txt".into(), Destination::Create), ("docs".into(), Destination::Create)]);
    }

    #[test]
    fn each_clash_is_asked_about_case_insensitively() {
        let mut resolver = Resolver::new(items(&[("Notes.txt", false), ("docs", true), ("new", false)]), &["notes.txt".into(), "DOCS".into()]);
        assert_eq!(resolver.step(), Step::Ask(ConflictQuestion { name: "Notes.txt".into(), is_dir: false, single: false }));
        resolver.answer(Conflict::Replace, false);
        assert_eq!(resolver.step(), Step::Ask(ConflictQuestion { name: "docs".into(), is_dir: true, single: true }));
        resolver.answer(Conflict::Skip, false);
        assert_eq!(targets(resolver.step()), [("Notes.txt".into(), Destination::Replace), ("new".into(), Destination::Create)]);
    }

    #[test]
    fn apply_to_all_answers_the_remaining_clashes() {
        let mut resolver = Resolver::new(items(&[("a", false), ("b", false)]), &["a".into(), "b".into()]);
        assert!(matches!(resolver.step(), Step::Ask(_)));
        resolver.answer(Conflict::Replace, true);
        assert_eq!(targets(resolver.step()), [("a".into(), Destination::Replace), ("b".into(), Destination::Replace)]);
    }

    #[test]
    fn duplicate_picks_a_free_copy_name() {
        let mut resolver = Resolver::new(
            items(&[("notes.txt", false), (".bashrc", false)]),
            &["notes.txt".into(), "copy of notes.txt".into(), ".bashrc".into()],
        );
        resolver.step();
        resolver.answer(Conflict::Duplicate, true);
        assert_eq!(targets(resolver.step()), [("Copy of Copy of notes.txt".into(), Destination::Create), (".Copy of bashrc".into(), Destination::Create)]);
    }

    #[test]
    fn a_duplicate_name_that_hits_a_later_item_is_asked_about() {
        let mut resolver = Resolver::new(items(&[("a", false), ("Copy of a", false)]), &["a".into()]);
        assert!(matches!(resolver.step(), Step::Ask(ConflictQuestion { single: true, .. })));
        resolver.answer(Conflict::Duplicate, false);
        assert_eq!(resolver.step(), Step::Ask(ConflictQuestion { name: "Copy of a".into(), is_dir: false, single: true }));
        resolver.answer(Conflict::Duplicate, false);
        assert_eq!(targets(resolver.step()), [("Copy of a".into(), Destination::Create), ("Copy of Copy of a".into(), Destination::Create)]);
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
