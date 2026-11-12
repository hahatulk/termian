//! SFTP tab: two file panels side by side. The left one is the local disk;
//! the right one shows a remote host once connected.

use futures::StreamExt;
use gpui::{div, prelude::*, px, rgb, svg, ClickEvent, Context, Div, KeyDownEvent, SharedString, Stateful, Window};
use termian_native::files::{crumbs, join, Entry, Panel, SortKey};
use termian_native::remote_fs::{RemoteEvent, RemoteFs};
use termian_native::session::SshTarget;
use termian_native::theme;
use termian_native::vault::HostId;

use super::assets::icon;
use super::hosts::{host_tile, os_tile};
use crate::Shell;

/// Folder blue from the legacy file icons.
const FOLDER: u32 = 0x5ab4f0;
const COLUMNS: [(SortKey, &str, f32); 4] =
    [(SortKey::Name, "Name", 0.), (SortKey::Modified, "Date Modified", 175.), (SortKey::Size, "Size", 115.), (SortKey::Kind, "Kind", 115.)];

/// Which panel an event belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Local,
    Remote,
}

impl Side {
    fn key(self) -> &'static str {
        match self {
            Side::Local => "local",
            Side::Remote => "remote",
        }
    }
}

/// Where the remote side is in its lifecycle.
pub enum RemoteState {
    Idle,
    /// Choosing a saved host, or typing `user@host` to connect directly.
    Picking { query: String },
    /// Logging in; `prompt` is the pending question, `notes` what the server said before it.
    Connecting { notes: Vec<String>, prompt: Option<(String, bool)>, answer: String },
    Connected { panel: Panel, error: Option<String> },
    Failed(String),
}

pub struct Remote {
    label: String,
    os: Option<String>,
    connection: RemoteFs,
}

pub struct SftpState {
    pub local: Panel,
    pub local_error: Option<String>,
    pub filtering: Option<Side>,
    pub remote: Option<Remote>,
    pub remote_state: RemoteState,
}

impl SftpState {
    pub fn new() -> Self {
        let home = dirs::home_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
        let mut state = Self {
            local: Panel::new(&home),
            local_error: None,
            filtering: None,
            remote: None,
            remote_state: RemoteState::Idle,
        };
        state.reload_local();
        state
    }

