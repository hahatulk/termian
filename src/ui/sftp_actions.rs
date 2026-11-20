//! What the SFTP panels can do with files, as in the legacy panel: the
//! Actions and right-click menus, Rename / New folder / Delete dialogs,
//! copying to the other panel (menu or drag and drop) with the "File already
//! exists" question, and the transfer bar with its Transfers dialog.

use std::collections::VecDeque;

use gpui::{
    div, prelude::*, px, rgb, svg, AnyElement, App, Context, Div, ExternalPaths, FontWeight, KeyDownEvent, MouseButton, PathPromptOptions,
    Pixels, Point, Render, SharedString, Stateful, Window,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

use termian_native::edits::{open_path, Associations, EditId, Edited, Edits};
use termian_native::files::{delete_title, format_size, format_time, join, list_local, name_error, parent, Entry, PERMISSION_BITS};
use termian_native::remote_fs::{self, Operation, TransferId};
use termian_native::theme;
use termian_native::transfer::{Conflict, ConflictQuestion, Control, Destination, Direction, Item, Job, Planned, Progress, Resolver, Step, TransferEvent};

use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::pointer::PointerFeedback;
use super::sftp::{Side, SftpState};
use super::text_input::{self, Mode};
use crate::Shell;

const MENU_WIDTH: f32 = 230.;
const MENU_ROW: f32 = 32.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    /// Right click: the short list.
    Context,
    /// The Actions button: every item.
    Actions,
}

pub struct Menu {
    pub side: Side,
    pub position: Point<Pixels>,
    pub kind: MenuKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Open,
    OpenWith,
    CopyToTarget,
    Rename,
    Delete,
    Refresh,
    NewFolder,
    ToggleHidden,
    EditPermissions,
    SelectAll,
    Close,
}

pub enum Dialog {
    Rename { side: Side, from: String, input: String },
    NewFolder { side: Side, input: String },
    Delete { side: Side, items: Vec<Entry> },
    Permissions { side: Side, entry: Entry, mode: u32 },
    /// "File already exists" for the front of `SftpState::resolving`.
    Conflict,
    TransferError(Side, TransferId),
    Transfers,
    Error(String),
    /// Disconnecting this side would lose saved but not uploaded copies.
    EditingInProgress(Side),
}

/// A copy waiting for answers about name clashes.
pub struct Resolving {
    /// `None` until the destination folder has been read again.
    resolver: Option<Resolver>,
    items: Vec<Item>,
    /// The request for the destination's names, when it is remote.
    waiting: Option<u64>,
    direction: Direction,
    /// The connection that runs the transfers.
    conn: Side,
    /// Host-to-host: the files go through a temporary folder to this side.
    relay_to: Option<Side>,
    source_dir: String,
    destination_dir: String,
    question: Option<ConflictQuestion>,
    apply_all: bool,
}

pub struct TransferRow {
    pub id: TransferId,
    /// The connection that runs it (each numbers its transfers).
    pub conn: Side,
    label: String,
    is_dir: bool,
    direction: Direction,
    progress: Progress,
    failure: Option<String>,
    after: After,
}

/// What happens when a transfer ends, besides refreshing the destination.
enum After {
    Nothing,
    /// A remote file downloaded for Open: start the app.
    Open { edit: EditId, app: Option<PathBuf> },
    /// An edited copy went back to the server.
    Uploaded(EditId),
    /// Host-to-host, first half: downloaded into `temp`; now upload it to `to`.
    Relay { to: Side, temp: PathBuf, destination_dir: String, planned: Planned },
    /// Host-to-host, second half: the temporary copy can go.
    Cleanup(PathBuf),
}

/// What a row drag carries: the row under the pointer and its panel.
#[derive(Clone)]
pub struct DraggedRows {
    pub side: Side,
    name: String,
    count: usize,
}

struct DragPreview(String);

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .font_family(theme::UI_FONT)
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgb(theme::ACCENT))
            .text_sm()
            .text_color(rgb(0xffffff))
            .child(self.0.clone())
    }
}

fn other(side: Side) -> Side {
    side.other()
}

/// A folder of its own for one host-to-host file.
fn relay_dir() -> Result<PathBuf, String> {
    tempfile::Builder::new().prefix("termian-relay-").tempdir()
        .map(|dir| dir.keep()).map_err(|e| e.to_string())
}

impl SftpState {
    /// Handles a transfer event from a side's connection.
    pub fn on_transfer(&mut self, conn: Side, id: TransferId, event: TransferEvent) {
        let Some(row) = self.actions.transfers.iter_mut().find(|r| r.conn == conn && r.id == id) else {
            if matches!(event, TransferEvent::Finished | TransferEvent::Incomplete | TransferEvent::CommitFailed { .. } | TransferEvent::Cancelled) {
                if let Some(remote) = self.remote_mut(conn) {
                    remote.connection.finished(id);
                }
            }
            return;
        };
        match event {
            TransferEvent::Progress(progress) => {
                row.progress = progress;
                row.failure = None;
            }
            TransferEvent::Failed { error, .. } => row.failure = Some(error),
            TransferEvent::Finished | TransferEvent::Incomplete | TransferEvent::CommitFailed { .. } | TransferEvent::Cancelled => {
                let direction = row.direction;
                let finished = event == TransferEvent::Finished;
                let (label, is_dir) = (row.label.clone(), row.is_dir);
                let after = std::mem::replace(&mut row.after, After::Nothing);
                self.actions.transfers.retain(|r| !(r.conn == conn && r.id == id));
                let mut refresh = None;
                match after {
                    After::Nothing => {}
                    After::Open { edit, app } if finished => {
                        self.actions.edits.downloaded(edit);
                        let app = app.or_else(|| self.actions.edits.get(edit).and_then(|f| self.actions.associations.app_for(&f.name()).map(PathBuf::from)));
                        if let Some(file) = self.actions.edits.get(edit) {
                            if let Err(error) = open_path(&file.local_path, app.as_deref()) {
                                self.actions.dialog = Some(Dialog::Error(error));
                            }
                        }
                    }
                    After::Open { edit, .. } => self.actions.edits.discard(edit),
                    After::Uploaded(edit) => {
                        if finished {
                            self.actions.edits.uploaded(edit);
                        }
                    }
                    After::Relay { to, temp, destination_dir, planned } if finished => {
                        // The copy is here: send it on to the other host.
                        let source = Planned { item: Item { name: planned.target.clone(), is_dir: planned.item.is_dir }, ..planned };
                        let job = Job { direction: Direction::Upload, source_dir: temp.to_string_lossy().into_owned(), destination_dir, planned: source };
                        match self.remote_mut(to) {
                            Some(remote) => {
                                let id = remote.connection.transfer(job);
                                self.actions.transfers.push(TransferRow {
                                    id,
                                    conn: to,
                                    label,
                                    is_dir,
                                    direction: Direction::Upload,
                                    progress: Progress::default(),
                                    failure: None,
                                    after: After::Cleanup(temp),
                                });
                            }
                            None => {
                                let _ = std::fs::remove_dir_all(temp);
                            }
                        }
                    }
                    After::Relay { temp, .. } | After::Cleanup(temp) => {
                        let _ = std::fs::remove_dir_all(temp);
                        if finished {
                            refresh = Some(conn);
                        }
                    }
                }
                if let Some(remote) = self.remote_mut(conn) {
                    remote.connection.finished(id);
                }
                if matches!(self.actions.dialog, Some(Dialog::TransferError(side, open)) if side == conn && open == id)
                    || (matches!(self.actions.dialog, Some(Dialog::Transfers)) && self.actions.transfers.is_empty())
                {
                    self.actions.dialog = None;
                }
                match event {
                    TransferEvent::CommitFailed { error } => self.actions.dialog = Some(Dialog::Error(error)),
                    TransferEvent::Incomplete => self.actions.dialog = Some(Dialog::Error("Transfer was incomplete".into())),
                    _ => {}
                }
                // Uploads land on their host; downloads on the disk panel.
                let destination = refresh.unwrap_or(if direction == Direction::Upload { conn } else { conn.other() });
                if direction == Direction::Upload || !self.is_host(destination) {
                    self.reload(destination);
                }
            }
        }
    }

