//! SFTP tab: two file panels side by side. The left one is the local disk;
//! the right one shows a remote host once connected.

use gpui::{div, prelude::*, px, rgb, svg, ClickEvent, Context, Div, KeyDownEvent, SharedString, Stateful, Window};
use termian_native::files::{crumbs, join, Entry, Panel, SortKey};
use termian_native::theme;

use super::assets::icon;
use crate::Shell;

/// Folder blue from the legacy file icons.
const FOLDER: u32 = 0x5ab4f0;
const COLUMNS: [(SortKey, &str, f32); 4] =
    [(SortKey::Name, "Name", 0.), (SortKey::Modified, "Date Modified", 175.), (SortKey::Size, "Size", 115.), (SortKey::Kind, "Kind", 115.)];

/// Which panel an event belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Local,
}

pub struct SftpState {
    pub local: Panel,
    pub local_error: Option<String>,
    pub filtering: Option<Side>,
}

impl SftpState {
    pub fn new() -> Self {
        let home = dirs::home_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
        let mut state = Self { local: Panel::new(&home), local_error: None, filtering: None };
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

    fn panel_mut(&mut self, side: Side) -> &mut Panel {
        match side {
            Side::Local => &mut self.local,
        }
    }

    /// Opens a folder (or `..`) in a panel.
    fn open(&mut self, side: Side, name: &str) {
        let panel = self.panel_mut(side);
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

impl Shell {
    pub fn render_sftp(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let local = self.render_panel(Side::Local, &self.sftp.local, self.sftp.local_error.as_deref(), window, cx);
        div()
            .size_full()
            .flex()
            .child(local.w_1_2().border_r_1().border_color(rgb(theme::TITLE_BAR)))
            .child(self.render_connect_placeholder().w_1_2())
    }

    fn render_connect_placeholder(&self) -> Div {
        div()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
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
    }

    fn render_panel(&self, side: Side, panel: &Panel, error: Option<&str>, window: &Window, cx: &mut Context<Self>) -> Div {
        let filtering = self.sftp.filtering == Some(side) && self.sftp_filter_focus.is_focused(window);
        let header = div()
            .h(px(56.))
            .flex_none()
            .px_4()
            .flex()
            .items_center()
            .gap_3()
            .bg(rgb(theme::SIDEBAR))
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
            .child(div().flex_1())
            .child(
                div()
                    .id("filter-local")
                    .track_focus(&self.sftp_filter_focus)
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
            .child(icon_button("back-local".into(), "arrow_left", panel.can_go_back()).on_click(cx.listener(
                move |shell, _, _, cx| {
                    if shell.sftp.panel_mut(side).go_back() {
                        shell.sftp.reload(side);
                    }
                    cx.notify();
                },
            )))
            .child(icon_button("forward-local".into(), "arrow_right", panel.can_go_forward()).on_click(cx.listener(
                move |shell, _, _, cx| {
                    if shell.sftp.panel_mut(side).go_forward() {
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
            .children(COLUMNS.map(|(key, label, width)| {
                let marker = (panel.sort == key).then(|| if panel.descending { " ˅" } else { " ˄" }).unwrap_or("");
                div()
                    .id(SharedString::from(format!("sort-{label}")))
                    .when(width == 0., |d| d.flex_1())
                    .when(width > 0., |d| d.w(px(width)).flex_none())
                    .child(format!("{label}{marker}"))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.sftp.panel_mut(side).sort_by(key);
                        cx.notify();
                    }))
            }));

        let mut rows = div().id("rows-local").flex_1().overflow_y_scroll();
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
            out.push(div().id("crumb-more").px_1().text_color(rgb(theme::TEXT)).child("…"));
        }
        for (i, (label, target)) in all.into_iter().enumerate().skip(skip) {
            out.push(
                div()
                    .id(SharedString::from(format!("crumb-{i}")))
                    .px_1()
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .text_color(rgb(theme::TEXT))
                    .child(svg().path(icon("folder")).size(px(16.)).text_color(rgb(FOLDER)))
                    .child(if label == "/" { "/".to_owned() } else { format!("{label}") })
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.sftp.panel_mut(side).navigate(target.clone());
                        shell.sftp.reload(side);
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
        let target = name.clone();
        div()
            .id(SharedString::from(format!("row-{name}")))
            .h(px(40.))
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
                    .child(div().text_color(rgb(text)).truncate().child(name.clone())),
            )
            .child(cell(175., entry.map(Entry::modified_label).unwrap_or_default()))
            .child(cell(115., entry.map(Entry::size_label).unwrap_or_default()))
            .child(cell(115., entry.map(Entry::kind).unwrap_or_default()))
            .on_click(cx.listener(move |shell, event: &ClickEvent, _, cx| {
                if event.click_count() >= 2 {
                    shell.sftp.open(side, &target);
                } else if target != ".." {
                    shell.sftp.panel_mut(side).selected = Some(target.clone());
                }
                cx.notify();
            }))
    }

    fn on_filter_key(&mut self, side: Side, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let panel = self.sftp.panel_mut(side);
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