    pub fn reload_local(&mut self) {
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

    fn panel_mut(&mut self, side: Side) -> Option<&mut Panel> {
        match side {
            Side::Local => Some(&mut self.local),
            Side::Remote => match &mut self.remote_state {
                RemoteState::Connected { panel, .. } => Some(panel),
                _ => None,
            },
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

    fn reload(&mut self, side: Side) {
        match side {
            Side::Local => self.reload_local(),
            Side::Remote => {
                if let (Some(remote), RemoteState::Connected { panel, .. }) = (&self.remote, &self.remote_state) {
                    remote.connection.list(&panel.path);
                }
            }
        }
    }

    fn handle(&mut self, event: RemoteEvent) {
        match (event, &mut self.remote_state) {
            (RemoteEvent::Message(text), RemoteState::Connecting { notes, .. }) => notes.push(text),
            (RemoteEvent::Prompt { text, echo }, RemoteState::Connecting { prompt, answer, .. }) => {
                *prompt = Some((text, echo));
                answer.clear();
            }
            (RemoteEvent::Connected { home }, _) => {
                self.remote_state = RemoteState::Connected { panel: Panel::new(&home), error: None };
                self.reload(Side::Remote);
            }
            (RemoteEvent::Listed { path, result }, RemoteState::Connected { panel, error }) if path == panel.path => {
                match result {
                    Ok(entries) => {
                        panel.set_entries(entries);
                        *error = None;
                    }
                    Err(message) => {
                        panel.set_entries(Vec::new());
                        *error = Some(message);
                    }
                }
            }
            (RemoteEvent::Closed(reason), _) => {
                self.remote = None;
                self.remote_state = match reason {
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
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .when(enabled, |d| d.hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE))))
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
        .child(label.to_owned())
}

/// Centered column used by every non-listing state of the remote side.
fn centered() -> Div {
    div().h_full().flex().flex_col().items_center().justify_center().gap_3().px_8()
}

impl Shell {
    pub fn render_sftp(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let local = self.render_panel(Side::Local, &self.sftp.local, self.sftp.local_error.as_deref(), window, cx);
        let remote = match &self.sftp.remote_state {
            RemoteState::Connected { panel, error } => self.render_panel(Side::Remote, panel, error.as_deref(), window, cx),
            RemoteState::Idle => self.render_connect_placeholder(None, cx),
            RemoteState::Failed(reason) => self.render_connect_placeholder(Some(reason), cx),
            RemoteState::Picking { query } => self.render_host_picker(query, window, cx),
            RemoteState::Connecting { notes, prompt, answer } => self.render_login(notes, prompt.as_ref(), answer, window, cx),
        };
        div()
            .size_full()
            .flex()
            .child(local.w_1_2().border_r_1().border_color(rgb(theme::TITLE_BAR)))
            .child(remote.w_1_2())
    }

    fn connect_sftp(&mut self, host: HostId, cx: &mut Context<Self>) {
        let Some(entry) = self.vault.hosts.iter().find(|h| h.id == host) else { return };
        let target = SshTarget::new(&entry.address, entry.username.clone());
        let (label, os) = (entry.label.clone(), entry.os.clone());
        self.connect_sftp_target(label, os, target, cx);
    }

    fn connect_sftp_target(&mut self, label: String, os: Option<String>, target: SshTarget, cx: &mut Context<Self>) {
        let mut connection = RemoteFs::connect(target);
        if let Some(mut events) = connection.take_events() {
            cx.spawn(async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update(cx, |shell, cx| {
                        shell.sftp.handle(event);
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        self.sftp.remote = Some(Remote { label, os, connection });
        self.sftp.remote_state = RemoteState::Connecting { notes: Vec::new(), prompt: None, answer: String::new() };
    }

    fn disconnect_sftp(&mut self) {
        self.sftp.remote = None; // dropping the connection closes it
        self.sftp.remote_state = RemoteState::Idle;
    }

    fn render_connect_placeholder(&self, error: Option<&str>, cx: &mut Context<Self>) -> Div {
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
            .child(button("select-host", "Select host", false).on_click(cx.listener(|shell, _, _, cx| {
                shell.sftp.remote_state = RemoteState::Picking { query: String::new() };
                cx.notify();
            })))
    }

    /// Enter in the picker: first matching saved host, else a typed `user@host`.
    fn pick_query(&mut self, cx: &mut Context<Self>) {
        let RemoteState::Picking { query } = &self.sftp.remote_state else { return };
        let query = query.clone();
        if let Some(host) = self.vault.search(&query).first().map(|h| h.id) {
            self.connect_sftp(host, cx);
        } else if let Some(target) = termian_native::session::parse_destination(&query) {
            let label = query.trim().trim_start_matches("ssh ").to_owned();
            self.connect_sftp_target(label, None, target, cx);
        }
    }

    fn on_pick_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.key == "enter" {
            self.pick_query(cx);
        } else {
            let RemoteState::Picking { query } = &mut self.sftp.remote_state else { return };
            match keystroke.key.as_str() {
                _ if keystroke.modifiers.platform || keystroke.modifiers.control => return,
                "backspace" => {
                    query.pop();
                }
                "escape" => self.sftp.remote_state = RemoteState::Idle,
                _ => match keystroke.key_char.as_deref() {
                    Some(text) if !text.chars().any(char::is_control) => query.push_str(text),
                    _ => return,
                },
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_host_picker(&self, query: &str, window: &Window, cx: &mut Context<Self>) -> Div {
        let focused = self.sftp_answer_focus.is_focused(window);
        let field = div()
            .id("pick-query")
            .track_focus(&self.sftp_answer_focus)
            .on_key_down(cx.listener(Self::on_pick_key))
            .on_click(cx.listener(|shell, _, window, cx| {
                window.focus(&shell.sftp_answer_focus);
                cx.notify();
            }))
            .mx_3()
            .mt_3()
            .h(px(36.))
            .px_3()
            .flex()
            .items_center()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
            .text_color(rgb(if query.is_empty() { theme::TEXT_MUTED } else { theme::TEXT }))
            .child(if query.is_empty() { "Find a host or user@hostname...".to_owned() } else { query.to_owned() })
            .when(focused, |d| d.child(div().mx(px(1.)).w(px(1.5)).h(px(16.)).bg(rgb(theme::TEXT))));
        let hosts = self.vault.search(query).into_iter().map(|host| {
            let id = host.id;
            div()
                .id(SharedString::from(format!("pick-{id}")))
                .h(px(60.))
                .flex_none()
                .px_3()
                .flex()
                .items_center()
                .gap_3()
                .rounded_lg()
                .hover(|s| s.bg(rgb(theme::SURFACE)))
                .child(host_tile(host))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_color(rgb(theme::TEXT)).child(host.label.clone()))
                        .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(host.subtitle())),
                )
                .on_click(cx.listener(move |shell, _, _, cx| {
                    shell.connect_sftp(id, cx);
                    cx.notify();
                }))
        });
        div()
            .h_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(56.))
                    .px_4()
                    .flex()
                    .items_center()
                    .bg(rgb(theme::SIDEBAR))
                    .text_color(rgb(theme::TEXT))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Select host")
                    .child(div().flex_1())
                    .child(button("pick-cancel", "Cancel", false).on_click(cx.listener(|shell, _, _, cx| {
                        shell.sftp.remote_state = RemoteState::Idle;
                        cx.notify();
                    }))),
            )
            .child(field)
            .child(div().id("pick-list").flex_1().overflow_y_scroll().p_3().flex().flex_col().gap_1().children(hosts))
    }

    fn render_login(
        &self,
        notes: &[String],
        prompt: Option<&(String, bool)>,
        answer: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let title = self.sftp.remote.as_ref().map(|r| format!("Connecting to {}…", r.label)).unwrap_or_else(|| "Connecting…".into());
        let mut body = centered()
            .child(div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title))
            .children(notes.iter().map(|note| div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(note.clone())));
        if let Some((question, echo)) = prompt {
            let focused = self.sftp_answer_focus.is_focused(window);
            let shown = if *echo { answer.to_owned() } else { "•".repeat(answer.chars().count()) };
            body = body
                .child(div().w_full().text_sm().text_color(rgb(theme::TEXT)).child(question.clone()))
                .child(
                    div()
                        .id("answer")
                        .track_focus(&self.sftp_answer_focus)
                        .on_key_down(cx.listener(Self::on_answer_key))
                        .on_click(cx.listener(|shell, _, window, cx| {
                            window.focus(&shell.sftp_answer_focus);
                            cx.notify();
                        }))
                        .w_full()
                        .h(px(36.))
                        .px_3()
                        .flex()
                        .items_center()
                        .rounded_md()
                        .bg(rgb(theme::INPUT))
                        .border_1()
                        .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
                        .text_color(rgb(theme::TEXT))
                        .child(shown)
                        .when(focused, |d| d.child(div().mx(px(1.)).w(px(1.5)).h(px(16.)).bg(rgb(theme::TEXT)))),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(button("answer-cancel", "Cancel", false).on_click(cx.listener(|shell, _, _, cx| {
                            shell.answer_prompt(false);
                            cx.notify();
                        })))
                        .child(button("answer-ok", "Continue", true).on_click(cx.listener(|shell, _, _, cx| {
                            shell.answer_prompt(true);
                            cx.notify();
                        }))),
                );
        }
        body
    }

    /// Sends the typed answer (or a cancel) to the pending login question.
    fn answer_prompt(&mut self, submit: bool) {
        if let (Some(remote), RemoteState::Connecting { prompt, answer, .. }) = (&self.sftp.remote, &mut self.sftp.remote_state) {
            remote.connection.answer(submit.then(|| std::mem::take(answer)));
            *prompt = None;
        }
    }

    fn on_answer_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let RemoteState::Connecting { answer, .. } = &mut self.sftp.remote_state else { return };
        match keystroke.key.as_str() {
            _ if keystroke.modifiers.platform && keystroke.key == "v" => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    answer.push_str(text.lines().next().unwrap_or(""));
                }
            }
            _ if keystroke.modifiers.platform || keystroke.modifiers.control => return,
            "backspace" => {
                answer.pop();
            }
            "enter" => self.answer_prompt(true),
            "escape" => self.answer_prompt(false),
            _ => match keystroke.key_char.as_deref() {
                Some(text) if !text.chars().any(char::is_control) => answer.push_str(text),
                _ => return,
            },
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_panel_title(&self, side: Side, cx: &mut Context<Self>) -> Div {
        let row = div().flex().items_center().gap_3();
        match side {
            Side::Local => row
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
                .child(div().text_color(rgb(theme::TEXT)).child("Local")),
            Side::Remote => {
                row.when_some(self.sftp.remote.as_ref(), |row, remote| {
                    row.child(os_tile(remote.os.as_deref()).size(px(26.))).child(div().text_color(rgb(theme::TEXT)).child(remote.label.clone()))
                })
                    .child(
                        div()
                            .id("disconnect")
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(rgb(theme::SURFACE_ACTIVE))
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(theme::TEXT))
                            .child("DISCONNECT")
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.disconnect_sftp();
                                cx.notify();
                            })),
                    )
            }
        }
    }

    fn render_panel(&self, side: Side, panel: &Panel, error: Option<&str>, window: &Window, cx: &mut Context<Self>) -> Div {
        let key = side.key();
        let filtering = self.sftp.filtering == Some(side) && self.sftp_filter_focus.is_focused(window);
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
            .child(
                div()
                    .id(SharedString::from(format!("filter-{key}")))
                    .when(self.sftp.filtering == Some(side), |d| d.track_focus(&self.sftp_filter_focus))
                    .on_key_down(cx.listener(move |shell, event: &KeyDownEvent, _, cx| shell.on_filter_key(side, event, cx)))
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        shell.sftp.filtering = Some(side);
                        window.focus(&shell.sftp_filter_focus);
                        cx.notify();
                    }))
                    .min_w(px(80.))
                    .h(px(28.))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .when(filtering, |d| d.bg(rgb(theme::INPUT)).border_1().border_color(rgb(theme::ACCENT)))
                    .child(svg().path(icon("search")).size(px(16.)).text_color(rgb(theme::TEXT)))
                    .child(
                        div()
                            .text_color(rgb(if panel.filter.is_empty() { theme::TEXT } else { theme::ACCENT }))
                            .child(if panel.filter.is_empty() && !filtering { "Filter".to_owned() } else { panel.filter.clone() }),
                    ),
            );

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
            .children(self.render_crumbs(side, &panel.path, cx));

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
                let marker = (panel.sort == column).then(|| if panel.descending { " ˅" } else { " ˄" }).unwrap_or("");
                div()
                    .id(SharedString::from(format!("sort-{key}-{label}")))
                    .when(width == 0., |d| d.flex_1())
                    .when(width > 0., |d| d.w(px(width)).flex_none())
                    .child(format!("{label}{marker}"))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(panel) = shell.sftp.panel_mut(side) {
                            panel.sort_by(column);
                        }
                        cx.notify();
                    }))
            }));

        let mut rows = div().id(SharedString::from(format!("rows-{key}"))).flex_1().overflow_y_scroll();
        if let Some(error) = error {
            rows = rows.child(div().p_4().text_color(rgb(theme::TILE_UBUNTU)).child(error.to_owned()));
        }
        if panel.path != "/" {
            rows = rows.child(self.render_row(side, None, false, cx));
        }
        for entry in panel.rows() {
            let selected = panel.selected.as_deref() == Some(entry.name.as_str());
            rows = rows.child(self.render_row(side, Some(entry), selected, cx));
        }

        div().h_full().flex().flex_col().child(header).child(navigation).child(columns).child(rows)
    }

    fn render_crumbs(&self, side: Side, path: &str, cx: &mut Context<Self>) -> Vec<Stateful<Div>> {
        let all = crumbs(path);
        // Long paths keep the last three folders, like the legacy breadcrumb.
        let skip = all.len().saturating_sub(3);
        let mut out = Vec::new();
        if skip > 0 {
            out.push(div().id(SharedString::from(format!("crumb-more-{}", side.key()))).px_1().text_color(rgb(theme::TEXT)).child("…"));
        }
        for (i, (label, target)) in all.into_iter().enumerate().skip(skip) {
            out.push(
                div()
                    .id(SharedString::from(format!("crumb-{}-{i}", side.key())))
                    .px_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .text_color(rgb(theme::TEXT))
                    .child(svg().path(icon("folder")).size(px(16.)).text_color(rgb(FOLDER)))
                    .child(label)
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
        let permissions = (side == Side::Remote).then(|| entry.and_then(Entry::permissions_label)).flatten();
        let target = name.clone();
        div()
            .id(SharedString::from(format!("row-{}-{name}", side.key())))
            .h(px(if permissions.is_some() { 44. } else { 40. }))
            .px_4()
            .flex()
            .items_center()
            .when(selected, |d| d.bg(rgb(theme::ACCENT)))
            .when(!selected, |d| d.hover(|s| s.bg(rgb(theme::SURFACE))))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap_3()
                    .overflow_hidden()
                    .child(icon_el)
                    .child(
                        div()
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
            .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                if event.click_count() >= 2 {
                    shell.sftp.open(side, &target);
                } else if target != ".." {
                    if let Some(panel) = shell.sftp.panel_mut(side) {
                        panel.selected = Some(target.clone());
                    }
                }
                cx.notify();
            }))
    }

    fn on_filter_key(&mut self, side: Side, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let Some(panel) = self.sftp.panel_mut(side) else { return };
        match keystroke.key.as_str() {
            _ if keystroke.modifiers.platform || keystroke.modifiers.control => return,
            "backspace" => {
                panel.filter.pop();
            }
            "escape" | "enter" => {
                if keystroke.key == "escape" {
                    panel.filter.clear();
                }
                self.sftp.filtering = None;
            }
            _ => match keystroke.key_char.as_deref() {
                Some(text) if !text.chars().any(char::is_control) => panel.filter.push_str(text),
                _ => return,
            },
        }
        cx.stop_propagation();
        cx.notify();
    }
}
