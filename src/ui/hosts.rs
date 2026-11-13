//! Vaults tab: navigation sidebar and the Hosts screen.

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, KeyDownEvent, SharedString, Stateful, Window};
use termian_native::session;
use termian_native::theme;
use termian_native::vault::{Group, GroupId, Host};

use super::assets::icon;
use crate::Shell;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Hosts,
    Keychain,
    PortForwarding,
    Snippets,
    KnownHosts,
    Logs,
}

const SECTIONS: [(Section, &str, &str); 6] = [
    (Section::Hosts, "Hosts", "hosts"),
    (Section::Keychain, "Keychain", "keychain"),
    (Section::PortForwarding, "Port Forwarding", "port_forwarding"),
    (Section::Snippets, "Snippets", "snippets"),
    (Section::KnownHosts, "Known Hosts", "known_hosts"),
    (Section::Logs, "Logs", "logs"),
];

fn glyph(name: &str, size: f32, color: u32) -> impl IntoElement {
    svg().path(icon(name)).size(px(size)).flex_none().text_color(rgb(color))
}

pub(super) fn tile(name: &str, background: u32) -> Div {
    div()
        .size(px(40.))
        .flex_none()
        .rounded_lg()
        .bg(rgb(background))
        .flex()
        .items_center()
        .justify_center()
        .child(glyph(name, 20., 0xffffff))
}

pub(super) fn host_tile(host: &Host) -> Div {
    os_tile(host.os.as_deref())
}

/// Host icon tile for a distribution name; unknown systems get a generic icon.
pub(super) fn os_tile(os: Option<&str>) -> Div {
    match os {
        Some("debian") => tile("debian", theme::TILE_DEBIAN),
        Some("ubuntu") => tile("ubuntu", theme::TILE_UBUNTU),
        Some(_) => tile("linux", theme::TILE_BLUE),
        None => tile("new_host", theme::TILE_BLUE),
    }
}

pub(super) fn card(id: SharedString, tile: Div, title: String, subtitle: String) -> Stateful<Div> {
    div()
        .id(id)
        .h(px(60.))
        .px_3()
        .flex()
        .items_center()
        .gap_3()
        .rounded_lg()
        .bg(rgb(theme::SURFACE))
        .border_2()
        .border_color(rgb(theme::SURFACE))
        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)).border_color(rgb(theme::SURFACE_ACTIVE)))
        .child(tile)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(div().text_color(rgb(theme::TEXT)).truncate().child(title))
                .when(!subtitle.is_empty(), |d| {
                    d.child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).truncate().child(subtitle))
                }),
        )
}

fn caret() -> Div {
    div().mx(px(1.)).w(px(1.5)).h(px(16.)).bg(rgb(theme::TEXT))
}

pub(super) fn section_title(title: &str) -> Div {
    div().pt_5().pb_3().text_color(rgb(theme::TEXT)).font_weight(gpui::FontWeight::SEMIBOLD).child(title.to_owned())
}

pub(super) fn tool_button(id: &str, icon_name: &str, label: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .gap_1()
        .rounded_md()
        .bg(rgb(theme::INPUT))
        .text_xs()
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(rgb(theme::TEXT))
        .child(glyph(icon_name, 14., theme::TEXT))
        .child(label.to_owned())
}

