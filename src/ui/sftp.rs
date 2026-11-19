//! SFTP tab: two file panels side by side. The left one is the local disk;
//! the right one shows a remote host once connected.

use futures::StreamExt;
use gpui::{
    div, prelude::*, px, rgb, svg, ClickEvent, Context, Div, ExternalPaths, KeyDownEvent, MouseButton, MouseDownEvent, SharedString,
    Stateful, Window,
};
use termian_native::files::{crumbs, join, Entry, Panel, SortKey};
use termian_native::login::{Answer, HostKeyDecision, LoginEvent, LoginState};
use termian_native::remote_fs::{RemoteEvent, RemoteFs};
use termian_native::session::SshTarget;
use termian_native::theme;
use termian_native::vault::{GroupId, HostId};

use super::assets::icon;
use super::hosts::{host_tile, os_tile};
use super::login_view::{self, LoginActions, LoginTarget};
use super::sftp_actions::{Actions, DraggedRows, MenuKind};
use super::text_input::{self, Mode};
use super::pointer::PointerFeedback;
use crate::Shell;

/// Folder blue from the legacy file icons.
pub const FOLDER: u32 = 0x5ab4f0;
const COLUMNS: [(SortKey, &str, f32); 4] =
    [(SortKey::Name, "Name", 0.), (SortKey::Modified, "Date Modified", 175.), (SortKey::Size, "Size", 115.), (SortKey::Kind, "Kind", 115.)];

/// Which panel: each one shows the local disk or a host.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    fn key(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }

    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Where a host panel is in its lifecycle.
pub enum RemoteState {
    Idle,
    /// Choosing a saved host, or typing `user@host` to connect directly.
    Picking { query: String, group: Option<GroupId> },
    /// Logging in: the connection screen.
    Connecting(LoginState),
    Connected { panel: Panel, error: Option<String> },
    Failed(String),
}

pub struct Remote {
    pub label: String,
    os: Option<String>,
    target: SshTarget,
    pub connection: RemoteFs,
}

/// One panel: the local disk (the left one by default) or a host.
pub struct PanelSide {
    /// Shows a host (or is choosing / connecting to one) rather than the disk.
    pub host: bool,
    pub local: Panel,
    pub local_error: Option<String>,
    pub remote: Option<Remote>,
    pub state: RemoteState,
    /// Bumped on every connect; events from older connections are ignored.
    generation: u64,
}

impl PanelSide {
    fn new(host: bool) -> Self {
        let home = dirs::home_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
        let mut side = Self { host, local: Panel::new(&home), local_error: None, remote: None, state: RemoteState::Idle, generation: 0 };
        side.reload_local();
        side
    }

    fn reload_local(&mut self) {
        match termian_native::files::list_local(&self.local.path) {
            Ok(entries) => {
                self.local.set_entries(entries);
                self.local_error = None;
            }
            Err(error) => {
                self.local.set_entries(Vec::new());
                self.local_error = Some(error);
            }
        }
    }
}

pub struct SftpState {
    sides: [PanelSide; 2],
    pub filtering: Option<Side>,
    pub actions: Actions,
}

impl SftpState {
    /// The disk on the left, a host on the right, as legacy opens the tab.
    pub fn new() -> Self {
        Self { sides: [PanelSide::new(false), PanelSide::new(true)], filtering: None, actions: Actions::default() }
    }

    pub fn side(&self, side: Side) -> &PanelSide {
        &self.sides[side.index()]
    }

    pub fn side_mut(&mut self, side: Side) -> &mut PanelSide {
        &mut self.sides[side.index()]
    }

    /// The host connection of a side, when it shows a host.
    pub fn remote(&self, side: Side) -> Option<&Remote> {
        self.side(side).remote.as_ref().filter(|_| self.side(side).host)
    }

    pub fn remote_mut(&mut self, side: Side) -> Option<&mut Remote> {
        let s = self.side_mut(side);
        if s.host {
            s.remote.as_mut()
        } else {
            None
        }
    }

    /// The side lists files: the disk, or a connected host.
    pub fn is_ready(&self, side: Side) -> bool {
        self.panel(side).is_some()
    }

    pub fn is_host(&self, side: Side) -> bool {
        self.side(side).host
    }

    pub fn panel(&self, side: Side) -> Option<&Panel> {
        let s = self.side(side);
        if !s.host {
            return Some(&s.local);
        }
        match &s.state {
            RemoteState::Connected { panel, .. } => Some(panel),
            _ => None,
        }
    }