    pub fn on_remote_changed(&mut self, side: Side, result: Result<(), String>) {
        self.reload(side);
        if let Err(error) = result {
            self.actions.dialog = Some(Dialog::Error(error));
        }
    }
}

/// State of the actions; part of `SftpState`.
pub struct Actions {
    pub menu: Option<Menu>,
    pub dialog: Option<Dialog>,
    pub resolving: VecDeque<Resolving>,
    pub transfers: Vec<TransferRow>,
    /// Counter for [`RemoteFs::names`] requests.
    next_request: u64,
    /// Remote files opened in local apps.
    pub edits: Edits,
    pub associations: Associations,
    /// The loop that looks for saved copies is running.
    watching: bool,
    /// The changed-files list is expanded.
    edits_open: bool,
    /// Which side each opened remote file came from.
    edit_sides: std::collections::HashMap<EditId, Side>,
}

impl Actions {
    /// A side's connection closed: its transfers and pending copies stop.
    pub fn forget_side(&mut self, side: Side) {
        self.transfers.retain(|r| {
            let gone = r.conn == side || matches!(&r.after, After::Relay { to, .. } if *to == side);
            if gone {
                if let After::Relay { temp, .. } | After::Cleanup(temp) = &r.after {
                    let _ = std::fs::remove_dir_all(temp);
                }
            }
            !gone
        });
        self.resolving.retain(|c| c.conn != side && c.relay_to != Some(side));
        self.menu = None;
        self.dialog = None;
    }

    fn edits_of(&self, side: Side) -> Vec<EditId> {
        self.edits.changed().iter().map(|f| f.id).filter(|id| self.edit_sides.get(id) == Some(&side)).collect()
    }
}

impl Default for Actions {
    fn default() -> Self {
        Self {
            menu: None,
            dialog: None,
            resolving: VecDeque::new(),
            transfers: Vec::new(),
            next_request: 0,
            edits: Edits::in_temp_dir(),
            associations: Associations::load_default(),
            watching: false,
            edits_open: false,
            edit_sides: Default::default(),
        }
    }
}

impl Shell {
    /// Copies need both panels listing and at least one host (local, host or host-to-host).
    fn can_copy_between(&self) -> bool {
        self.sftp.is_ready(Side::Left) && self.sftp.is_ready(Side::Right) && (self.sftp.is_host(Side::Left) || self.sftp.is_host(Side::Right))
    }

    fn panel_dir(&self, side: Side) -> Option<String> {
        self.sftp.panel(side).map(|p| p.path.clone())
    }

    fn menu_items(&self, side: Side, kind: MenuKind) -> Vec<(Action, &'static str, bool, bool)> {
        let Some(panel) = self.sftp.panel(side) else { return Vec::new() };
        let selected = panel.selected.len();
        let can_copy = selected > 0 && self.can_copy_between();
        let hidden = if panel.show_hidden { "Hide Hidden Files" } else { "Show Hidden Files" };
        // (action, title, red, disabled) in the legacy order.
        let mut items = vec![
            (Action::Open, "Open", false, selected == 0),
            (Action::OpenWith, "Open with...", false, selected == 0),
            (Action::CopyToTarget, "Copy to target directory", false, !can_copy),
            (Action::Rename, "Rename", false, selected != 1),
            (Action::Delete, "Delete", true, selected == 0),
            (Action::Refresh, "Refresh", false, false),
            (Action::NewFolder, "New Folder", false, false),
            (Action::ToggleHidden, hidden, false, false),
            (Action::EditPermissions, "Edit Permissions", false, selected != 1),
            (Action::SelectAll, "Select All", false, false),
        ];
        // Open works on one file: with folders or several rows it is not offered.
        if selected > 1 || panel.selected_entries().iter().any(|e| e.is_dir) {
            items.retain(|(action, ..)| !matches!(action, Action::Open | Action::OpenWith));
        }
        // Windows has no Unix permission bits, as in the legacy local panel.
        if cfg!(windows) && !self.sftp.is_host(side) {
            items.retain(|(action, ..)| *action != Action::EditPermissions);
        }
        if self.sftp.is_host(side) {
            items.push((Action::Close, "Close", true, false));
        }
        if kind == MenuKind::Context {
            items.retain(|(action, ..)| {
                matches!(
                    action,
                    Action::Open
                        | Action::OpenWith
                        | Action::CopyToTarget
                        | Action::Rename
                        | Action::Delete
                        | Action::Refresh
                        | Action::NewFolder
                        | Action::EditPermissions
                )
            });
        }
        items
    }

    pub fn open_menu(&mut self, side: Side, position: Point<Pixels>, kind: MenuKind, cx: &mut Context<Self>) {
        self.sftp.actions.menu = Some(Menu { side, position, kind });
        cx.notify();
    }

