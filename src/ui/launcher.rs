//! New Tab, as in the legacy client: the "Search hosts or tabs" field on
//! top (it opens the command palette), "Workspace templates" (click
//! selects, double click or Enter opens; rename and delete on hover), then
//! "Recent connections" (click selects, double click or Restore connects,
//! «Create a workspace» saves the selection as a template), or the "Recent
//! sessions" empty state when there is neither.

use gpui::{div, prelude::*, px, rgb, AnyElement, ClickEvent, Context, Div, FontWeight, KeyDownEvent, SharedString, Window};
use termian_native::theme;
use termian_native::vault::HostId;
use termian_native::workspace_template::{self as templates, Target, TemplateTerminal, WorkspaceTemplate};

use super::dialog::{self, Key, Kind};
use super::host_editor::caret;
use super::hosts::host_tile;
use crate::Shell;

/// State of the New Tab screen; one is shared by every New Tab.
#[derive(Default)]
pub struct Launcher {
    query: String,
    selected: Vec<HostId>,
    template: Option<u64>,
    /// A template being renamed and the text typed so far.
    renaming: Option<(u64, String)>,
}

#[cfg(target_os = "macos")]
pub const PALETTE_SHORTCUT: &str = "⌘K";
#[cfg(not(target_os = "macos"))]
pub const PALETTE_SHORTCUT: &str = "Ctrl+K";

impl Shell {
    /// Connects from a New Tab: the session takes the New Tab's place.
    fn launch(&mut self, hosts: Vec<HostId>, window: &mut Window, cx: &mut Context<Self>) {
        let launcher = self.tabs.active().id;
        for host in hosts {
            self.connect_host(host, window, cx);
        }
        if self.tabs.all().iter().any(|t| t.id == launcher) && self.tabs.active().id != launcher {
            self.tabs.close(launcher);
        }
        self.launcher = Launcher::default();
        cx.notify();
    }

    /// Legacy rename: Enter keeps a non-empty name; Escape restores the old
    /// one, and a new template without a name gets the default one.
    fn finish_rename(&mut self, keep: bool) {
        let Some((id, text)) = self.launcher.renaming.take() else { return };
        let Some(mut template) = templates::templates(&self.vault).into_iter().find(|t| t.id == id) else { return };
        let text = text.trim();
        if keep && !text.is_empty() {
            template.label = text.to_owned();
        } else if template.label.is_empty() {
            template.label = templates::DEFAULT_NAME.to_owned();
        } else {
            return;
        }
        templates::save(&mut self.vault, &template);
        self.save_vault();
    }

    /// «Create a workspace»: the selected connections as a template without a
    /// name, which is renamed at once.
    fn create_workspace(&mut self, cx: &mut Context<Self>) {
        let terminals: Vec<TemplateTerminal> = self.launcher.selected.iter().map(|host| TemplateTerminal::new(Target::Host(*host))).collect();
        if terminals.is_empty() {
            return;
        }
        let id = templates::next_id(&self.vault);
        let template = WorkspaceTemplate { id, label: String::new(), view_mode: Default::default(), workspace_terminals: terminals, workspace_layout: None, extra: Default::default() };
        templates::save(&mut self.vault, &template);
        self.save_vault();
        self.launcher.selected.clear();
        self.launcher.template = Some(id);
        self.launcher.renaming = Some((id, String::new()));
        cx.notify();
    }

    fn open_selected_template(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let launcher = self.tabs.active().id;
        self.open_template(id, window, cx);
        if self.tabs.all().iter().any(|t| t.id == launcher) && self.tabs.active().id != launcher {
            self.tabs.close(launcher);
        }
        self.launcher = Launcher::default();
        cx.notify();
    }