    pub fn panel_mut(&mut self, side: Side) -> Option<&mut Panel> {
        let s = self.side_mut(side);
        if !s.host {
            return Some(&mut s.local);
        }
        match &mut s.state {
            RemoteState::Connected { panel, .. } => Some(panel),
            _ => None,
        }
    }

    /// Opens a folder (or `..`) in a panel.
    fn open(&mut self, side: Side, name: &str) {
        let Some(panel) = self.panel_mut(side) else { return };
        if name == ".." {
            panel.up();
        } else if panel.entry(name).is_some_and(|e| e.is_dir) {
            let path = join(&panel.path, name);
            panel.navigate(path);
        } else {
            return;
        }
        self.reload(side);
    }

    pub fn reload(&mut self, side: Side) {
        let s = self.side_mut(side);
        if !s.host {
            s.reload_local();
        } else if let (Some(remote), RemoteState::Connected { panel, .. }) = (&s.remote, &s.state) {
            remote.connection.list(&panel.path);
        }
    }

    /// Transfers and pending copies that use a side's connection die with it.
    fn forget_remote_work(&mut self, side: Side) {
        self.actions.forget_side(side);
    }

    fn handle(&mut self, side: Side, event: RemoteEvent) {
        let s = self.side_mut(side);
        match (event, &mut s.state) {
            (RemoteEvent::Login(event), RemoteState::Connecting(login)) => login.apply(event),
            (RemoteEvent::Closed(reason), RemoteState::Connecting(login)) => {
                // Keep the screen (and the target) so the user can read why and retry.
                login.fail(reason.unwrap_or_else(|| "The connection was closed.".into()));
            }
            (RemoteEvent::Connected { home }, _) => {
                s.state = RemoteState::Connected { panel: Panel::new(&home), error: None };
                self.reload(side);
            }
            (RemoteEvent::Listed { path, result }, RemoteState::Connected { panel, error }) if path == panel.path => match result {
                Ok(entries) => {
                    panel.set_entries(entries);
                    *error = None;
                }
                Err(message) => {
                    panel.set_entries(Vec::new());
                    *error = Some(message);
                }
            },
            (RemoteEvent::Transfer { id, event }, _) => self.on_transfer(side, id, event),
            (RemoteEvent::Changed(result), _) => self.on_remote_changed(side, result),
            (RemoteEvent::Closed(reason), _) => {
                self.forget_remote_work(side);
                let s = self.side_mut(side);
                s.remote = None;
                s.state = match reason {
                    Some(reason) => RemoteState::Failed(reason),
                    None => RemoteState::Idle,
                };
            }
            _ => {}
        }
    }
}

