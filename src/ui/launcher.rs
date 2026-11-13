//! New Tab, as in the legacy client: the "Search hosts or tabs" field on
//! top (it opens the command palette), then "Recent connections" (click
//! selects, double click or Restore connects), or the "Recent sessions"
//! empty state when there is no history.

use gpui::{div, prelude::*, px, rgb, AnyElement, ClickEvent, Context, Div, FontWeight, KeyDownEvent, SharedString, Window};
use termian_native::theme;
use termian_native::vault::HostId;

use super::dialog::{self, Key, Kind};
use super::host_editor::caret;
use super::hosts::host_tile;
use crate::Shell;

/// State of the New Tab screen; one is shared by every New Tab.
#[derive(Default)]
pub struct Launcher {
    query: String,
    selected: Vec<HostId>,
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

    fn on_launcher_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        match dialog::edit(Some(&mut self.launcher.query), &event.keystroke, paste) {
            // Typing goes to the command palette, as the legacy field does.
            Key::Edited => {
                let query = std::mem::take(&mut self.launcher.query);
                self.open_palette(query, window, cx);
            }
            Key::Enter => {
                let hosts = self.launcher.selected.clone();
                if !hosts.is_empty() {
                    self.launch(hosts, window, cx);
                }
            }
            Key::Escape => {
                self.launcher.query.clear();
                self.launcher.selected.clear();
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

        let body = {
            let recent: Vec<HostId> = self.vault.recent_hosts().iter().map(|h| h.id).collect();
            if recent.is_empty() {
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
                section("Recent connections", vec![restore])
                    .children(recent.into_iter().filter_map(|id| self.launcher_row(id, self.launcher.selected.contains(&id), cx)))
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