    fn on_launcher_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        if let Some((_, text)) = self.launcher.renaming.as_mut() {
            match dialog::edit(Some(text), &event.keystroke, paste) {
                Key::Enter => {
                    if !text.trim().is_empty() {
                        self.finish_rename(true);
                    }
                }
                Key::Escape => self.finish_rename(false),
                Key::Edited => {}
                Key::Ignored => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match dialog::edit(Some(&mut self.launcher.query), &event.keystroke, paste) {
            // Typing goes to the command palette, as the legacy field does.
            Key::Edited => {
                let query = std::mem::take(&mut self.launcher.query);
                self.open_palette(query, window, cx);
            }
            Key::Enter => {
                let hosts = self.launcher.selected.clone();
                if let Some(id) = self.launcher.template {
                    self.open_selected_template(id, window, cx);
                } else if !hosts.is_empty() {
                    self.launch(hosts, window, cx);
                }
            }
            Key::Escape => {
                self.launcher.query.clear();
                self.launcher.selected.clear();
                self.launcher.template = None;
            }
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn launcher_row(&self, id: HostId, selected: bool, cx: &mut Context<Self>) -> Option<gpui::Stateful<Div>> {
        let host = self.vault.host(id)?;
        Some(
            div()
                .id(SharedString::from(format!("recent-{id}")))
                .h(px(48.))
                .px_2()
                .flex()
                .items_center()
                .gap_3()
                .rounded_lg()
                .when(selected, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                .when(!selected, |d| d.hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE))))
                .child(host_tile(host).size(px(32.)).rounded_md())
                .child(div().flex_1().min_w_0().truncate().text_color(rgb(theme::TEXT)).child(host.display_label().to_owned()))
                .child(div().flex_none().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(self.vault.host_location(host)))
                .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
                    if event.click_count() >= 2 {
                        shell.launch(vec![id], window, cx);
                        return;
                    }
                    shell.finish_rename(false);
                    shell.launcher.template = None;
                    let selected = &mut shell.launcher.selected;
                    if event.modifiers().secondary() || event.modifiers().shift {
                        match selected.iter().position(|s| *s == id) {
                            Some(at) => {
                                selected.remove(at);
                            }
                            None => selected.push(id),
                        }
                    } else {
                        *selected = vec![id];
                    }
                    window.focus(&shell.launcher_focus);
                    cx.notify();
                })),
        )
    }

    fn template_row(&self, template: &WorkspaceTemplate, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let id = template.id;
        let selected = self.launcher.template == Some(id);
        let renaming = self.launcher.renaming.as_ref().filter(|(r, _)| *r == id).map(|(_, text)| text.clone());
        let action = |name: &str, icon: &'static str, danger: bool| {
            div()
                .id(SharedString::from(format!("template-{name}-{id}")))
                .size(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .child(gpui::svg().path(super::assets::icon(icon)).size(px(14.)).text_color(rgb(if selected { theme::ACCENT } else { theme::TEXT_MUTED })))
                .hover(move |s| s.text_color(rgb(if danger { theme::ERROR } else { theme::ACCENT })))
        };
        let label = match &renaming {
            Some(text) => div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .text_color(rgb(if text.is_empty() { theme::TEXT_MUTED } else { theme::TEXT }))
                .when(text.is_empty(), |d| d.child(caret()).child("New Workspace"))
                .when(!text.is_empty(), |d| d.child(text.clone()).child(caret())),
            None => div().flex_1().min_w_0().truncate().text_color(rgb(theme::TEXT)).child(template.label.clone()),
        };
        div()
            .id(SharedString::from(format!("template-{id}")))
            .group("template-row")
            .h(px(48.))
            .px_2()
            .flex()
            .items_center()
            .gap_3()
            .rounded_lg()
            .when(selected && renaming.is_none(), |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
            .when(!selected && renaming.is_none(), |d| d.hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE))))
            .child(
                div()
                    .size(px(32.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(rgb(theme::TITLE_BAR))
                    .child(gpui::svg().path(super::assets::icon("workspace")).size(px(16.)).text_color(rgb(theme::ACCENT))),
            )
            .child(label)
            .when(renaming.is_none(), |d| {
                d.child(div().flex_none().text_sm().text_color(rgb(theme::TEXT_MUTED)).group_hover("template-row", |s| s.invisible()).child(template.connections()))
                    .child(
                        div()
                            .absolute()
                            .right(px(8.))
                            .flex()
                            .gap(px(5.))
                            .invisible()
                            .group_hover("template-row", |s| s.visible())
                            .child(action("rename", "edit", false).on_click(cx.listener(move |shell, _, window, cx| {
                                cx.stop_propagation();
                                shell.finish_rename(false);
                                let label = templates::templates(&shell.vault).into_iter().find(|t| t.id == id).map(|t| t.label).unwrap_or_default();
                                shell.launcher.template = Some(id);
                                shell.launcher.renaming = Some((id, label));
                                window.focus(&shell.launcher_focus);
                                cx.notify();
                            })))
                            .child(action("delete", "close", true).on_click(cx.listener(move |shell, _, _, cx| {
                                cx.stop_propagation();
                                templates::remove(&mut shell.vault, id);
                                shell.save_vault();
                                if shell.launcher.template == Some(id) {
                                    shell.launcher.template = None;
                                }
                                cx.notify();
                            }))),
                    )
            })
            .relative()
            .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
                if shell.launcher.renaming.as_ref().is_some_and(|(r, _)| *r == id) {
                    return;
                }
                shell.finish_rename(false);
                if event.click_count() >= 2 {
                    shell.open_selected_template(id, window, cx);
                    return;
                }
                shell.launcher.selected.clear();
                shell.launcher.template = Some(id);
                window.focus(&shell.launcher_focus);
                cx.notify();
            }))
    }

    pub fn render_launcher(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let focused = self.launcher_focus.is_focused(window);
        let query = &self.launcher.query;
        let search = div()
            .id("launcher-search")
            .w_full()
            .h(px(44.))
            .px_3()
            .flex()
            .items_center()
            .rounded(px(10.))
            .bg(rgb(theme::TITLE_BAR))
            .border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { theme::DIALOG_ROW }))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .text_color(rgb(if query.is_empty() { theme::TEXT_MUTED } else { theme::ACCENT }))
                    .when(focused && query.is_empty(), |d| d.child(caret()))
                    .child(if query.is_empty() { "Search hosts or tabs".to_owned() } else { query.clone() })
                    .when(focused && !query.is_empty(), |d| d.child(caret())),
            )
            .child(div().mx(px(5.)).text_color(rgb(theme::TEXT_MUTED)).child(PALETTE_SHORTCUT))
            .on_click(cx.listener(|shell, _, window, cx| shell.open_palette(String::new(), window, cx)));

        let section = |title: &str, actions: Vec<AnyElement>| {
            div()
                .w_full()
                .p(px(15.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .rounded(px(20.))
                .bg(rgb(theme::SURFACE))
                .child(div().flex().items_center().gap(px(10.)).child(div().flex_1().font_weight(FontWeight::BOLD).text_color(rgb(theme::TEXT)).child(title.to_owned())).children(actions))
        };

        let saved = templates::templates(&self.vault);
        let body = {
            let recent: Vec<HostId> = self.vault.recent_hosts().iter().map(|h| h.id).collect();
            if recent.is_empty() && saved.is_empty() {
                div()
                    .w_full()
                    .pt(px(55.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("Recent sessions"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(format!("Your recent sessions will be visible here. In the meantime, start connecting with {PALETTE_SHORTCUT}")),
                    )
            } else {
                let any = !self.launcher.selected.is_empty();
                let restore = dialog::button("recent-restore", "Restore", Kind::Accent, !any)
                    .h(px(30.))
                    .px_3()
                    .when(any, |d| {
                        d.on_click(cx.listener(|shell, _, window, cx| {
                            let hosts = shell.launcher.selected.clone();
                            shell.launch(hosts, window, cx);
                        }))
                    })
                    .into_any_element();
                let create = dialog::button("recent-create-workspace", "Create a workspace", Kind::Regular, !any)
                    .h(px(30.))
                    .px_3()
                    .when(any, |d| d.on_click(cx.listener(|shell, _, _, cx| shell.create_workspace(cx))))
                    .into_any_element();
                let workspaces = (!saved.is_empty()).then(|| section("Workspace templates", Vec::new()).children(saved.iter().map(|t| self.template_row(t, cx))));
                let recent = (!recent.is_empty()).then(|| {
                    section("Recent connections", vec![create, restore])
                        .children(recent.into_iter().filter_map(|id| self.launcher_row(id, self.launcher.selected.contains(&id), cx)))
                });
                div().w_full().flex().flex_col().gap(px(20.)).children(workspaces).children(recent)
            }
        };

        div()
            .id("launcher")
            .track_focus(&self.launcher_focus)
            .on_key_down(cx.listener(Self::on_launcher_key))
            .size_full()
            .overflow_y_scroll()
            .px(px(20.))
            .py(px(40.))
            .child(div().max_w(px(720.)).w_full().mx_auto().flex().flex_col().gap(px(20.)).child(search).child(body))
            .into_any_element()
    }
}