impl Shell {
    pub fn render_vaults(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let content = match self.section {
            Section::Hosts => self.render_hosts(window, cx).into_any_element(),
            Section::Keychain => self.render_keychain(cx).into_any_element(),
            Section::PortForwarding => self.render_port_forwarding().into_any_element(),
            Section::Snippets => self.render_snippets(cx).into_any_element(),
            Section::KnownHosts => self.render_known_hosts().into_any_element(),
            Section::Logs => self.render_logs().into_any_element(),
        };
        let editor = match self.section {
            Section::Hosts => self.render_host_editor(window, cx),
            Section::Keychain => self.render_key_editor(window, cx),
            _ => None,
        };
        div().size_full().relative().flex().child(self.render_sidebar(cx)).child(content).children(editor).into_any_element()
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Div {
        div().w(px(185.)).h_full().flex_none().p_2().flex().flex_col().gap_1().bg(rgb(theme::SIDEBAR)).children(
            SECTIONS.map(|(section, label, icon_name)| {
                let active = section == self.section;
                div()
                    .id(SharedString::from(format!("section-{label}")))
                    .h(px(36.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .rounded_md()
                    .text_color(rgb(theme::TEXT))
                    .when(active, |s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .hover(|s| s.bg(rgb(theme::SURFACE)))
                    .child(glyph(icon_name, 16., theme::TEXT))
                    .child(label)
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.section = section;
                        cx.notify();
                    }))
            }),
        )
    }

    /// Typing in the search field. Enter opens the first match, or connects
    /// to the typed `user@host` when nothing matches; Escape clears.
    fn on_search_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            if keystroke.modifiers.platform && keystroke.key == "v" {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.query.push_str(text.lines().next().unwrap_or(""));
                }
            }
        } else {
            match keystroke.key.as_str() {
                "backspace" => {
                    self.query.pop();
                }
                "escape" => self.query.clear(),
                "enter" => self.connect_query(window, cx),
                _ => match keystroke.key_char.as_deref() {
                    Some(text) if !text.chars().any(char::is_control) => self.query.push_str(text),
                    _ => return,
                },
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn connect_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.query.trim().is_empty() {
            return;
        }
        if let Some(host) = self.vault.search(&self.query).first().map(|h| h.id) {
            self.query.clear();
            self.connect_host(host, window, cx);
        } else if let Some(target) = session::parse_destination(&self.query) {
            let title = self.query.trim().trim_start_matches("ssh ").to_owned();
            self.query.clear();
            self.open_ssh(title, None, target, window, cx);
        }
    }

    fn render_hosts(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let focused = self.search_focus.is_focused(window);
        let (text, color) = if self.query.is_empty() {
            ("Find a host or ssh user@hostname...".to_owned(), theme::TEXT_MUTED)
        } else {
            (self.query.clone(), theme::TEXT)
        };
        let search = div()
            .id("search")
            .track_focus(&self.search_focus)
            .on_key_down(cx.listener(Self::on_search_key))
            .on_click(cx.listener(|shell, _, window, cx| {
                window.focus(&shell.search_focus);
                cx.notify();
            }))
            .m_2()
            .h(px(36.))
            .px_3()
            .flex()
            .items_center()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .text_color(rgb(color))
                    .when(focused && self.query.is_empty(), |d| d.child(caret()))
                    .child(text)
                    .when(focused && !self.query.is_empty(), |d| d.child(caret())),
            )
            .child(
                div()
                    .id("connect")
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(rgb(theme::SURFACE))
                    .text_xs()
                    .text_color(rgb(if self.query.is_empty() { theme::TEXT_MUTED } else { theme::TEXT }))
                    .child("CONNECT")
                    .on_click(cx.listener(|shell, _, window, cx| shell.connect_query(window, cx))),
            );

        let toolbar = div()
            .px_3()
            .pb_2()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(tool_button("new-host", "new_host", "NEW HOST").on_click(cx.listener(|shell, _, window, cx| shell.new_host(window, cx))))
            .child(tool_button("terminal", "terminal", "TERMINAL").on_click(cx.listener(|shell, _, window, cx| {
                shell.open_local_terminal(window, cx);
            })))
            .child(tool_button("serial", "serial", "SERIAL"))
            .child(div().flex_1())
            .child(glyph("grid", 16., theme::TEXT))
            .child(glyph("tag", 16., theme::TEXT));

        let mut body = div().id("hosts-scroll").flex_1().overflow_y_scroll().px_5().pb_5();

        if let Some(error) = &self.vault_error {
            body = body.child(div().pt_5().text_color(rgb(theme::TILE_UBUNTU)).child(format!("Vault: {error}")));
        }

        if !self.query.trim().is_empty() {
            let results = self.vault.search(&self.query);
            body = body.child(section_title(if results.is_empty() { "No hosts found" } else { "Hosts" }));
            return div().flex_1().h_full().flex().flex_col().child(search).child(toolbar).child(
                body.child(div().grid().grid_cols(4).gap_3().children(results.into_iter().map(|h| self.render_host(h, cx)))),
            );
        }

        let path = self.vault.breadcrumbs(self.group);
        if !path.is_empty() {
            body = body.child(self.render_breadcrumbs(&path, cx));
        }

        let groups = self.vault.child_groups(self.group);
        if !groups.is_empty() {
            body = body
                .child(section_title("Groups"))
                .child(div().grid().grid_cols(4).gap_3().children(groups.into_iter().map(|g| self.render_group(g, cx))));
        }

        let hosts = self.vault.hosts_under(self.group);
        if !hosts.is_empty() {
            body = body.child(section_title("Hosts")).child(div().grid().grid_cols(4).gap_3().children(
                hosts.into_iter().map(|h| self.render_host(h, cx)),
            ));
        }

        div().flex_1().h_full().flex().flex_col().child(search).child(toolbar).child(body)
    }

    fn render_host(&self, host: &Host, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = host.id;
        let group = SharedString::from(format!("host-card-{id}"));
        // The pencil shows on hover, as the legacy entity cell's edit button.
        let edit = div()
            .id(SharedString::from(format!("host-edit-{id}")))
            .size(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .invisible()
            .group_hover(group.clone(), |s| s.visible())
            .hover(|s| s.bg(rgb(theme::INPUT)))
            .child(svg().path(icon("edit")).size(px(12.)).text_color(rgb(theme::TEXT)))
            .on_click(cx.listener(move |shell, _, window, cx| {
                cx.stop_propagation();
                shell.edit_host(id, window, cx);
            }));
        card(format!("host-{id}").into(), host_tile(host), host.display_label().to_owned(), host.subtitle())
            .group(group)
            .child(edit)
            .on_click(cx.listener(move |shell, event: &gpui::ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    shell.connect_host(id, window, cx);
                }
            }))
            .on_mouse_down(gpui::MouseButton::Right, cx.listener(move |shell, event: &gpui::MouseDownEvent, _, cx| {
                shell.open_host_menu(id, event, cx);
            }))
    }

    fn render_group(&self, group: &Group, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = group.id;
        let count = self.vault.hosts_under(Some(id)).len();
        let noun = if count == 1 { "Host" } else { "Hosts" };
        card(format!("group-{id}").into(), tile("group", theme::TILE_BLUE), group.label.clone(), format!("{count} {noun}"))
            .on_click(cx.listener(move |shell, _, _, cx| {
                shell.group = Some(id);
                cx.notify();
            }))
    }

    fn render_breadcrumbs(&self, path: &[&Group], cx: &mut Context<Self>) -> Div {
        let crumb = |id: &str, label: String, target: Option<GroupId>, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(id.to_owned()))
                .text_color(rgb(theme::ACCENT))
                .child(label)
                .on_click(cx.listener(move |shell, _, _, cx| {
                    shell.group = target;
                    cx.notify();
                }))
        };
        let mut row = div().pt_5().flex().items_center().gap_2().text_sm().child(crumb("crumb-root", "All hosts".into(), None, cx));
        for (i, group) in path.iter().enumerate() {
            row = row.child(div().text_color(rgb(theme::TEXT_MUTED)).child("›"));
            row = if i + 1 == path.len() {
                row.child(div().text_color(rgb(theme::TEXT_MUTED)).child(group.label.clone()))
            } else {
                row.child(crumb(&format!("crumb-{}", group.id), group.label.clone(), Some(group.id), cx))
            };
        }
        row
    }
}