fn icon_button(id: String, name: &str, enabled: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .size(px(24.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .when(enabled, |d| d.icon_feedback())
        .child(svg().path(icon(name)).size(px(10.)).text_color(rgb(if enabled { theme::TEXT } else { theme::TEXT_MUTED })))
}

fn entry_icon(entry: &Entry) -> impl IntoElement {
    let (name, color) = if entry.is_link {
        ("link", FOLDER)
    } else if entry.is_dir {
        ("folder", FOLDER)
    } else {
        ("file", theme::TEXT)
    };
    svg().path(icon(name)).size(px(18.)).flex_none().text_color(rgb(color))
}

fn button(id: &str, label: &str, primary: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .px_4()
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(if primary { 0xffffff } else { theme::TEXT }))
        .bg(rgb(if primary { theme::ACCENT } else { theme::SURFACE }))
        .surface_feedback(if primary { theme::ACCENT } else { theme::SURFACE })
        .child(label.to_owned())
}

/// Centered column used by every non-listing state of the remote side.
fn centered() -> Div {
    div().h_full().flex().flex_col().items_center().justify_center().gap_3().px_8()
}

impl Shell {
    pub fn render_sftp(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let panels = self.render_panels(window, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(panels.flex_1().min_h_0())
            .children(self.render_edits_bar(cx))
            .children(self.render_transfer_bar(cx))
    }

    fn render_side(&self, side: Side, window: &Window, cx: &mut Context<Self>) -> Div {
        let s = self.sftp.side(side);
        if !s.host {
            return self.render_panel(side, &s.local, s.local_error.as_deref(), window, cx);
        }
        match &s.state {
            RemoteState::Connected { panel, error } => self.render_panel(side, panel, error.as_deref(), window, cx),
            RemoteState::Idle => self.render_connect_placeholder(side, None, cx),
            RemoteState::Failed(reason) => self.render_connect_placeholder(side, Some(reason), cx),
            RemoteState::Picking { query, group } => self.render_host_picker(side, query, *group, window, cx),
            RemoteState::Connecting(login) => self.render_sftp_login(side, login, window, cx),
        }
    }

    fn render_panels(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        div()
            .w_full()
            .flex()
            .child(self.render_side(Side::Left, window, cx).w_1_2().border_r_1().border_color(rgb(theme::TITLE_BAR)))
            .child(self.render_side(Side::Right, window, cx).w_1_2())
    }

    /// "Connect with SFTP" from a host card: the SFTP tab, the right panel on that host.
    pub fn open_sftp_for(&mut self, host: HostId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.all().iter().find(|t| matches!(t.kind, termian_native::tabs::TabKind::Sftp)).map(|t| t.id) {
            self.tabs.activate(tab);
        }
        self.disconnect_sftp(Side::Right);
        self.connect_sftp(Side::Right, host, window, cx);
        cx.notify();
    }

    pub(crate) fn connect_sftp(&mut self, side: Side, host: HostId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.vault.hosts.iter().find(|h| h.id == host) else { return };
        let target = SshTarget::for_host(entry, &self.vault);
        let (label, os) = (entry.display_label().to_owned(), entry.os.clone());
        self.connect_sftp_target(side, label, os, target, window, cx);
    }

    fn connect_sftp_target(&mut self, side: Side, label: String, os: Option<String>, mut target: SshTarget, window: &mut Window, cx: &mut Context<Self>) {
        target.options = termian_native::session::SshOptions::from_vault(&self.vault);
        let mut connection = RemoteFs::connect(target.clone());
        let panel = self.sftp.side_mut(side);
        panel.generation += 1;
        let generation = panel.generation;
        if let Some(mut events) = connection.take_events() {
            cx.spawn_in(window, async move |this, cx| {
                while let Some(event) = events.next().await {
                    let asks_text = matches!(event, RemoteEvent::Login(LoginEvent::Prompt { .. }));
                    let alive = this.update_in(cx, |shell, window, cx| {
                        if shell.sftp.side(side).generation != generation {
                            return; // a newer connection replaced this one
                        }
                        match event {
                            RemoteEvent::Names { request, result } => shell.on_names(request, result, window, cx),
                            event => shell.sftp.handle(side, event),
                        }
                        if asks_text {
                            window.focus(&shell.sftp_answer_focus);
                        }
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        let panel = self.sftp.side_mut(side);
        panel.host = true;
        panel.remote = Some(Remote { label, os, target, connection });
        panel.state = RemoteState::Connecting(LoginState::default());
    }

    /// Closes a side's connection: the left panel goes back to the disk,
    /// the right one to «Connect to host».
    pub(super) fn disconnect_sftp(&mut self, side: Side) {
        self.sftp.forget_remote_work(side);
        let panel = self.sftp.side_mut(side);
        panel.generation += 1;
        panel.remote = None; // dropping the connection closes it
        panel.state = RemoteState::Idle;
        if side == Side::Left {
            panel.host = false;
            panel.reload_local();
        }
    }

    /// The legacy green «Local» button: this panel shows the disk.
    fn show_local(&mut self, side: Side) {
        self.disconnect_sftp(side);
        let panel = self.sftp.side_mut(side);
        panel.host = false;
        panel.reload_local();
    }

    /// Back from choosing a host: the left panel shows the disk again.
    fn cancel_pick(&mut self, side: Side) {
        let panel = self.sftp.side_mut(side);
        panel.state = RemoteState::Idle;
        if side == Side::Left && panel.remote.is_none() {
            panel.host = false;
        }
    }

    fn render_connect_placeholder(&self, side: Side, error: Option<&str>, cx: &mut Context<Self>) -> Div {
        let key = side.key();
        centered()
            .child(
                div()
                    .size(px(60.))
                    .rounded_xl()
                    .bg(rgb(theme::SURFACE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().path(icon("sftp")).size(px(26.)).text_color(rgb(theme::TEXT))),
            )
            .child(div().pt_4().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("Connect to host"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child("Start by connecting to a saved host to manage your files with SFTP."),
            )
            .when_some(error, |d, error| d.child(div().text_sm().text_color(rgb(theme::TILE_UBUNTU)).child(error.to_owned())))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button(&format!("select-host-{key}"), "Select host", false).on_click(cx.listener(move |shell, _, _, cx| {
                        shell.sftp.side_mut(side).state = RemoteState::Picking { query: String::new(), group: None };
                        cx.notify();
                    })))
                    .when(side == Side::Left, |d| {
                        d.child(button("show-local", "Local", false).on_click(cx.listener(|shell, _, _, cx| {
                            shell.disconnect_sftp(Side::Left);
                            cx.notify();
                        })))
                    }),
            )
    }

    /// Enter in the picker: first matching saved host, else a typed `user@host`.
    fn pick_query(&mut self, side: Side, window: &mut Window, cx: &mut Context<Self>) {
        let RemoteState::Picking { query, .. } = &self.sftp.side(side).state else { return };
        let query = query.clone();
        if query.trim().is_empty() {
            return;
        }
        if let Some(host) = self.vault.search(&query).first().map(|h| h.id) {
            self.connect_sftp(side, host, window, cx);
        } else if let Some(target) = termian_native::session::parse_destination(&query) {
            let label = query.trim().trim_start_matches("ssh ").to_owned();
            self.connect_sftp_target(side, label, None, target, window, cx);
        }
    }

    fn on_pick_key(&mut self, side: Side, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.key == "enter" {
            self.pick_query(side, window, cx);
        } else {
            if keystroke.key == "escape" {
                self.cancel_pick(side);
            } else {
                let RemoteState::Picking { query, .. } = &mut self.sftp.side_mut(side).state else { return };
                if matches!(text_input::edit(("sftp-picker", side.key()), Some(query), keystroke, Mode::Line, cx), super::dialog::Key::Ignored) {
                    return;
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The legacy «Select Host» panel: back, «Vaults», the green Local
    /// button, search, then Groups (opened in place, with breadcrumbs) and
    /// the hosts of the open group, as on the Hosts screen.
    fn render_host_picker(&self, side: Side, query: &str, group: Option<GroupId>, window: &Window, cx: &mut Context<Self>) -> Div {
        let key = side.key();
        let focused = self.sftp_answer_focus.is_focused(window);
        let set_group = move |target: Option<GroupId>| {
            move |shell: &mut Shell, _: &ClickEvent, _: &mut Window, cx: &mut Context<Shell>| {
                if let RemoteState::Picking { group, .. } = &mut shell.sftp.side_mut(side).state {
                    *group = target;
                }
                cx.notify();
            }
        };
        let header = div()
            .h(px(64.))
            .flex_none()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .bg(rgb(theme::SIDEBAR))
            .child(
                div()
                    .id(SharedString::from(format!("pick-back-{key}")))
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .child(svg().path(icon("arrow_left")).size(px(14.)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.cancel_pick(side);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("Select Host"))
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Vaults")),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id(SharedString::from(format!("pick-local-{key}")))
                    .h(px(30.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .bg(rgb(theme::ACCENT))
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(0xffffff))
                    .child(svg().path(icon("local")).size(px(14.)).text_color(rgb(0xffffff)))
                    .child("Local")
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.show_local(side);
                        cx.notify();
                    })),
            );
        let field = div()
            .id(SharedString::from(format!("pick-query-{key}")))
            .track_focus(&self.sftp_answer_focus)
            .on_key_down(cx.listener(move |shell, event: &KeyDownEvent, window, cx| shell.on_pick_key(side, event, window, cx)))
            .h(px(44.))
            .flex_none()
            .px_4()
            .flex()
            .items_center()
            .gap_2()
            .bg(rgb(theme::SIDEBAR))
            .border_t_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(svg().path(icon("search")).size(px(16.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(text_input::text(("sftp-picker", key), query, "Search", focused, Mode::Line, cx));
        let field = text_input::field(field, ("sftp-picker", key), Mode::Line, |shell, window, _| window.focus(&shell.sftp_answer_focus), cx);
        let row = |id: String, tile: Div, title: String, subtitle: String| {
            div()
                .id(SharedString::from(id))
                .h(px(64.))
                .flex_none()
                .px_3()
                .flex()
                .items_center()
                .gap_3()
                .rounded_lg()
                .hover(|s| s.bg(rgb(theme::SURFACE)))
                .child(tile)
                .child(div().flex().flex_col().child(div().text_color(rgb(theme::TEXT)).child(title)).child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(subtitle)))
        };
        let title = |text: &str| div().px_3().pt_4().pb_2().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(text.to_owned());
        let mut list = div().flex().flex_col();
        let searching = !query.trim().is_empty();
        if !searching {
            if let Some(open) = group {
                let mut crumbs = div().px_3().pt_4().flex().items_center().gap_2().text_sm().child(
                    div().id(SharedString::from(format!("pick-crumb-root-{key}"))).text_color(rgb(theme::ACCENT)).child("All hosts").on_click(cx.listener(set_group(None))),
                );
                for g in self.vault.breadcrumbs(Some(open)) {
                    crumbs = crumbs.child(div().text_color(rgb(theme::TEXT_MUTED)).child("›")).child(
                        div()
                            .id(SharedString::from(format!("pick-crumb-{key}-{}", g.id)))
                            .text_color(rgb(if g.id == open { theme::TEXT } else { theme::ACCENT }))
                            .child(g.label.clone())
                            .on_click(cx.listener(set_group(Some(g.id)))),
                    );
                }
                list = list.child(crumbs);
            }
            let groups = self.vault.child_groups(group);
            if !groups.is_empty() {
                list = list.child(title("Groups"));
                for g in groups {
                    let count = self.vault.hosts_under(Some(g.id)).len();
                    list = list.child(
                        row(format!("pick-group-{key}-{}", g.id), super::hosts::tile("group", theme::TILE_BLUE), g.label.clone(), format!("{count} {}", if count == 1 { "Host" } else { "Hosts" }))
                            .on_click(cx.listener(set_group(Some(g.id)))),
                    );
                }
            }
        }
        let hosts = if searching { self.vault.search(query) } else { self.vault.hosts_under(group) };
        if !hosts.is_empty() {
            list = list.child(title("Hosts"));
            for host in hosts {
                let id = host.id;
                list = list.child(row(format!("pick-{key}-{id}"), host_tile(host), host.display_label().to_owned(), host.subtitle()).on_click(cx.listener(move |shell, _, window, cx| {
                    shell.connect_sftp(side, id, window, cx);
                    cx.notify();
                })));
            }
        }
        div()
            .h_full()
            .flex()
            .flex_col()
            .child(header)
            .child(field)
            .child(div().id(SharedString::from(format!("pick-list-{key}"))).flex_1().overflow_y_scroll().px_3().pb_3().child(list))
    }

    fn render_sftp_login(&self, side: Side, login: &LoginState, window: &Window, cx: &mut Context<Self>) -> Div {
        let Some(remote) = &self.sftp.side(side).remote else { return div() };
        let target = LoginTarget {
            label: &remote.label,
            subtitle: format!("SFTP {}:{}", remote.target.host, remote.target.port),
            os: remote.os.as_deref(),
            identities: &[],
        };
        // The login screen's callbacks are plain functions: one set per side.
        fn actions<const LEFT: bool>() -> LoginActions<Shell> {
            LoginActions {
                answer: |shell: &mut Shell, answer, _, cx| {
                    shell.sftp_login_answer(if LEFT { Side::Left } else { Side::Right }, answer);
                    cx.notify();
                },
                close: |shell: &mut Shell, _, cx| {
                    shell.disconnect_sftp(if LEFT { Side::Left } else { Side::Right });
                    cx.notify();
                },
                retry: |shell: &mut Shell, window, cx| {
                    let side = if LEFT { Side::Left } else { Side::Right };
                    if let Some(remote) = shell.sftp.side_mut(side).remote.take() {
                        shell.connect_sftp_target(side, remote.label, remote.os, remote.target, window, cx);
                    }
                    cx.notify();
                },
                toggle_logs: |shell: &mut Shell, cx| {
                    if let RemoteState::Connecting(login) = &mut shell.sftp.side_mut(if LEFT { Side::Left } else { Side::Right }).state {
                        login.show_logs = !login.show_logs;
                    }
                    cx.notify();
                },
                key: |shell: &mut Shell, event, _, cx| {
                    let side = if LEFT { Side::Left } else { Side::Right };
                    let RemoteState::Connecting(login) = &mut shell.sftp.side_mut(side).state else { return };
                    if let Some(answer) = login_view::edit(login, event, cx) {
                        if let Some(answer) = answer {
                            shell.sftp_login_answer(side, answer);
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }
                },
                submit: |shell: &mut Shell, _, cx| {
                    let side = if LEFT { Side::Left } else { Side::Right };
                    if let RemoteState::Connecting(login) = &mut shell.sftp.side_mut(side).state {
                        if let Some(answer) = login.submit_text() {
                            shell.sftp_login_answer(side, answer);
                        }
                    }
                    cx.notify();
                },
                edit_host: None,
                verbose: None,
                continue_save: None,
                ui: None,
            }
        }
        let actions = if side == Side::Left { actions::<true>() } else { actions::<false>() };
        login_view::render(target, login, &self.sftp_answer_focus, window, &actions, cx)
    }

    /// Sends an answer to the pending login question; cancelling disconnects.
    fn sftp_login_answer(&mut self, side: Side, answer: Answer) {
        let cancels = matches!(answer, Answer::Text(None) | Answer::HostKey(HostKeyDecision::Reject));
        let panel = self.sftp.side_mut(side);
        if let (Some(remote), RemoteState::Connecting(login)) = (&panel.remote, &mut panel.state) {
            if let Some(answer) = login.answer(answer) {
                remote.connection.answer(answer);
            }
        }
        if cancels {
            self.disconnect_sftp(side);
        }
    }

    fn render_panel_title(&self, side: Side, cx: &mut Context<Self>) -> Div {
        let key = side.key();
        let row = div().flex().items_center().gap_3();
        match self.sftp.remote(side) {
            // The disk: on the left, a click chooses a host instead (host-to-host copies).
            None => row
                .child(
                    div()
                        .size(px(26.))
                        .rounded_md()
                        .bg(rgb(theme::TILE_BLUE))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(svg().path(icon("local")).size(px(14.)).text_color(rgb(0xffffff))),
                )
                .child(div().text_color(rgb(theme::TEXT)).child("Local"))
                .child({
                    let row = div();
                    row.child(
                        div()
                            .id(SharedString::from(format!("choose-host-{key}")))
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(theme::SURFACE_ACTIVE))
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(theme::TEXT))
                            .child("DISCONNECT")
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                let panel = shell.sftp.side_mut(side);
                                panel.host = true;
                                panel.state = RemoteState::Picking { query: String::new(), group: None };
                                cx.notify();
                            })),
                    )
                }),
            Some(remote) => row.child(os_tile(remote.os.as_deref()).size(px(26.))).child(div().text_color(rgb(theme::TEXT)).child(remote.label.clone())).child(
                div()
                    .id(SharedString::from(format!("disconnect-{key}")))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(rgb(theme::SURFACE_ACTIVE))
                    .surface_feedback(theme::SURFACE_ACTIVE)
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT))
                    .child("DISCONNECT")
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        shell.request_disconnect(side, window);
                        cx.notify();
                    })),
            ),
        }
    }

    fn render_panel(&self, side: Side, panel: &Panel, error: Option<&str>, window: &Window, cx: &mut Context<Self>) -> Div {
        let key = side.key();
        let filtering = self.sftp.filtering == Some(side) && self.sftp_filter_focus.is_focused(window);
        let input = div().id(SharedString::from(format!("filter-{key}")))
            .when(self.sftp.filtering == Some(side), |d| d.track_focus(&self.sftp_filter_focus))
            .on_key_down(cx.listener(move |shell, event: &KeyDownEvent, _, cx| shell.on_filter_key(side, event, cx)))
            .min_w(px(80.)).h(px(28.)).px_2().flex().items_center().gap_2().rounded_md()
            .when(filtering, |d| d.bg(rgb(theme::INPUT)).border_1().border_color(rgb(theme::ACCENT)))
            .child(svg().path(icon("search")).size(px(16.)).text_color(rgb(theme::TEXT)))
            .child(text_input::text(("sftp-filter", key), &panel.filter, if filtering { "" } else { "Filter" }, filtering, Mode::Line, cx));
        let filter = text_input::field(input, ("sftp-filter", key), Mode::Line, move |shell, window, _| {
            shell.sftp.filtering = Some(side);
            window.focus(&shell.sftp_filter_focus);
        }, cx);
        let header = div()
            .h(px(56.))
            .flex_none()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .bg(rgb(theme::SIDEBAR))
            .child(self.render_panel_title(side, cx))
            .child(div().flex_1())
            .child(filter)
            .child(self.render_actions_button(side, cx));

        let navigation = div()
            .h(px(44.))
            .flex_none()
            .px_3()
            .flex()
            .items_center()
            .gap_1()
            .bg(rgb(theme::SIDEBAR))
            .child(icon_button(format!("back-{key}"), "arrow_left", panel.can_go_back()).on_click(cx.listener(
                move |shell, _, _, cx| {
                    if shell.sftp.panel_mut(side).is_some_and(|p| p.go_back()) {
                        shell.sftp.reload(side);
                    }
                    cx.notify();
                },
            )))
            .child(icon_button(format!("forward-{key}"), "arrow_right", panel.can_go_forward()).on_click(cx.listener(
                move |shell, _, _, cx| {
                    if shell.sftp.panel_mut(side).is_some_and(|p| p.go_forward()) {
                        shell.sftp.reload(side);
                    }
                    cx.notify();
                },
            )))
            // Long paths are clipped here instead of spilling into the other panel.
            .child(div().flex_1().min_w_0().overflow_hidden().flex().items_center().gap_1().children(self.render_crumbs(side, &panel.path, cx)));

        let columns = div()
            .h(px(40.))
            .flex_none()
            .px_4()
            .flex()
            .items_center()
            .bg(rgb(theme::SIDEBAR))
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .text_sm()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(theme::TEXT))
            .children(COLUMNS.map(|(column, label, width)| {
                // Legacy: a caret beside the sorted column, pointing down when descending.
                let marker = (panel.sort == column).then(|| {
                    let caret = svg().path(icon("caret_down")).w(px(9.)).h(px(6.)).text_color(rgb(theme::TEXT));
                    if panel.descending { caret } else { caret.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5))) }
                });
                div()
                    .id(SharedString::from(format!("sort-{key}-{label}")))
                    .when(width == 0., |d| d.flex_1())
                    .when(width > 0., |d| d.w(px(width)).flex_none())
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(label)
                    .children(marker)
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(panel) = shell.sftp.panel_mut(side) {
                            panel.sort_by(column);
                        }
                        cx.notify();
                    }))
            }));

        let mut rows = div()
            .id(SharedString::from(format!("rows-{key}")))
            .flex_1()
            .overflow_y_scroll()
            .on_click(cx.listener(move |shell, _, _, cx| {
                if let Some(panel) = shell.sftp.panel_mut(side) {
                    panel.clear_selection();
                }
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                shell.open_menu(side, event.position, MenuKind::Context, cx);
            }));
        if let Some(error) = error {
            rows = rows.child(div().p_4().text_color(rgb(theme::TILE_UBUNTU)).child(error.to_owned()));
        }
        if panel.path != "/" {
            rows = rows.child(self.render_row(side, None, false, cx));
        }
        for entry in panel.rows() {
            let selected = panel.is_selected(&entry.name);
            rows = rows.child(self.render_row(side, Some(entry), selected, cx));
        }

        // Rows dropped from the other panel, or files from Finder onto the remote one, are copied here.
        let panel = div()
            .id(SharedString::from(format!("panel-{key}")))
            .size_full()
            .flex()
            .flex_col()
            .border_2()
            .border_color(gpui::transparent_black())
            .drag_over::<DraggedRows>(move |style, dragged, _, _| {
                if dragged.side != side { style.border_color(rgb(theme::ACCENT)) } else { style }
            })
            .on_drop(cx.listener(move |shell, dragged: &DraggedRows, window, cx| shell.drop_rows(dragged, side, window, cx)))
            .when(self.sftp.is_host(side), |d| {
                d.drag_over::<ExternalPaths>(|style, _, _, _| style.border_color(rgb(theme::ACCENT)))
                    .on_drop(cx.listener(move |shell, paths: &ExternalPaths, window, cx| shell.drop_paths(side, paths, window, cx)))
            })
            .child(header)
            .child(navigation)
            .child(columns)
            .child(rows);
        div().h_full().child(panel)
    }

    fn render_crumbs(&self, side: Side, path: &str, cx: &mut Context<Self>) -> Vec<Stateful<Div>> {
        let mut all = crumbs(path);
        // As in legacy, "/" shows only at the root itself.
        if all.len() > 1 {
            all.remove(0);
        }
        // Long paths keep the last three folders, like the legacy breadcrumb.
        let skip = all.len().saturating_sub(3);
        let mut out = Vec::new();
        if skip > 0 {
            out.push(div().id(SharedString::from(format!("crumb-more-{}", side.key()))).px_1().text_color(rgb(theme::TEXT)).child("…"));
        }
        for (i, (label, target)) in all.into_iter().enumerate().skip(skip) {
            if i > skip || skip > 0 {
                out.push(div().id(SharedString::from(format!("crumb-sep-{}-{i}", side.key()))).flex_none().child(svg().path(icon("arrow_right")).size(px(8.)).text_color(rgb(theme::TEXT_MUTED))));
            }
            out.push(
                div()
                    .id(SharedString::from(format!("crumb-{}-{i}", side.key())))
                    .min_w_0()
                    .px_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .text_color(rgb(theme::TEXT))
                    .child(svg().path(icon("folder")).size(px(16.)).flex_none().text_color(rgb(FOLDER)))
                    .child(div().min_w_0().truncate().child(label))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(panel) = shell.sftp.panel_mut(side) {
                            panel.navigate(target.clone());
                            shell.sftp.reload(side);
                        }
                        cx.notify();
                    })),
            );
        }
        out
    }

    fn render_row(&self, side: Side, entry: Option<&Entry>, selected: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let name = entry.map(|e| e.name.clone()).unwrap_or_else(|| "..".into());
        let text = if selected { 0xffffff } else { theme::TEXT };
        let muted = if selected { 0xffffff } else { theme::TEXT_MUTED };
        let cell = |width: f32, content: String| div().w(px(width)).flex_none().text_color(rgb(muted)).text_sm().truncate().child(content);
        let icon_el = match entry {
            Some(e) => entry_icon(e).into_any_element(),
            None => svg().path(icon("folder")).size(px(18.)).text_color(rgb(FOLDER)).into_any_element(),
        };
        // Remote rows show permissions under the name, as in the legacy panel.
        let permissions = self.sftp.is_host(side).then(|| entry.and_then(Entry::permissions_label)).flatten();
        let target = name.clone();
        let menu_target = name.clone();
        let row = div()
            .id(SharedString::from(format!("row-{}-{name}", side.key())))
            .h(px(if permissions.is_some() { 44. } else { 40. }))
            .px_4()
            .flex()
            .items_center()
            .when(selected, |d| d.bg(rgb(theme::ACCENT)).surface_feedback(theme::ACCENT))
            .when(!selected, |d| d.icon_feedback())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .pr_2()
                    .flex()
                    .items_center()
                    .gap_3()
                    .overflow_hidden()
                    .child(icon_el)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(div().text_color(rgb(text)).truncate().child(name.clone()))
                            .when_some(permissions, |d, p| d.child(div().text_xs().text_color(rgb(muted)).child(p))),
                    ),
            )
            .child(cell(175., entry.map(Entry::modified_label).unwrap_or_default()))
            .child(cell(115., entry.map(Entry::size_label).unwrap_or_default()))
            .child(cell(115., entry.map(Entry::kind).unwrap_or_default()))
            .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
                cx.stop_propagation();
                let modifiers = event.modifiers();
                let file = shell.sftp.panel(side).and_then(|p| p.entry(&target)).filter(|e| !e.is_dir).cloned();
                if matches!(event, ClickEvent::Keyboard(key) if key.button == gpui::KeyboardButton::Enter) || event.click_count() >= 2 {
                    // Double click opens folders here and files in their app, as in the legacy panel.
                    match file {
                        Some(entry) => shell.open_entry(side, &entry, None, window, cx),
                        None => shell.sftp.open(side, &target),
                    }
                } else if target != ".." {
                    if let Some(panel) = shell.sftp.panel_mut(side) {
                        // Cmd adds a row on macOS, Ctrl elsewhere; Shift selects a range.
                        if modifiers.secondary() {
                            panel.toggle(&target);
                        } else if modifiers.shift {
                            panel.select_range(&target);
                        } else {
                            panel.select(&target);
                        }
                    }
                }
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                if menu_target != ".." {
                    if let Some(panel) = shell.sftp.panel_mut(side) {
                        if !panel.is_selected(&menu_target) {
                            panel.select(&menu_target);
                        }
                    }
                }
                shell.open_menu(side, event.position, MenuKind::Context, cx);
            }));
        match entry {
            Some(_) => row.on_drag(self.drag_value(side, &name), |value, _, _, cx| Shell::drag_preview(value, cx)),
            None => row,
        }
    }

    fn on_filter_key(&mut self, side: Side, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let Some(panel) = self.sftp.panel_mut(side) else { return };
        use super::dialog::Key;
        match text_input::edit(("sftp-filter", side.key()), Some(&mut panel.filter), keystroke, Mode::Line, cx) {
            Key::Escape => {
                panel.filter.clear();
                self.sftp.filtering = None;
            }
            Key::Enter => self.sftp.filtering = None,
            Key::Edited => {}
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }
}
