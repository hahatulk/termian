//! Terminal tabs as legacy workspaces: one or more panes laid out by a
//! [`SplitTree`], the focused pane taking the keyboard, Broadcast input
//! sending what is typed in one pane to all of them, and «Split
//! horizontally» opening the same connection again beside a pane.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, px, rgb, AnyElement, Context, Entity, MouseButton, Window};
use termian_native::autocomplete::Candidates;
use termian_native::session::SshTarget;
use termian_native::vault::HostId;
use termian_native::split_tree::{Direction, Layout, Position, SplitTree};
use termian_native::tabs::{TabId, TabKind};
use termian_native::theme;
use termian_native::workspace_template::{self as templates, OpenTerminal, Target, ViewMode, WorkspaceTemplate};

use super::terminal_view::{TerminalEvent, TerminalView, TOP_INSET};
use crate::Shell;

/// What a pane connects to, so Duplicate and «Split horizontally» can
/// open it again; serial ports and logs cannot be reopened.
#[derive(Clone)]
pub enum Origin {
    Host(HostId),
    Ssh { label: String, os: Option<String>, target: Box<SshTarget> },
    Local,
    None,
}

/// The tab strip lies under the macOS title bar, which drags the window on
/// its own before the app sees the press: while the pointer is over a
/// terminal tab the window is not movable, so the tab itself can be dragged.
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)] // objc 0.2's msg_send! checks a cfg this crate does not declare
pub fn set_window_movable(window: &Window, movable: bool) {
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
    if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
        let view = appkit.ns_view.as_ptr() as *mut objc::runtime::Object;
        // SAFETY: the view belongs to this live window; both calls are plain AppKit messages.
        unsafe {
            let ns_window: *mut objc::runtime::Object = msg_send![view, window];
            if !ns_window.is_null() {
                let _: () = msg_send![ns_window, setMovable: movable as objc::runtime::BOOL];
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_window_movable(_: &Window, _: bool) {}

/// What dragging a terminal tab carries.
#[derive(Clone)]
pub struct TabDrag {
    pub tab: TabId,
    pub title: String,
}

impl gpui::Render for TabDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().px_3().py_1().rounded_md().bg(rgb(theme::ACCENT)).text_sm().text_color(rgb(0xffffff)).child(self.title.clone())
    }
}

pub struct Pane {
    pub title: String,
    pub view: Entity<TerminalView>,
    /// The Logs row of this pane's session.
    pub history: Option<u64>,
    pub origin: Origin,
    /// The template terminal this pane was opened from.
    pub template_terminal: Option<String>,
}

/// Width of the Focus View terminal list.
const FOCUS_LIST: f32 = 240.;

/// Height of a split pane's title row.
const PANE_HEADER: f32 = 24.;

pub struct Workspace {
    pub tree: SplitTree,
    pub panes: BTreeMap<u64, Pane>,
    pub focused: u64,
    /// «Broadcast input»: keys typed in a pane go to every pane of the tab.
    pub broadcast: bool,
    /// A divider being dragged.
    drag: Option<Drag>,
    /// A pane closed: the focused one takes the keyboard on the next frame.
    refocus: bool,
    /// The workspace template the tab was opened from or saved as.
    pub template: Option<u64>,
    /// Legacy «Focus View» (a list of the terminals and one of them) or «Split View».
    pub view_mode: ViewMode,
}

struct Drag {
    split: u64,
    index: usize,
    vertical: bool,
    start: f32,
    /// The split's length in pixels along the drag.
    length: f32,
    sizes: Vec<f32>,
}

/// Panes are never dragged smaller than this share of their split.
const MIN_SHARE: f32 = 0.1;

impl Workspace {
    pub fn focused_view(&self) -> Option<&Entity<TerminalView>> {
        self.panes.get(&self.focused).map(|p| &p.view)
    }
}

impl Shell {
    pub fn focused_view(&self, tab: TabId) -> Option<Entity<TerminalView>> {
        self.workspaces.get(&tab).and_then(|w| w.focused_view()).cloned()
    }

    pub fn all_views(&self) -> Vec<Entity<TerminalView>> {
        self.workspaces.values().flat_map(|w| w.panes.values().map(|p| p.view.clone())).collect()
    }

    fn new_pane_id(&mut self) -> u64 {
        self.next_pane += 1;
        self.next_pane
    }

    /// A new terminal tab with one pane, or a pane beside the one being split.
    pub(crate) fn add_terminal(&mut self, title: String, view: Entity<TerminalView>, history: Option<u64>, origin: Origin, cx: &mut Context<Self>) {
        if let Some((tab, target)) = self.split_into.take().filter(|(tab, _)| self.workspaces.contains_key(tab)) {
            self.tabs.activate(tab);
            self.add_pane(tab, target, title, view, history, origin, Layout::Horizontal, cx);
            return;
        }
        let tab = self.tabs.open(termian_native::tabs::TabKind::Session { title: title.clone() });
        let pane = self.new_pane_id();
        self.subscribe_pane(pane, &view, cx);
        let mut panes = BTreeMap::new();
        panes.insert(pane, Pane { title, view, history, origin, template_terminal: None });
        self.workspaces.insert(tab, Workspace { tree: SplitTree::single(pane), panes, focused: pane, broadcast: false, drag: None, refocus: false, template: None, view_mode: ViewMode::Split });
        cx.notify();
    }

    /// A pane beside `target` in `tab`, focused.
    #[allow(clippy::too_many_arguments)]
    fn add_pane(&mut self, tab: TabId, target: u64, title: String, view: Entity<TerminalView>, history: Option<u64>, origin: Origin, layout: Layout, cx: &mut Context<Self>) {
        let pane = self.new_pane_id();
        self.subscribe_pane(pane, &view, cx);
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            workspace.tree.split(target, pane, layout, Position::End);
            workspace.panes.insert(pane, Pane { title, view, history, origin, template_terminal: None });
            workspace.focused = pane;
        }
        cx.notify();
    }

    /// The tab a pane is in now (panes move when tabs are merged).
    fn tab_of(&self, pane: u64) -> Option<TabId> {
        self.workspaces.iter().find(|(_, w)| w.panes.contains_key(&pane)).map(|(tab, _)| *tab)
    }

    fn subscribe_pane(&mut self, pane: u64, view: &Entity<TerminalView>, cx: &mut Context<Self>) {
        cx.subscribe(view, move |shell, view, event: &TerminalEvent, cx| {
            let Some(tab) = shell.tab_of(pane) else { return };
            let history = shell.workspaces.get(&tab).and_then(|w| w.panes.get(&pane)).and_then(|p| p.history);
            match event {
                // A Serial tab gets its Logs row only once the port opens.
                TerminalEvent::SerialConnected(label) => {
                    let (entry, log) = shell.record_session(None, label, "serial".into());
                    if let Some(p) = shell.workspaces.get_mut(&tab).and_then(|w| w.panes.get_mut(&pane)) {
                        p.history = Some(entry);
                    }
                    view.update(cx, |view, _| view.set_log(log));
                }
                TerminalEvent::Os(os) => shell.host_os_detected(history, os),
                TerminalEvent::SaveUsername(username) => {
                    let host = shell.workspaces.get(&tab).and_then(|w| w.panes.get(&pane)).and_then(|p| match p.origin {
                        Origin::Host(host) => Some(host),
                        _ => None,
                    });
                    if let Some(host) = host.and_then(|id| shell.vault.hosts.iter_mut().find(|h| h.id == id)) {
                        host.username = Some(username.clone());
                        shell.save_vault();
                    }
                }
                TerminalEvent::EditHost => {
                    let host = shell.workspaces.get(&tab).and_then(|w| w.panes.get(&pane)).and_then(|p| match p.origin {
                        Origin::Host(host) => Some(host),
                        _ => None,
                    });
                    // As in legacy: back to Vaults with the host's editor open.
                    if let (Some(host), Some(main)) = (host, shell.main_window) {
                        cx.defer(move |cx| {
                            let _ = main.update(cx, |shell, window, cx| {
                                if let Some(vaults) = shell.tabs.all().iter().find(|t| t.kind == TabKind::Vaults).map(|t| t.id) {
                                    shell.tabs.activate(vaults);
                                }
                                shell.section = super::hosts::Section::Hosts;
                                shell.edit_host(host, window, cx);
                                cx.notify();
                            });
                        });
                    }
                }
                TerminalEvent::Input(bytes) => shell.broadcast_input(tab, pane, bytes, cx),
                TerminalEvent::Command(command) => {
                    let host = shell.workspaces.get(&tab).and_then(|w| w.panes.get(&pane)).and_then(|p| match p.origin {
                        Origin::Host(host) => Some(host),
                        _ => None,
                    });
                    shell.vault.record_command(command, host, crate::now());
                    shell.save_vault();
                }
                TerminalEvent::History(commands) => {
                    let host = shell.workspaces.get(&tab).and_then(|w| w.panes.get(&pane)).and_then(|p| match p.origin {
                        Origin::Host(host) => Some(host),
                        _ => None,
                    });
                    if shell.vault.merge_shell_history(commands, host) {
                        shell.save_vault();
                        cx.notify();
                    }
                }
                TerminalEvent::Ended | TerminalEvent::Close => {
                    if shell.session_ended(history) {
                        shell.backup_logs_soon(cx);
                    }
                    if let TerminalEvent::Close = event {
                        shell.close_pane(tab, pane, cx);
                    }
                    cx.notify();
                }
            }
        })
        .detach();
    }

    /// Typed in a broadcasting tab: the other panes get the same bytes.
    fn broadcast_input(&mut self, tab: TabId, from: u64, bytes: &[u8], cx: &mut Context<Self>) {
        let Some(workspace) = self.workspaces.get(&tab).filter(|w| w.broadcast) else { return };
        let others: Vec<_> = workspace.panes.iter().filter(|(id, _)| **id != from).map(|(_, p)| p.view.clone()).collect();
        for view in others {
            view.update(cx, |view, _| view.write_input(bytes));
        }
    }

    /// Closes one pane; the last one closes the tab.
    pub(crate) fn close_pane(&mut self, tab: TabId, pane: u64, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspaces.get_mut(&tab) else { return };
        let Some(closed) = workspace.panes.remove(&pane) else { return };
        if workspace.panes.is_empty() {
            self.close_terminal_tab(tab);
            return;
        }
        workspace.tree.remove(pane);
        if workspace.focused == pane {
            workspace.focused = workspace.tree.leaves()[0];
        }
        workspace.refocus = true;
        self.session_ended(closed.history);
        cx.notify();
    }

    /// Closes a tab; dropping its terminals ends the sessions and finishes their logs.
    pub(crate) fn close_terminal_tab(&mut self, tab: TabId) {
        if let Some(workspace) = self.workspaces.remove(&tab) {
            for pane in workspace.panes.into_values() {
                self.session_ended(pane.history);
            }
        }
        let was_active = self.tabs.active().id == tab;
        self.tabs.close(tab);
        // Back on Vaults after a disconnect, the Hosts list is shown.
        if was_active && self.tabs.active().kind == TabKind::Vaults {
            self.section = super::hosts::Section::Hosts;
        }
    }

    /// Opens a pane's connection again: in a new tab (Duplicate) or beside
    /// it (`split`).
    pub fn reopen(&mut self, tab: TabId, split: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspaces.get(&tab) else { return };
        let focused = workspace.focused;
        let Some(origin) = workspace.panes.get(&focused).map(|p| p.origin.clone()) else { return };
        self.split_into = split.then_some((tab, focused));
        match origin {
            Origin::Host(host) => self.connect_host(host, window, cx),
            Origin::Ssh { label, os, target } => self.open_ssh(label, os, *target, window, cx),
            Origin::Local => self.open_local_terminal(window, cx),
            Origin::None => {}
        }
        self.split_into = None;
        if let Some(view) = self.focused_view(self.tabs.active().id) {
            view.read(cx).focus(window);
        }
        cx.notify();
    }

    /// Legacy «Run» on several targets: one workspace named after the
    /// snippet, its panes laid out as a balanced tree.
    pub fn open_workspace_for(&mut self, title: &str, hosts: &[HostId], script: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some((first, rest)) = hosts.split_first() else { return };
        self.connect_host_with(*first, Some(script.to_owned()), window, cx);
        let tab = self.tabs.active().id;
        for host in rest {
            let target = self.workspaces.get(&tab).map(|w| w.focused);
            self.split_into = target.map(|pane| (tab, pane));
            self.connect_host_with(*host, Some(script.to_owned()), window, cx);
        }
        self.split_into = None;
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            let mut panes: Vec<u64> = workspace.panes.keys().copied().collect();
            panes.sort();
            workspace.tree = SplitTree::balanced(&panes);
            workspace.focused = panes[0];
        }
        self.tabs.set_title(tab, title);
        cx.notify();
    }

    /// A terminal tab dropped on a pane joins it, on the side nearest to
    /// where it was dropped (legacy: drag a tab into a workspace view).
    #[allow(clippy::too_many_arguments)]
    fn merge_tab(&mut self, source: TabId, tab: TabId, target: u64, layout: Layout, position: Position, window: &mut Window, cx: &mut Context<Self>) {
        if source == tab || !self.workspaces.contains_key(&tab) {
            return;
        }
        let Some(moved) = self.workspaces.remove(&source) else { return };
        self.tabs.close(source);
        let leaves = moved.tree.leaves();
        let mut panes = moved.panes;
        let workspace = self.workspaces.get_mut(&tab).expect("checked above");
        let mut anchor = target;
        for (i, leaf) in leaves.iter().enumerate() {
            let Some(pane) = panes.remove(leaf) else { continue };
            // The first pane goes where it was dropped; the rest follow it.
            let (layout, position) = if i == 0 { (layout, position) } else { (layout, Position::End) };
            workspace.tree.split(anchor, *leaf, layout, position);
            workspace.panes.insert(*leaf, pane);
            anchor = *leaf;
        }
        workspace.broadcast &= moved.broadcast;
        workspace.focused = leaves[0];
        workspace.refocus = true;
        self.tabs.activate(tab);
        let _ = window;
        cx.notify();
    }

    pub fn can_reopen(&self, tab: TabId) -> bool {
        self.workspaces.get(&tab).and_then(|w| w.panes.get(&w.focused)).is_some_and(|p| !matches!(p.origin, Origin::None))
    }

    pub fn toggle_broadcast(&mut self, cx: &mut Context<Self>) {
        let tab = self.tabs.active().id;
        if let Some(workspace) = self.workspaces.get_mut(&tab).filter(|w| w.panes.len() > 1) {
            workspace.broadcast = !workspace.broadcast;
        }
        cx.notify();
    }

    /// The modifier + arrow keys of «Move focus between Split View panes».
    pub fn focus_pane(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tabs.active().id;
        let Some(workspace) = self.workspaces.get_mut(&tab) else { return };
        if let Some(next) = workspace.tree.adjacent(workspace.focused, direction) {
            workspace.focused = next;
            if let Some(view) = workspace.focused_view() {
                view.read(cx).focus(window);
            }
        }
        cx.notify();
    }

    /// The panes of the active tab, each in its share of the area.
    /// Legacy «open workspace template»: its terminals in one tab, with its
    /// layout, titles and view mode. Hosts that are gone are skipped.
    pub fn open_template(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(template) = templates::templates(&self.vault).into_iter().find(|t| t.id == id) else { return };
        let mut tab = None;
        let mut opened: Vec<(String, u64)> = Vec::new();
        for terminal in &template.workspace_terminals {
            let Some(target) = terminal.target(&self.vault) else { continue };
            self.split_into = tab.and_then(|tab| Some((tab, self.workspaces.get(&tab)?.focused)));
            let before = self.next_pane;
            // Legacy runs the template's startup snippet (directory, then script).
            let startup = terminal.startup();
            match target {
                Target::Host(host) => self.connect_host_with(host, startup.clone(), window, cx),
                Target::Local => self.open_local_terminal(window, cx),
            }
            self.split_into = None;
            if self.next_pane == before {
                continue;
            }
            let pane = self.next_pane;
            let Some(at) = self.tab_of(pane) else { continue };
            tab = Some(at);
            if let Some(p) = self.workspaces.get_mut(&at).and_then(|w| w.panes.get_mut(&pane)) {
                if let (Target::Local, Some(script)) = (target, startup) {
                    p.view.update(cx, |view, _| view.set_local_startup(script));
                }
                p.template_terminal = Some(terminal.id.clone());
                if let Some(label) = terminal.label.as_ref().filter(|l| !l.is_empty()) {
                    p.title = label.clone();
                }
            }
            opened.push((terminal.id.clone(), pane));
        }
        let Some(tab) = tab else { return };
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            if let Some(tree) = template.tree(&opened) {
                workspace.tree = tree;
            }
            workspace.focused = workspace.tree.leaves()[0];
            workspace.refocus = true;
            workspace.template = Some(template.id);
            workspace.view_mode = template.view_mode;
        }
        self.tabs.set_title(tab, &template.label);
        self.tabs.activate(tab);
        cx.notify();
    }

    /// Open workspaces: tabs with several panes or opened from a template.
    pub(crate) fn workspace_tabs(&self) -> Vec<(TabId, String)> {
        self.tabs
            .all()
            .iter()
            .filter(|t| self.workspaces.get(&t.id).is_some_and(|w| w.panes.len() > 1 || w.template.is_some()))
            .map(|t| (t.id, t.title().to_owned()))
            .collect()
    }

    /// Legacy «Add to Workspace»: the host opens beside the focused pane.
    pub(crate) fn add_to_workspace(&mut self, tab: TabId, host: HostId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(focused) = self.workspaces.get(&tab).map(|w| w.focused) else { return };
        self.split_into = Some((tab, focused));
        self.connect_host(host, window, cx);
        self.split_into = None;
        cx.notify();
    }

    /// The tab as a template: its saved hosts and local terminals, in order.
    fn tab_as_template(&self, tab: TabId, id: u64, previous: Option<&WorkspaceTemplate>) -> Option<WorkspaceTemplate> {
        let workspace = self.workspaces.get(&tab)?;
        let panes: Vec<OpenTerminal> = workspace
            .tree
            .leaves()
            .into_iter()
            .filter_map(|leaf| {
                let pane = workspace.panes.get(&leaf)?;
                let target = match pane.origin {
                    Origin::Host(host) => Target::Host(host),
                    Origin::Local => Target::Local,
                    _ => return None,
                };
                Some(OpenTerminal { pane: leaf, target, from: pane.template_terminal.clone() })
            })
            .collect();
        // A tab saved for the first time gets the legacy default name.
        let label = match previous {
            Some(_) => self.tabs.all().iter().find(|t| t.id == tab).map(|t| t.title().to_owned()).unwrap_or_default(),
            None => templates::DEFAULT_NAME.to_owned(),
        };
        WorkspaceTemplate::from_tab(id, &label, workspace.view_mode, &panes, &workspace.tree, previous)
    }

    /// Legacy workspace changes: `Some(true)` for a workspace not saved yet
    /// («Save template»), `Some(false)` when it differs from its template
    /// («Update template»), `None` with nothing to save.
    pub(crate) fn template_changes(&self, tab: TabId) -> Option<bool> {
        let workspace = self.workspaces.get(&tab)?;
        let saved = workspace.template.and_then(|id| templates::templates(&self.vault).into_iter().find(|t| t.id == id));
        match saved {
            Some(saved) => self.tab_as_template(tab, saved.id, Some(&saved)).filter(|now| now.differs(&saved)).map(|_| false),
            None => (workspace.panes.len() > 1 && self.tab_as_template(tab, 0, None).is_some()).then_some(true),
        }
    }

    /// «Save template» / «Update template».
    pub(crate) fn save_tab_template(&mut self, tab: TabId, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspaces.get(&tab) else { return };
        let saved = workspace.template.and_then(|id| templates::templates(&self.vault).into_iter().find(|t| t.id == id));
        let id = saved.as_ref().map_or_else(|| templates::next_id(&self.vault), |t| t.id);
        let Some(template) = self.tab_as_template(tab, id, saved.as_ref()) else { return };
        templates::save(&mut self.vault, &template);
        // Panes keep the ids they were saved with, so a later update keeps titles and snippets.
        let terminal_ids: Vec<String> = template.workspace_terminals.iter().map(|t| t.id.clone()).collect();
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            let supported: Vec<u64> = workspace.tree.leaves().into_iter().filter(|leaf| workspace.panes.get(leaf).is_some_and(|p| matches!(p.origin, Origin::Host(_) | Origin::Local))).collect();
            for (leaf, terminal) in supported.into_iter().zip(terminal_ids) {
                if let Some(pane) = workspace.panes.get_mut(&leaf) {
                    pane.template_terminal = Some(terminal);
                }
            }
            workspace.template = Some(id);
        }
        self.tabs.set_title(tab, &template.label);
        self.save_vault();
        cx.notify();
    }

    pub(crate) fn set_view_mode(&mut self, tab: TabId, mode: ViewMode, focus: Option<u64>, cx: &mut Context<Self>) {
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            workspace.view_mode = mode;
            if let Some(pane) = focus.filter(|p| workspace.panes.contains_key(p)) {
                workspace.focused = pane;
            }
            workspace.refocus = true;
        }
        cx.notify();
    }

    pub(crate) fn render_workspace(&mut self, tab: TabId, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            if std::mem::take(&mut workspace.refocus) {
                if let Some(view) = workspace.focused_view() {
                    view.read(cx).focus(window);
                }
            }
        }
        // Autocomplete candidates are rebuilt when history or snippets change.
        let key = (self.vault.shell_history.len(), self.vault.shell_history.first().map(|c| (c.script.clone(), c.completed_at)), self.vault.snippets.len(), self.vault.snippets.iter().map(|s| s.script.len()).sum::<usize>());
        if self.candidates_key != key {
            self.candidates_key = key;
            self.candidates.clear();
        }
        let Some(workspace) = self.workspaces.get(&tab) else { return div().into_any_element() };
        let viewport = window.viewport_size();
        let panel = if self.side_panel.is_some() { super::side_panel::WIDTH } else { 0. };
        let split = workspace.panes.len() > 1;
        // Focus View: the terminal list, then the focused terminal alone.
        let focus = split && workspace.view_mode == ViewMode::Focus;
        let list = if focus { FOCUS_LIST } else { 0. };
        let (width, height) = (f32::from(viewport.width) - panel - list, f32::from(viewport.height) - TOP_INSET);
        let rects = if focus {
            vec![(workspace.focused, true, termian_native::split_tree::Rect { left: 0., top: 0., right: 1., bottom: 1. })]
        } else {
            workspace.tree.rects()
        };
        let mut area = div().size_full().relative();
        for (id, leaf, rect) in rects {
            if !leaf {
                continue;
            }
            let Some(pane) = workspace.panes.get(&id) else { continue };
            let (left, top) = (rect.left * width, rect.top * height);
            let (w, h) = ((rect.right - rect.left) * width, (rect.bottom - rect.top) * height);
            // A split pane has a header with its title and a close cross.
            let framed = split && !focus;
            let header = if framed { PANE_HEADER } else { 0. };
            let host = match pane.origin {
                Origin::Host(host) => Some(host),
                _ => None,
            };
            let candidates = self.candidates.entry(host).or_insert_with(|| std::sync::Arc::new(Candidates::for_host(&self.vault, host))).clone();
            let enabled = self.vault.terminal.autocomplete && !split;
            pane.view.update(cx, |view, _| {
                view.set_area(list + left, TOP_INSET + top + header, w, h - header);
                view.set_autocomplete(enabled, candidates);
            });
            let focused = framed && id == workspace.focused;
            let border = if workspace.broadcast { theme::ERROR } else { theme::ACCENT };
            area = area.child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(w))
                    .h(px(h))
                    .when(framed, |d| d.border_1().border_color(rgb(if focused || workspace.broadcast { border } else { theme::TITLE_BAR })))
                    .flex()
                    .flex_col()
                    .drag_over::<TabDrag>(|style, _, _, _| style.border_2().border_color(rgb(theme::ACCENT)))
                    .on_drop(cx.listener(move |shell, drag: &TabDrag, window, cx| {
                        // The nearest edge of the pane decides the side.
                        let at = shell.pointer;
                        let x = ((f32::from(at.x) - left) / w.max(1.)).clamp(0., 1.);
                        let y = ((f32::from(at.y) - TOP_INSET - top) / h.max(1.)).clamp(0., 1.);
                        let (layout, position) = [
                            (x, Layout::Horizontal, Position::Start),
                            (1. - x, Layout::Horizontal, Position::End),
                            (y, Layout::Vertical, Position::Start),
                            (1. - y, Layout::Vertical, Position::End),
                        ]
                        .into_iter()
                        .min_by(|a, b| a.0.total_cmp(&b.0))
                        .map(|(_, l, p)| (l, p))
                        .unwrap_or((Layout::Horizontal, Position::End));
                        shell.merge_tab(drag.tab, tab, id, layout, position, window, cx);
                    }))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |shell, _, _, cx| {
                        if let Some(workspace) = shell.workspaces.get_mut(&tab) {
                            workspace.focused = id;
                        }
                        cx.notify();
                    }))
                    .when(framed, |d| {
                        d.child(
                            div()
                                .h(px(PANE_HEADER))
                                .flex_none()
                                .px_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .bg(rgb(theme::TITLE_BAR))
                                .text_xs()
                                .text_color(rgb(if focused { theme::TEXT } else { theme::TEXT_MUTED }))
                                .child(div().flex_1().truncate().child(pane.title.clone()))
                                .child(
                                    div()
                                        .id(gpui::SharedString::from(format!("pane-focus-{id}")))
                                        .size(px(18.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                                        .child(gpui::svg().path(super::assets::icon("side_panel")).size(px(11.)).text_color(rgb(theme::TEXT)))
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            cx.stop_propagation();
                                            shell.set_view_mode(tab, ViewMode::Focus, Some(id), cx);
                                        })),
                                )
                                .child(
                                    div()
                                        .id(gpui::SharedString::from(format!("pane-close-{id}")))
                                        .size(px(18.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_sm()
                                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                                        .child(gpui::svg().path(super::assets::icon("close")).size(px(9.)).text_color(rgb(theme::TEXT)))
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            cx.stop_propagation();
                                            shell.close_pane(tab, id, cx);
                                        })),
                                ),
                        )
                    })
                    .child(div().flex_1().min_h_0().child(pane.view.clone())),
            );
        }
        // Dividers between the children of every split, draggable.
        for (split_id, layout, rect, sizes) in if focus { Vec::new() } else { workspace.tree.splits() } {
            let vertical = layout == Layout::Vertical;
            let (x, y) = (rect.left * width, rect.top * height);
            let (w, h) = ((rect.right - rect.left) * width, (rect.bottom - rect.top) * height);
            let length = if vertical { h } else { w };
            let mut offset = 0.;
            for (index, size) in sizes.iter().enumerate().take(sizes.len().saturating_sub(1)) {
                offset += size * length;
                let sizes = sizes.clone();
                let handle = div()
                    .id(gpui::SharedString::from(format!("divider-{split_id}-{index}")))
                    .absolute()
                    .map(|d| if vertical { d.left(px(x)).top(px(y + offset - 3.)).w(px(w)).h(px(6.)).cursor_row_resize() } else { d.left(px(x + offset - 3.)).top(px(y)).w(px(6.)).h(px(h)).cursor_col_resize() })
                    .on_mouse_down(MouseButton::Left, cx.listener(move |shell, event: &gpui::MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        let start = if vertical { f32::from(event.position.y) } else { f32::from(event.position.x) };
                        if let Some(workspace) = shell.workspaces.get_mut(&tab) {
                            workspace.drag = Some(Drag { split: split_id, index, vertical, start, length, sizes: sizes.clone() });
                        }
                    }));
                area = area.child(handle);
            }
        }
        let area = area.on_mouse_move(cx.listener(move |shell, event: &gpui::MouseMoveEvent, _, cx| {
            shell.pointer = event.position;
            let Some(workspace) = shell.workspaces.get_mut(&tab) else { return };
            let Some(drag) = &workspace.drag else { return };
            if !event.dragging() {
                workspace.drag = None;
                return;
            }
            let at = if drag.vertical { f32::from(event.position.y) } else { f32::from(event.position.x) };
            let sizes = termian_native::split_tree::dragged(&drag.sizes, drag.index, (at - drag.start) / drag.length.max(1.), MIN_SHARE);
            let split = drag.split;
            workspace.tree.resize(split, &sizes);
            cx.notify();
        }))
        .on_mouse_up(MouseButton::Left, cx.listener(move |shell, _, _, _| {
            if let Some(workspace) = shell.workspaces.get_mut(&tab) {
                workspace.drag = None;
            }
        }));
        if !focus {
            return area.into_any_element();
        }
        div().size_full().flex().child(self.render_focus_list(tab, cx)).child(div().flex_1().min_w_0().h_full().child(area)).into_any_element()
    }

    /// Legacy Focus View list: «Terminals • N», the Split view and Broadcast
    /// input buttons, a row per terminal (click shows it, × closes it).
    fn render_focus_list(&self, tab: TabId, cx: &mut Context<Self>) -> AnyElement {
        let Some(workspace) = self.workspaces.get(&tab) else { return div().into_any_element() };
        let control = |id: &'static str, icon: &'static str, active: bool| {
            div()
                .id(id)
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(10.))
                .text_color(rgb(if active { theme::TEXT } else { theme::TEXT_MUTED }))
                .when(active, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)).text_color(rgb(theme::TEXT)))
                .child(gpui::svg().path(super::assets::icon(icon)).size(px(14.)).text_color(rgb(if active { theme::TEXT } else { theme::TEXT_MUTED })))
        };
        let header = div()
            .pt(px(5.))
            .pl(px(5.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_sm()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child("Terminals")
                    .child(div().text_xs().opacity(0.5).child("•"))
                    .child(div().opacity(0.5).child(workspace.panes.len().to_string())),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .child(control("focus-split-view", "workspace", false).on_click(cx.listener(move |shell, _, _, cx| shell.set_view_mode(tab, ViewMode::Split, None, cx))))
                    .child(control("focus-broadcast", "broadcast", workspace.broadcast).on_click(cx.listener(move |shell, _, _, cx| {
                        shell.tabs.activate(tab);
                        shell.toggle_broadcast(cx);
                    }))),
            );
        let rows = workspace.tree.leaves().into_iter().filter_map(|id| {
            let pane = workspace.panes.get(&id)?;
            let selected = id == workspace.focused;
            Some(
                div()
                    .id(gpui::SharedString::from(format!("focus-row-{id}")))
                    .group("focus-row")
                    .w_full()
                    .p(px(5.))
                    .pl(px(10.))
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded(px(8.))
                    .bg(rgb(if selected { theme::SURFACE_ACTIVE } else { theme::SURFACE }))
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .text_sm()
                    .text_color(rgb(if selected { theme::TEXT } else { theme::TEXT_MUTED }))
                    .child(div().flex_1().min_w_0().truncate().child(pane.title.clone()))
                    .child(
                        div()
                            .id(gpui::SharedString::from(format!("focus-close-{id}")))
                            .invisible()
                            .group_hover("focus-row", |s| s.visible())
                            .size(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .hover(|s| s.bg(rgb(theme::TITLE_BAR)))
                            .child(gpui::svg().path(super::assets::icon("close")).size(px(9.)).text_color(rgb(theme::TEXT)))
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                cx.stop_propagation();
                                shell.close_pane(tab, id, cx);
                            })),
                    )
                    .on_click(cx.listener(move |shell, _, _, cx| shell.set_view_mode(tab, ViewMode::Focus, Some(id), cx))),
            )
        });
        div()
            .w(px(FOCUS_LIST))
            .h_full()
            .flex_none()
            .p(px(5.))
            .child(
                div()
                    .size_full()
                    .p(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(15.))
                    .rounded(px(10.))
                    .bg(rgb(theme::TITLE_BAR))
                    .child(header)
                    .child(div().id("focus-rows").flex_1().overflow_y_scroll().flex().flex_col().gap(px(5.)).children(rows)),
            )
            .into_any_element()
    }

    /// Legacy terminal tab menu: Duplicate, «Split horizontally», Close.
    pub(crate) fn render_tab_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (tab, at) = self.tab_menu?;
        let reopen = self.can_reopen(tab);
        let items = super::host_editor::menu_panel("tab-menu")
            .child(super::host_editor::menu_row("tab-menu-duplicate".into(), "Duplicate", false, !reopen).when(reopen, |d| {
                d.on_click(cx.listener(move |shell, _, window, cx| {
                    shell.tab_menu = None;
                    shell.reopen(tab, false, window, cx);
                }))
            }))
            .child(super::host_editor::menu_row("tab-menu-split".into(), "Split horizontally", false, !reopen).when(reopen, |d| {
                d.on_click(cx.listener(move |shell, _, window, cx| {
                    shell.tab_menu = None;
                    shell.reopen(tab, true, window, cx);
                }))
            }))
            .child(super::host_editor::menu_row("tab-menu-close".into(), "Close", false, false).on_click(cx.listener(move |shell, _, _, cx| {
                shell.tab_menu = None;
                shell.close_terminal_tab(tab);
                cx.notify();
            })));
        let viewport = window.viewport_size();
        let at = gpui::Point { x: at.x.min(viewport.width - px(230.)), y: at.y };
        Some(
            gpui::deferred(
                div()
                    .id("tab-menu-dismiss")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                        shell.tab_menu = None;
                        cx.notify();
                    }))
                    .child(gpui::anchored().position(at).child(items)),
            )
            .into_any_element(),
        )
    }

    pub(crate) fn is_broadcasting(&self, tab: TabId) -> bool {
        self.workspaces.get(&tab).is_some_and(|w| w.broadcast)
    }

    /// «Broadcast input» in the title bar while the active tab is split.
    pub(crate) fn render_broadcast_button(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.tabs.active().id;
        let workspace = self.workspaces.get(&tab).filter(|w| w.panes.len() > 1)?;
        let on = workspace.broadcast;
        Some(
            div()
                .id("broadcast-toggle")
                .h(px(30.))
                .px_2()
                .flex()
                .items_center()
                .rounded_md()
                .text_xs()
                .font_weight(gpui::FontWeight::BOLD)
                .bg(rgb(if on { theme::ERROR } else { theme::SURFACE }))
                .text_color(rgb(if on { 0xffffff } else { theme::TEXT }))
                .hover(|s| s.opacity(0.85))
                .child("Broadcast")
                .on_click(cx.listener(|shell, _, _, cx| shell.toggle_broadcast(cx)))
                .into_any_element(),
        )
    }
}