    fn run_action(&mut self, side: Side, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        self.sftp.actions.menu = None;
        let Some(panel) = self.sftp.panel_mut(side) else { return };
        let (selected, first, show_hidden) = (panel.selected_entries(), panel.selected.first().cloned(), panel.show_hidden);
        match action {
            Action::Open => {
                if let Some(entry) = selected.into_iter().next() {
                    self.open_entry(side, &entry, None, window, cx);
                }
            }
            Action::OpenWith => {
                if let Some(entry) = selected.into_iter().next() {
                    self.open_entry_with(side, entry, window, cx);
                }
            }
            Action::CopyToTarget => self.start_copy(side, selected, window, cx),
            Action::Rename => {
                if let Some(from) = first {
                    self.open_dialog(Dialog::Rename { side, input: from.clone(), from }, window);
                }
            }
            Action::Delete => self.open_dialog(Dialog::Delete { side, items: selected }, window),
            Action::Refresh => {
                panel.clear_selection();
                self.sftp.reload(side);
            }
            Action::NewFolder => self.open_dialog(Dialog::NewFolder { side, input: String::new() }, window),
            Action::ToggleHidden => {
                // One setting for both panels, as in the legacy store.
                for side in [Side::Left, Side::Right] {
                    self.sftp.side_mut(side).local.show_hidden = !show_hidden;
                    if let Some(panel) = self.sftp.panel_mut(side) {
                        panel.show_hidden = !show_hidden;
                    }
                }
            }
            Action::EditPermissions => {
                if let Some(entry) = selected.into_iter().next() {
                    let mode = entry.mode.unwrap_or(0);
                    self.open_dialog(Dialog::Permissions { side, entry, mode }, window);
                }
            }
            Action::SelectAll => panel.select_all(),
            Action::Close => self.request_disconnect(side, window),
        }
        cx.notify();
    }

    /// Disconnects a side, first asking when its saved copies were not uploaded yet.
    pub fn request_disconnect(&mut self, side: Side, window: &mut Window) {
        if self.sftp.actions.edits_of(side).is_empty() {
            self.disconnect_sftp(side);
        } else {
            self.open_dialog(Dialog::EditingInProgress(side), window);
        }
    }

