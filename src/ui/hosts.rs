//! Vaults tab: navigation sidebar and the Hosts screen.

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, SharedString, Stateful};
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

fn tile(name: &str, background: u32) -> Div {
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

fn host_tile(host: &Host) -> Div {
    match host.os.as_deref() {
        Some("debian") => tile("debian", theme::TILE_DEBIAN),
        Some("ubuntu") => tile("ubuntu", theme::TILE_UBUNTU),
        Some(_) => tile("linux", theme::TILE_BLUE),
        None => tile("new_host", theme::TILE_BLUE),
    }
}

fn card(id: SharedString, tile: Div, title: String, subtitle: String) -> Stateful<Div> {
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
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(div().text_color(rgb(theme::TEXT)).truncate().child(title))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).truncate().child(subtitle)),
        )
}

fn section_title(title: &str) -> Div {
    div().pt_5().pb_3().text_color(rgb(theme::TEXT)).font_weight(gpui::FontWeight::SEMIBOLD).child(title.to_owned())
}

fn tool_button(id: &str, icon_name: &str, label: &str) -> Stateful<Div> {
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
    pub fn render_vaults(&self, cx: &mut Context<Self>) -> AnyElement {
        let content = match self.section {
            Section::Hosts => self.render_hosts(cx).into_any_element(),
            _ => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(theme::TEXT_MUTED))
                .child("This screen is not ported to the native client yet.")
                .into_any_element(),
        };
        div().size_full().flex().child(self.render_sidebar(cx)).child(content).into_any_element()
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

    fn render_hosts(&self, cx: &mut Context<Self>) -> Div {
        let search = div()
            .m_2()
            .h(px(36.))
            .px_3()
            .flex()
            .items_center()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .text_color(rgb(theme::TEXT_MUTED))
            .child(div().flex_1().child("Find a host or ssh user@hostname..."))
            .child(div().px_2().py_1().rounded_sm().bg(rgb(theme::SURFACE)).text_xs().child("CONNECT"));

        let toolbar = div()
            .px_3()
            .pb_2()
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(tool_button("new-host", "new_host", "NEW HOST"))
            .child(tool_button("terminal", "terminal", "TERMINAL"))
            .child(tool_button("serial", "serial", "SERIAL"))
            .child(div().flex_1())
            .child(glyph("grid", 16., theme::TEXT))
            .child(glyph("tag", 16., theme::TEXT));

        let mut body = div().id("hosts-scroll").flex_1().overflow_y_scroll().px_5().pb_5();

        if let Some(error) = &self.vault_error {
            body = body.child(div().pt_5().text_color(rgb(theme::TILE_UBUNTU)).child(format!("Could not read the legacy profile: {error}")));
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
                hosts.into_iter().map(|h| card(format!("host-{}", h.id).into(), host_tile(h), h.label.clone(), h.subtitle())),
            ));
        }

        div().flex_1().h_full().flex().flex_col().child(search).child(toolbar).child(body)
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
