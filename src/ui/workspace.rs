//! Terminal tabs as legacy workspaces: one or more panes laid out by a
//! [`SplitTree`], the focused pane taking the keyboard, Broadcast input
//! sending what is typed in one pane to all of them, and «Split
//! horizontally» opening the same connection again beside a pane.

use std::collections::BTreeMap;

use gpui::{div, prelude::*, px, rgb, AnyElement, Context, Entity, MouseButton, Window};
use termian_native::session::SshTarget;
use termian_native::vault::HostId;
use termian_native::split_tree::{Direction, Layout, Position, SplitTree};
use termian_native::tabs::TabId;
use termian_native::theme;

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

pub struct Pane {
    pub title: String,
    pub view: Entity<TerminalView>,
    /// The Logs row of this pane's session.
    pub history: Option<u64>,
    pub origin: Origin,
}

/// Height of a split pane's title row.
const PANE_HEADER: f32 = 24.;

pub struct Workspace {
    pub tree: SplitTree,
    pub panes: BTreeMap<u64, Pane>,
    pub focused: u64,
    /// «Broadcast input»: keys typed in a pane go to every pane of the tab.
    pub broadcast: bool,
}

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
        self.subscribe_pane(tab, pane, &view, cx);
        let mut panes = BTreeMap::new();
        panes.insert(pane, Pane { title, view, history, origin });
        self.workspaces.insert(tab, Workspace { tree: SplitTree::single(pane), panes, focused: pane, broadcast: false });
        cx.notify();
    }

    /// A pane beside `target` in `tab`, focused.
    #[allow(clippy::too_many_arguments)]
    fn add_pane(&mut self, tab: TabId, target: u64, title: String, view: Entity<TerminalView>, history: Option<u64>, origin: Origin, layout: Layout, cx: &mut Context<Self>) {
        let pane = self.new_pane_id();
        self.subscribe_pane(tab, pane, &view, cx);
        if let Some(workspace) = self.workspaces.get_mut(&tab) {
            workspace.tree.split(target, pane, layout, Position::End);
            workspace.panes.insert(pane, Pane { title, view, history, origin });
            workspace.focused = pane;
        }
        cx.notify();
    }

    fn subscribe_pane(&mut self, tab: TabId, pane: u64, view: &Entity<TerminalView>, cx: &mut Context<Self>) {
        cx.subscribe(view, move |shell, view, event: &TerminalEvent, cx| {
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
                TerminalEvent::Input(bytes) => shell.broadcast_input(tab, pane, bytes, cx),
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
        self.tabs.close(tab);
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
    pub(crate) fn render_workspace(&self, tab: TabId, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(workspace) = self.workspaces.get(&tab) else { return div().into_any_element() };
        let viewport = window.viewport_size();
        let panel = if self.side_panel.is_some() { super::side_panel::WIDTH } else { 0. };
        let (width, height) = (f32::from(viewport.width) - panel, f32::from(viewport.height) - TOP_INSET);
        let split = workspace.panes.len() > 1;
        let mut area = div().size_full().relative();
        for (id, leaf, rect) in workspace.tree.rects() {
            if !leaf {
                continue;
            }
            let Some(pane) = workspace.panes.get(&id) else { continue };
            let (left, top) = (rect.left * width, rect.top * height);
            let (w, h) = ((rect.right - rect.left) * width, (rect.bottom - rect.top) * height);
            // A split pane has a header with its title and a close cross.
            let header = if split { PANE_HEADER } else { 0. };
            pane.view.update(cx, |view, _| view.set_area(left, TOP_INSET + top + header, w, h - header));
            let focused = split && id == workspace.focused;
            let border = if workspace.broadcast { theme::ERROR } else { theme::ACCENT };
            area = area.child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(w))
                    .h(px(h))
                    .when(split, |d| d.border_1().border_color(rgb(if focused || workspace.broadcast { border } else { theme::TITLE_BAR })))
                    .flex()
                    .flex_col()
                    .on_mouse_down(MouseButton::Left, cx.listener(move |shell, _, _, cx| {
                        if let Some(workspace) = shell.workspaces.get_mut(&tab) {
                            workspace.focused = id;
                        }
                        cx.notify();
                    }))
                    .when(split, |d| {
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
        area.into_any_element()
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