    /// Open: a local file in its app; a remote one after a download to a temporary copy.
    pub fn open_entry(&mut self, side: Side, entry: &Entry, app: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dir) = self.panel_dir(side) else { return };
        let app = app.or_else(|| self.sftp.actions.associations.app_for(&entry.name).map(PathBuf::from));
        #[cfg(windows)]
        if self.sftp.is_host(side) && app.is_none() {
            return self.open_entry_with(side, entry.clone(), window, cx);
        }
        match self.sftp.is_host(side) {
            false => {
                if let Err(error) = open_path(Path::new(&join(&dir, &entry.name)), app.as_deref()) {
                    self.open_dialog(Dialog::Error(error), window);
                }
            }
            true => {
                let file = match self.sftp.actions.edits.prepare(&join(&dir, &entry.name)) {
                    Ok(file) => file.clone(),
                    Err(error) => return self.open_dialog(Dialog::Error(error), window),
                };
                self.sftp.actions.edit_sides.insert(file.id, side);
                let Some(remote) = self.sftp.remote_mut(side) else { return };
                let item = Item { name: entry.name.clone(), is_dir: false };
                let planned = Planned { target: entry.name.clone(), item, destination: Destination::Create };
                let job = Job { direction: Direction::Download, source_dir: dir, destination_dir: file.local_dir(), planned };
                let id = remote.connection.transfer(job);
                self.sftp.actions.transfers.push(TransferRow {
                    id,
                    conn: side,
                    label: entry.name.clone(),
                    is_dir: false,
                    direction: Direction::Download,
                    progress: Progress::default(),
                    failure: None,
                    after: After::Open { edit: file.id, app },
                });
                self.watch_edits(cx);
            }
        }
    }

    /// Open with…: asks for an app, remembers it for the extension, then opens.
    fn open_entry_with(&mut self, side: Side, entry: Entry, window: &mut Window, cx: &mut Context<Self>) {
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose application".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = answer.await else { return };
            let Some(app) = paths.into_iter().next() else { return };
            let _ = this.update_in(cx, |shell, window, cx| {
                if let Err(error) = shell.sftp.actions.associations.remember(&entry.name, app.clone()) {
                    shell.open_dialog(Dialog::Error(error), window);
                }
                shell.open_entry(side, &entry, Some(app), window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Looks for saved copies once a second while any are open.
    fn watch_edits(&mut self, cx: &mut Context<Self>) {
        if self.sftp.actions.watching {
            return;
        }
        self.sftp.actions.watching = true;
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            let running = this.update(cx, |shell, cx| {
                let edits = &mut shell.sftp.actions.edits;
                if edits.poll() {
                    cx.notify();
                }
                if edits.is_empty() && shell.sftp.actions.transfers.is_empty() {
                    shell.sftp.actions.watching = false;
                }
                shell.sftp.actions.watching
            });
            if !matches!(running, Ok(true)) {
                break;
            }
        })
        .detach();
    }

    /// UPLOAD: sends a saved copy back over the remote file.
    fn upload_edit(&mut self, edit: EditId) {
        let side = self.sftp.actions.edit_sides.get(&edit).copied().unwrap_or(Side::Right);
        let Some(file) = self.sftp.actions.edits.get(edit).cloned() else { return };
        let Some(remote) = self.sftp.remote_mut(side) else { return };
        let Some(destination_dir) = parent(&file.remote_path) else { return };
        let name = file.name();
        let item = Item { name: name.clone(), is_dir: false };
        let job = Job {
            direction: Direction::Upload,
            source_dir: file.local_dir(),
            destination_dir,
            planned: Planned { target: name.clone(), item, destination: Destination::Replace },
        };
        let id = remote.connection.transfer(job);
        self.sftp.actions.transfers.push(TransferRow {
            id,
            conn: side,
            label: name,
            is_dir: false,
            direction: Direction::Upload,
            progress: Progress::default(),
            failure: None,
            after: After::Uploaded(edit),
        });
    }

    fn open_dialog(&mut self, dialog: Dialog, window: &mut Window) {
        self.sftp.actions.dialog = Some(dialog);
        window.focus(&self.sftp_dialog_focus);
    }

    /// Runs file operations on a side, then refreshes it; failures open the Error dialog.
    fn operate(&mut self, side: Side, operations: Vec<Operation>, cx: &mut Context<Self>) {
        match self.sftp.is_host(side) {
            true => {
                if let Some(remote) = self.sftp.remote(side) {
                    remote.connection.operate(operations);
                }
            }
            false => {
                let task = cx.background_spawn(async move { remote_fs::operate_locally(operations) });
                cx.spawn(async move |this, cx| {
                    let result = task.await;
                    let _ = this.update(cx, |shell, cx| {
                        shell.sftp.reload(side);
                        if let Err(error) = result {
                            shell.sftp.actions.dialog = Some(Dialog::Error(error));
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
        }
    }

    fn confirm_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = self.sftp.actions.dialog.take() else { return };
        match dialog {
            Dialog::Rename { side, from, input } => {
                let (Some(dir), Some(panel)) = (self.panel_dir(side), self.sftp.panel(side)) else { return };
                if name_error(&input, &panel.names(), Some(&from)).is_some() {
                    self.sftp.actions.dialog = Some(Dialog::Rename { side, from, input });
                    return;
                }
                if input != from {
                    self.operate(side, vec![Operation::Rename { from: join(&dir, &from), to: join(&dir, &input) }], cx);
                }
            }
            Dialog::NewFolder { side, input } => {
                let (Some(dir), Some(panel)) = (self.panel_dir(side), self.sftp.panel(side)) else { return };
                if name_error(&input, &panel.names(), None).is_some() {
                    self.sftp.actions.dialog = Some(Dialog::NewFolder { side, input });
                    return;
                }
                self.operate(side, vec![Operation::MakeDir(join(&dir, &input))], cx);
            }
            Dialog::Delete { side, items } => {
                let Some(dir) = self.panel_dir(side) else { return };
                let operations =
                    items.iter().map(|e| Operation::Remove { path: join(&dir, &e.name), is_dir: e.is_dir && !e.is_link }).collect();
                if let Some(panel) = self.sftp.panel_mut(side) {
                    panel.clear_selection();
                }
                self.operate(side, operations, cx);
            }
            Dialog::Permissions { side, entry, mode } => {
                let Some(dir) = self.panel_dir(side) else { return };
                self.operate(side, vec![Operation::Chmod { path: join(&dir, &entry.name), mode }], cx);
            }
            Dialog::EditingInProgress(side) => {
                for edit in self.sftp.actions.edits_of(side) {
                    self.sftp.actions.edits.discard(edit);
                }
                self.disconnect_sftp(side);
            }
            // These dialogs have no default button: Enter leaves them open.
            dialog @ (Dialog::Conflict | Dialog::TransferError(..)) => self.sftp.actions.dialog = Some(dialog),
            Dialog::Transfers | Dialog::Error(_) => {}
        }
        cx.notify();
    }

    fn close_dialog(&mut self, cx: &mut Context<Self>) {
        if matches!(self.sftp.actions.dialog, Some(Dialog::Conflict)) {
            // Stop: the whole copy is called off.
            self.sftp.actions.resolving.pop_front();
        }
        self.sftp.actions.dialog = None;
        cx.notify();
    }

    fn on_dialog_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if super::keyboard::tab(event, window, cx, Some(&self.sftp_dialog_focus)) {
            return;
        }
        if !self.sftp_dialog_focus.is_focused(window) {
            if event.keystroke.key == "escape" {
                self.close_dialog(cx);
                cx.stop_propagation();
            }
            return;
        }
        let input = match &mut self.sftp.actions.dialog {
            Some(Dialog::Rename { input, .. } | Dialog::NewFolder { input, .. }) => Some(input),
            _ => None,
        };
        match text_input::edit("dialog-input", input, &event.keystroke, Mode::Line, cx) {
            Key::Edited => {}
            Key::Enter => self.confirm_dialog(window, cx),
            Key::Escape => self.close_dialog(cx),
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Copies entries of one panel into the folder open in the other: up to
    /// a host, down to the disk, or host to host through a temporary copy.
    pub fn start_copy(&mut self, side: Side, items: Vec<Entry>, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(source_dir), Some(destination)) = (self.panel_dir(side), self.sftp.panel(other(side))) else { return };
        if items.is_empty() || !self.can_copy_between() {
            return;
        }
        let items = items.into_iter().map(|e| Item { name: e.name, is_dir: e.is_dir }).collect();
        let destination_dir = destination.path.clone();
        let (conn, direction, relay_to) = match (self.sftp.is_host(side), self.sftp.is_host(other(side))) {
            (false, _) => (other(side), Direction::Upload, None),
            (true, false) => (side, Direction::Download, None),
            (true, true) => (side, Direction::Download, Some(other(side))),
        };
        self.queue_copy(items, conn, direction, relay_to, source_dir, destination_dir, window, cx);
    }

    /// Queues a copy. Like the legacy panel, clashes are checked against the
    /// destination folder as it is now, not as it was last listed.
    #[allow(clippy::too_many_arguments)]
    fn queue_copy(&mut self, items: Vec<Item>, conn: Side, direction: Direction, relay_to: Option<Side>, source_dir: String, destination_dir: String, window: &mut Window, cx: &mut Context<Self>) {
        let mut copy = Resolving { resolver: None, items, waiting: None, direction, conn, relay_to, source_dir, destination_dir, question: None, apply_all: false };
        // The side the files land on: the host uploaded to, the relay's host, or the disk.
        let destination = relay_to.unwrap_or(if direction == Direction::Upload { conn } else { conn.other() });
        match self.sftp.is_host(destination) {
            true => {
                self.sftp.actions.next_request += 1;
                let request = self.sftp.actions.next_request;
                let Some(remote) = self.sftp.remote(destination) else { return };
                remote.connection.names(request, &copy.destination_dir);
                copy.waiting = Some(request);
            }
            false => match list_local(&copy.destination_dir) {
                Ok(entries) => {
                    let names: Vec<String> = entries.into_iter().map(|e| e.name).collect();
                    copy.resolver = Some(Resolver::new(std::mem::take(&mut copy.items), &names));
                }
                Err(error) => return self.open_dialog(Dialog::Error(error), window),
            },
        }
        self.sftp.actions.resolving.push_back(copy);
        self.advance_copies(window, cx);
    }

    /// The destination of a queued upload has been read.
    pub fn on_names(&mut self, request: u64, result: Result<Vec<String>, String>, window: &mut Window, cx: &mut Context<Self>) {
        let queue = &mut self.sftp.actions.resolving;
        let Some(index) = queue.iter().position(|c| c.waiting == Some(request)) else { return };
        match result {
            Ok(names) => {
                let copy = &mut queue[index];
                copy.waiting = None;
                copy.resolver = Some(Resolver::new(std::mem::take(&mut copy.items), &names));
            }
            Err(error) => {
                queue.remove(index);
                self.open_dialog(Dialog::Error(error), window);
            }
        }
        self.advance_copies(window, cx);
    }

    /// Plans queued copies until one needs an answer; starts the planned items.
    fn advance_copies(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while let Some(front) = self.sftp.actions.resolving.front_mut() {
            let Some(resolver) = &mut front.resolver else { break };
            match resolver.step() {
                Step::Ask(question) => {
                    front.question = Some(question);
                    front.apply_all = false;
                    self.open_dialog(Dialog::Conflict, window);
                    return;
                }
                Step::Done(planned) => {
                    let Some(copy) = self.sftp.actions.resolving.pop_front() else { return };
                    for planned in planned {
                        let label = planned.target.clone();
                        let is_dir = planned.item.is_dir;
                        // Host to host: download into a folder of its own first.
                        let (destination_dir, after) = match copy.relay_to {
                            Some(to) => match relay_dir() {
                                Ok(temp) => (
                                    temp.to_string_lossy().into_owned(),
                                    After::Relay { to, temp, destination_dir: copy.destination_dir.clone(), planned: planned.clone() },
                                ),
                                Err(error) => {
                                    self.open_dialog(Dialog::Error(error), window);
                                    continue;
                                }
                            },
                            None => (copy.destination_dir.clone(), After::Nothing),
                        };
                        // The temporary folder is empty: the first half never replaces anything.
                        let planned = if copy.relay_to.is_some() { Planned { destination: Destination::Create, ..planned } } else { planned };
                        let job = Job { direction: copy.direction, source_dir: copy.source_dir.clone(), destination_dir, planned };
                        let Some(remote) = self.sftp.remote_mut(copy.conn) else { continue };
                        let id = remote.connection.transfer(job);
                        self.sftp.actions.transfers.push(TransferRow {
                            id,
                            conn: copy.conn,
                            label,
                            is_dir,
                            direction: copy.direction,
                            progress: Progress::default(),
                            failure: None,
                            after,
                        });
                    }
                }
            }
        }
        if matches!(self.sftp.actions.dialog, Some(Dialog::Conflict)) {
            self.sftp.actions.dialog = None;
        }
        cx.notify();
    }

    fn answer_conflict(&mut self, conflict: Conflict, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(front) = self.sftp.actions.resolving.front_mut() {
            if let Some(resolver) = &mut front.resolver {
                resolver.answer(conflict, front.apply_all);
            }
            front.question = None;
        }
        self.advance_copies(window, cx);
    }

    fn control_transfer(&mut self, conn: Side, id: TransferId, control: Control) {
        if let Some(remote) = self.sftp.remote_mut(conn) {
            remote.connection.control(id, control);
        }
        if let Some(row) = self.sftp.actions.transfers.iter_mut().find(|r| r.conn == conn && r.id == id) {
            row.failure = None;
        }
        if matches!(self.sftp.actions.dialog, Some(Dialog::TransferError(side, open)) if side == conn && open == id) {
            self.sftp.actions.dialog = None;
        }
    }

    /// Drop of rows from the other panel.
    pub fn drop_rows(&mut self, dragged: &DraggedRows, target: Side, window: &mut Window, cx: &mut Context<Self>) {
        if dragged.side == target {
            return;
        }
        let Some(panel) = self.sftp.panel(dragged.side) else { return };
        let items = if panel.is_selected(&dragged.name) {
            panel.selected_entries()
        } else {
            panel.entry(&dragged.name).cloned().into_iter().collect()
        };
        self.start_copy(dragged.side, items, window, cx);
    }

    /// Files dragged in from Finder or Explorer onto a host panel.
    pub fn drop_paths(&mut self, side: Side, paths: &ExternalPaths, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sftp.is_host(side) {
            return;
        }
        let Some(destination) = self.sftp.panel(side) else { return };
        let destination_dir = destination.path.clone();
        // One copy per source folder, since a copy reads from one folder.
        let mut groups: Vec<(String, Vec<Item>)> = Vec::new();
        for path in paths.paths() {
            let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { continue };
            let (parent, name) = (parent.to_string_lossy().into_owned(), name.to_string_lossy().into_owned());
            let item = Item { name, is_dir: path.is_dir() };
            match groups.iter_mut().find(|(dir, _)| *dir == parent) {
                Some((_, items)) => items.push(item),
                None => groups.push((parent, vec![item])),
            }
        }
        for (source_dir, items) in groups {
            self.queue_copy(items, side, Direction::Upload, None, source_dir, destination_dir.clone(), window, cx);
        }
    }

    pub fn drag_value(&self, side: Side, name: &str) -> DraggedRows {
        let count = self.sftp.panel(side).filter(|p| p.is_selected(name)).map(|p| p.selected.len()).unwrap_or(1);
        DraggedRows { side, name: name.to_owned(), count }
    }

    pub fn drag_preview(value: &DraggedRows, cx: &mut App) -> gpui::Entity<impl Render> {
        let label = if value.count > 1 { format!("{} items", value.count) } else { value.name.clone() };
        cx.new(|_| DragPreview(label))
    }

    /// The Actions button in a panel header.
    pub fn render_actions_button(&self, side: Side, cx: &mut Context<Self>) -> Stateful<Div> {
        let open = self.sftp.actions.menu.as_ref().is_some_and(|m| m.side == side && m.kind == MenuKind::Actions);
        div()
            .id(SharedString::from(format!("actions-{}", side_key(side))))
            .h(px(28.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .rounded_md()
            .text_sm()
            .text_color(rgb(if open { theme::ACCENT } else { theme::TEXT }))
            .icon_feedback()
            .child("Actions")
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT)).when(open, |s| s.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5)))))
            .on_click(cx.listener(move |shell, event: &gpui::ClickEvent, _, cx| {
                let at = event.position();
                shell.open_menu(side, Point { x: at.x - px(MENU_WIDTH - 40.), y: at.y + px(14.) }, MenuKind::Actions, cx);
            }))
    }

    /// Menus and dialogs, drawn over the whole window.
    pub fn render_sftp_overlay(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if let Some(dialog) = &self.sftp.actions.dialog {
            let card = self.render_dialog(dialog, window, cx);
            return Some(
                dialog::backdrop(card)
                    .track_focus(&self.sftp_dialog_focus)
                    .tab_group()
                    .on_key_down(cx.listener(Self::on_dialog_key))
                    .into_any_element(),
            );
        }
        let menu = self.sftp.actions.menu.as_ref()?;
        let items = self.menu_items(menu.side, menu.kind);
        let viewport = window.viewport_size();
        let height = px(items.len() as f32 * MENU_ROW + 8.);
        let left = menu.position.x.min(viewport.width - px(MENU_WIDTH + 8.)).max(px(8.));
        let top = menu.position.y.min(viewport.height - height - px(8.)).max(px(8.));
        let side = menu.side;
        let rows = items.into_iter().map(|(action, title, red, disabled)| {
            div()
                .id(SharedString::from(format!("menu-{title}")))
                .h(px(MENU_ROW))
                .px_3()
                .flex()
                .items_center()
                .rounded_sm()
                .text_sm()
                .text_color(rgb(match (disabled, red) {
                    (true, _) => theme::TEXT_MUTED,
                    (false, true) => theme::ERROR,
                    (false, false) => theme::TEXT,
                }))
                .when(!disabled, |d| d.menu_feedback(red))
                .child(title)
                .when(!disabled, |d| {
                    d.on_click(cx.listener(move |shell, _, window, cx| {
                        cx.stop_propagation();
                        shell.run_action(side, action, window, cx);
                    }))
                })
        });
        Some(
            div()
                .id("menu-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                    shell.sftp.actions.menu = None;
                    cx.notify();
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(|shell, _, _, cx| {
                    shell.sftp.actions.menu = None;
                    cx.notify();
                }))
                .child(super::motion::popover(
                    div()
                        .id("menu")
                        .absolute()
                        .left(left)
                        .top(top)
                        .w(px(MENU_WIDTH))
                        .p_1()
                        .rounded_md()
                        .bg(rgb(theme::SURFACE))
                        .border_1()
                        .border_color(rgb(theme::SURFACE_ACTIVE))
                        .shadow_lg()
                        // Clicks on the menu itself must not dismiss it before the item runs.
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .children(rows),
                    "sftp-menu-enter",
                ))
                .into_any_element(),
        )
    }

    fn render_dialog(&self, dialog: &Dialog, window: &Window, cx: &mut Context<Self>) -> Div {
        let close = dialog::close_button("dialog-close").on_click(cx.listener(|shell, _, _, cx| shell.close_dialog(cx)));
        let confirm = |id: &str, label: &str, kind: Kind, disabled: bool| {
            let button = dialog::button(id, label, kind, disabled);
            if disabled { button } else { button.on_click(cx.listener(|shell, _, window, cx| shell.confirm_dialog(window, cx))) }
        };
        let focused = self.sftp_dialog_focus.is_focused(window);
        match dialog {
            Dialog::Rename { side, from, input } => {
                let taken = self.sftp.panel(*side).map(|p| p.names()).unwrap_or_default();
                let error = name_error(input, &taken, Some(from));
                dialog::card("Rename", false, close, dialog::field("New filename *", input, error, &self.sftp_dialog_focus, focused, cx), vec![
                    confirm("dialog-confirm", "Confirm", Kind::Accent, error.is_some()).into_any_element(),
                ])
            }
            Dialog::NewFolder { side, input } => {
                let taken = self.sftp.panel(*side).map(|p| p.names()).unwrap_or_default();
                let error = name_error(input, &taken, None);
                dialog::card("New folder", false, close, dialog::field("Foldername *", input, error, &self.sftp_dialog_focus, focused, cx), vec![
                    confirm("dialog-confirm", "Confirm", Kind::Accent, error.is_some()).into_any_element(),
                ])
            }
            Dialog::Delete { items, .. } => {
                let names = items.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join("\n");
                dialog::card(&delete_title(items), false, close, dialog::text(&names), vec![
                    confirm("dialog-confirm", "Delete", Kind::Error, false).into_any_element(),
                ])
            }
            Dialog::Permissions { side, entry, mode } => self.render_permissions(*side, entry, *mode, close, cx),
            Dialog::Conflict => self.render_conflict(close, cx),
            Dialog::TransferError(conn, id) => {
                let (conn, id) = (*conn, *id);
                let error = self.sftp.actions.transfers.iter().find(|r| r.conn == conn && r.id == id).and_then(|r| r.failure.clone()).unwrap_or_default();
                let send = |control: Control| cx.listener(move |shell: &mut Shell, _, _, cx| {
                    shell.control_transfer(conn, id, control);
                    cx.notify();
                });
                dialog::card("Error during transfer", true, close, dialog::text(&format!("{error} What do you want to do?")), vec![
                    dialog::button("transfer-cancel", "Cancel transfer", Kind::Error, false).on_click(send(Control::Cancel)).into_any_element(),
                    div().flex_1().into_any_element(),
                    dialog::button("transfer-skip-all", "Skip all", Kind::Text, false).on_click(send(Control::SkipAll)).into_any_element(),
                    dialog::button("transfer-skip", "Skip", Kind::Text, false).on_click(send(Control::Skip)).into_any_element(),
                    dialog::button("transfer-retry", "Retry", Kind::Accent, false).on_click(send(Control::Retry)).into_any_element(),
                ])
            }
            Dialog::Transfers => {
                let rows = self.sftp.actions.transfers.iter().rev().map(|row| self.render_transfer(row, true, cx));
                let body = div().id("transfer-list").max_h(window.viewport_size().height * 0.5).overflow_y_scroll().flex().flex_col().gap(px(6.)).children(rows);
                dialog::card("Transfers", false, close, div().child(body), vec![
                    dialog::button("transfers-discard", "Discard all", Kind::Error, false)
                        .on_click(cx.listener(|shell, _, _, cx| {
                            let ids: Vec<_> = shell.sftp.actions.transfers.iter().map(|r| (r.conn, r.id)).collect();
                            for (conn, id) in ids {
                                shell.control_transfer(conn, id, Control::Cancel);
                            }
                            shell.sftp.actions.dialog = None;
                            cx.notify();
                        }))
                        .into_any_element(),
                    div().flex_1().into_any_element(),
                    dialog::button("transfers-close", "Close", Kind::Accent, false).on_click(cx.listener(|shell, _, _, cx| shell.close_dialog(cx))).into_any_element(),
                ])
            }
            Dialog::Error(error) => dialog::card("Error", false, close, dialog::text(error), Vec::new()),
            Dialog::EditingInProgress(side) => {
                let count = self.sftp.actions.edits_of(*side).len();
                let files = if count == 1 { "1 file".to_owned() } else { format!("{count} files") };
                let host = self.sftp.remote(*side).map(|r| r.label.clone()).unwrap_or_default();
                let text = format!(
                    "There {} currently {files} being edited on {host}. Do you want to discard {} and connect to a different server?",
                    if count == 1 { "is" } else { "are" },
                    if count == 1 { "the file" } else { "all files" },
                );
                dialog::card("File editing is in progress", false, close, dialog::text(&text), vec![
                    confirm("dialog-confirm", &format!("Discard {files}"), Kind::Error, false).into_any_element(),
                ])
            }
        }
    }

    /// Legacy "Edit permissions": path, the read/write/execute grid and ownership.
    fn render_permissions(&self, side: Side, entry: &Entry, mode: u32, close: Stateful<Div>, cx: &mut Context<Self>) -> Div {
        let path = self.panel_dir(side).map(|dir| join(&dir, &entry.name)).unwrap_or_default();
        let row = || div().w_full().flex().items_center().py(px(11.)).border_b_1().border_color(rgb(theme::SURFACE_ACTIVE));
        let name = |text: &str, bold: bool| {
            div()
                .w(gpui::relative(0.34))
                .when(bold, |d| d.font_weight(FontWeight::BOLD))
                .when(!bold, |d| d.text_color(rgb(theme::TEXT_MUTED)))
                .child(text.to_owned())
        };
        let column = || div().w(gpui::relative(0.22)).flex().justify_end();
        let mut grid = div().mb(px(20.)).flex().flex_col().child(
            row()
                .pt_0()
                .child(name("File Access", true))
                .children(["Read", "Write", "Execute"].map(|title| column().font_weight(FontWeight::BOLD).child(title))),
        );
        for (who, bits) in ["Owner", "Groups", "Others"].into_iter().zip(PERMISSION_BITS) {
            grid = grid.child(row().when(who == "Others", |d| d.border_b_0()).child(name(who, false)).children(bits.map(|bit| {
                column().child(dialog::checkbox(&format!("perm-{bit:o}"), "", mode & bit != 0).on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(Dialog::Permissions { mode, .. }) = &mut shell.sftp.actions.dialog {
                        *mode ^= bit;
                    }
                    cx.notify();
                })))
            })));
        }
        let (user, group) = entry.owner.clone().unwrap_or_default();
        let ownership = div()
            .flex()
            .flex_col()
            .child(row().pt_0().font_weight(FontWeight::BOLD).child("Ownership"))
            .child(row().justify_between().text_color(rgb(theme::TEXT_MUTED)).child("User").child(user))
            .child(row().justify_between().text_color(rgb(theme::TEXT_MUTED)).child("Group").child(group));
        let body = div().flex().flex_col().child(div().mb(px(20.)).child(path)).child(grid).child(ownership);
        dialog::card("Edit permissions", false, close, body, vec![
            dialog::button("dialog-confirm", "Save", Kind::Accent, false)
                .on_click(cx.listener(|shell, _, window, cx| shell.confirm_dialog(window, cx)))
                .into_any_element(),
        ])
    }

    fn render_conflict(&self, close: Stateful<Div>, cx: &mut Context<Self>) -> Div {
        let Some(Resolving { question: Some(question), apply_all, .. }) = self.sftp.actions.resolving.front() else {
            return dialog::card("File already exists", false, close, div(), Vec::new());
        };
        let answer = |conflict: Conflict| cx.listener(move |shell: &mut Shell, _, window, cx| shell.answer_conflict(conflict, window, cx));
        let mut body = dialog::text(&format!(
            "An item named \"{}\" already exists in this location.\nDo you want to replace it with the one you are moving?",
            question.name
        ));
        if !question.single {
            body = div().child(body).child(div().mt(px(25.)).child(dialog::checkbox("apply-all", "Apply to All", *apply_all).on_click(
                cx.listener(|shell, _, _, cx| {
                    if let Some(front) = shell.sftp.actions.resolving.front_mut() {
                        front.apply_all = !front.apply_all;
                    }
                    cx.notify();
                }),
            )));
        }
        let mut buttons = vec![
            dialog::button("conflict-stop", "Stop", Kind::Error, false).on_click(cx.listener(|shell, _, _, cx| shell.close_dialog(cx))).into_any_element(),
            div().flex_1().into_any_element(),
            dialog::button("conflict-skip", "Skip", Kind::Text, false).on_click(answer(Conflict::Skip)).into_any_element(),
            dialog::button("conflict-replace", "Replace", Kind::Regular, false).on_click(answer(Conflict::Replace)).into_any_element(),
            dialog::button("conflict-duplicate", "Duplicate", Kind::Accent, false).on_click(answer(Conflict::Duplicate)).into_any_element(),
        ];
        if question.is_dir {
            buttons.push(dialog::button("conflict-merge", "Merge", Kind::Regular, false).on_click(answer(Conflict::Merge)).into_any_element());
        }
        dialog::card("File already exists", false, close, body, buttons)
    }

    /// One transfer: icon, path and numbers, then the bar or the failure, Retry and Discard.
    fn render_transfer(&self, row: &TransferRow, in_dialog: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let (id, conn) = (row.id, row.conn);
        let link = |key: &str, label: &str| {
            div()
                .id(SharedString::from(format!("{key}-{}-{id}-{in_dialog}", side_key(conn))))
                .px_2()
                .py_1()
                .rounded_sm()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(theme::TEXT))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(label.to_owned())
        };
        let percentage = row.progress.percentage().clamp(0., 100.);
        let end = match &row.failure {
            Some(error) => div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!("failed-{}-{id}-{in_dialog}", side_key(conn))))
                        .rounded_sm().icon_feedback()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(rgb(theme::ERROR))
                        .child(svg().path(icon("alert")).size(px(14.)).text_color(rgb(theme::ERROR)))
                        .child(if row.direction == Direction::Upload { "Upload failed" } else { "Download failed" })
                        .tooltip({
                            let error = error.clone();
                            move |_, cx| cx.new(|_| DragPreview(error.clone())).into()
                        })
                        .on_click(cx.listener(move |shell, _, window, cx| {
                            shell.open_dialog(Dialog::TransferError(conn, id), window);
                            cx.notify();
                        })),
                )
                .child(link("retry", "Retry").on_click(cx.listener(move |shell, _, _, cx| {
                    shell.control_transfer(conn, id, Control::Retry);
                    cx.notify();
                }))),
            // Legacy bar: 30% of the row, between 80 and 300 px.
            None => div().w(gpui::relative(0.3)).min_w(px(80.)).max_w(px(300.)).flex_none().px(px(10.)).child(
                div()
                    .h(px(4.))
                    .rounded_full()
                    .bg(rgb(theme::SURFACE_ACTIVE))
                    .child(div().h_full().rounded_full().bg(rgb(theme::ACCENT)).w(gpui::relative(percentage / 100.))),
            ),
        };
        div()
            .id(SharedString::from(format!("transfer-{}-{id}-{in_dialog}", side_key(conn))))
            .h(px(40.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .when(in_dialog, |d| d.rounded_lg().bg(rgb(theme::DIALOG_ROW)))
            .child(svg().path(icon(if row.is_dir { "folder" } else { "file" })).size(px(18.)).flex_none().text_color(rgb(if row.is_dir { super::sftp::FOLDER } else { theme::TEXT })))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(div().flex_none().max_w(gpui::relative(0.5)).truncate().text_color(rgb(theme::TEXT)).child(row.label.clone()))
                    .when(row.failure.is_none(), |d| {
                        d.child(div().flex_1().min_w_0().truncate().text_color(rgb(theme::TEXT_MUTED)).child(row.progress.summary(row.is_dir)))
                    }),
            )
            .child(end)
            .child(link("discard", "Discard").on_click(cx.listener(move |shell, _, _, cx| {
                shell.control_transfer(conn, id, Control::Cancel);
                cx.notify();
            })))
    }

    /// Saved copies of opened remote files, with UPLOAD and Discard.
    pub fn render_edits_bar(&self, cx: &mut Context<Self>) -> Option<Div> {
        let changed = self.sftp.actions.edits.changed();
        if changed.is_empty() {
            return None;
        }
        let link = |id: String, label: &str, accent: bool| {
            div()
                .id(SharedString::from(id))
                .px_2()
                .py_1()
                .rounded_sm()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if accent { theme::ACCENT } else { theme::TEXT }))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(label.to_owned())
        };
        let row = |file: &Edited| {
            let id = file.id;
            div()
                .h(px(40.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap_2()
                .rounded_md()
                .bg(rgb(theme::DIALOG_ROW))
                .text_xs()
                .child(svg().path(icon("file")).size(px(18.)).flex_none().text_color(rgb(theme::TEXT)))
                .child(div().flex_1().min_w_0().truncate().text_color(rgb(theme::TEXT)).child(file.name()))
                .child(div().flex_none().text_color(rgb(theme::TEXT_MUTED)).child(format_size(file.size())))
                .child(div().flex_none().text_color(rgb(theme::TEXT_MUTED)).child(file.modified().map(format_time).unwrap_or_default()))
                .child(link(format!("edit-upload-{id}"), "UPLOAD", true).on_click(cx.listener(move |shell, _, _, cx| {
                    shell.upload_edit(id);
                    cx.notify();
                })))
                .child(link(format!("edit-discard-{id}"), "Discard", false).on_click(cx.listener(move |shell, _, _, cx| {
                    shell.sftp.actions.edits.discard(id);
                    cx.notify();
                })))
        };
        let bar = div().flex_none().p(px(10.)).flex().flex_col().gap(px(6.)).bg(rgb(theme::SURFACE)).border_t_1().border_color(rgb(theme::TITLE_BAR));
        if let [file] = changed.as_slice() {
            return Some(bar.child(row(file)));
        }
        let ids: Vec<EditId> = changed.iter().map(|f| f.id).collect();
        let open = self.sftp.actions.edits_open;
        let header = div()
            .flex()
            .items_center()
            .text_xs()
            .text_color(rgb(theme::TEXT))
            .child(div().flex_1().child(format!("{} opened files were changed", changed.len())))
            .child(link("edits-upload-all".into(), "UPLOAD ALL", true).on_click(cx.listener(move |shell, _, _, cx| {
                for id in &ids {
                    shell.upload_edit(*id);
                }
                cx.notify();
            })))
            .child(link("edits-discard-all".into(), "DISCARD ALL", false).on_click(cx.listener(|shell, _, _, cx| {
                for id in shell.sftp.actions.edits.changed().iter().map(|f| f.id).collect::<Vec<_>>() {
                    shell.sftp.actions.edits.discard(id);
                }
                cx.notify();
            })))
            .child(
                div()
                    .id("edits-toggle")
                    .rounded_sm().icon_feedback()
                    .px_2()
                    .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT)).when(open, |s| {
                        s.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5)))
                    }))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        shell.sftp.actions.edits_open = !shell.sftp.actions.edits_open;
                        cx.notify();
                    })),
            );
        Some(bar.child(header).when(open, |d| {
            d.child(div().id("edits-list").max_h(px(240.)).overflow_y_scroll().flex().flex_col().gap(px(6.)).children(changed.into_iter().map(row)))
        }))
    }

    /// The bar under both panels: newest transfer and the way to see all.
    pub fn render_transfer_bar(&self, cx: &mut Context<Self>) -> Option<Div> {
        let newest = self.sftp.actions.transfers.last()?;
        let count = self.sftp.actions.transfers.len();
        let details = if count > 1 { format!("All transfers ({count})") } else { "Details".to_owned() };
        Some(
            div()
                .flex_none()
                .pr(px(10.))
                .flex()
                .items_center()
                .bg(rgb(theme::DIALOG_HEADER))
                .border_t_1()
                .border_color(rgb(theme::TITLE_BAR))
                .child(div().flex_1().min_w_0().child(self.render_transfer(newest, false, cx)))
                .child(
                    div()
                        .id("transfers-details")
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(theme::TEXT))
                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                        .child(details)
                        .on_click(cx.listener(|shell, _, window, cx| {
                            shell.open_dialog(Dialog::Transfers, window);
                            cx.notify();
                        })),
                ),
        )
    }
}

fn side_key(side: Side) -> &'static str {
    match side {
        Side::Left => "local",
        Side::Right => "remote",
    }
}
