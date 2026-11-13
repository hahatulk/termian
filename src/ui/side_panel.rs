//! The terminal side panel («Show Terminal Side-panel»), as in legacy:
//! a Snippets tab (search, packages that expand, click shows the whole
//! script, double click runs; right click: Run, Edit, Run in all tabs, Paste,
//! Copy, View more) and a History tab (Shell History with PASTE and SAVE).

use std::collections::HashSet;

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, ClickEvent, ClipboardItem, Context, Div, FocusHandle, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Pixels, Point, SharedString, Window};
use termian_native::theme;
use termian_native::vault::{Snippet, ShellCommand};

use super::assets::icon;
use super::dialog::{self, Key};
use super::host_editor::{caret, menu_panel, menu_row};
use super::hosts::Section;
use crate::Shell;

const WIDTH: f32 = 340.;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Snippets,
    History,
}

#[derive(Default)]
pub struct SidePanel {
    tab: Tab,
    search: String,
    selected: Option<u64>,
    expanded: HashSet<u64>,
    menu: Option<(u64, Point<Pixels>)>,
    /// History row being saved as a snippet, with the label typed so far.
    saving: Option<(usize, String)>,
    saved: Vec<usize>,
}

/// Snippets and packages matching the search, as legacy filters them:
/// a package is shown when its name or one of its snippets matches, and
/// expands by itself while searching.
fn matches(snippet: &Snippet, needle: &str) -> bool {
    needle.is_empty() || snippet.display_label().to_lowercase().contains(needle) || snippet.script.to_lowercase().contains(needle)
}

impl Shell {
    pub fn toggle_side_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.side_panel = match self.side_panel.take() {
            Some(_) => None,
            None => Some(SidePanel::default()),
        };
        if self.side_panel.is_none() {
            if let Some(view) = self.terminals.get(&self.tabs.active().id) {
                view.read(cx).focus(window);
            }
        }
        cx.notify();
    }

    /// The title bar button, shown on terminal tabs.
    pub fn render_side_panel_button(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("side-panel-toggle")
            .size(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
            .child(svg().path(icon("side_panel")).w(px(18.)).h(px(14.)).text_color(rgb(if self.side_panel.is_some() { theme::TEXT } else { theme::TEXT_MUTED })))
            .on_click(cx.listener(|shell, _, window, cx| shell.toggle_side_panel(window, cx)))
            .into_any_element()
    }

    fn active_terminal(&self) -> Option<gpui::Entity<super::terminal_view::TerminalView>> {
        self.terminals.get(&self.tabs.active().id).cloned()
    }

    /// Run types the script and Enter; Paste only types it.
    fn panel_send(&mut self, script: &str, run: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(view) = self.active_terminal() {
            view.update(cx, |view, _| if run { view.run(script) } else { view.paste(script) });
            view.read(cx).focus(window);
        }
        if let Some(panel) = &mut self.side_panel {
            panel.menu = None;
        }
        cx.notify();
    }

    fn run_in_all_tabs(&mut self, script: &str, cx: &mut Context<Self>) {
        for view in self.terminals.values() {
            view.update(cx, |view, _| view.run(script));
        }
        if let Some(panel) = &mut self.side_panel {
            panel.menu = None;
        }
        cx.notify();
    }

    /// Edit and New Snippet use the Snippets screen's form.
    fn panel_edit_snippet(&mut self, id: Option<u64>, window: &mut Window, cx: &mut Context<Self>) {
        let vaults = self.tabs.all()[0].id;
        self.tabs.activate(vaults);
        self.section = Section::Snippets;
        match id {
            Some(id) => self.edit_snippet(id, window, cx),
            None => self.new_snippet(window, cx),
        }
    }

    fn on_side_panel_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(panel) = &mut self.side_panel else { return };
        let target = match &mut panel.saving {
            Some((_, label)) => label,
            None => &mut panel.search,
        };
        match dialog::edit(Some(target), &event.keystroke, paste) {
            Key::Enter => {
                if let Some((row, label)) = panel.saving.take() {
                    self.save_panel_history(row, label);
                }
            }
            Key::Escape => {
                if panel.saving.take().is_none() {
                    panel.search.clear();
                }
            }
            Key::Edited => {}
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn save_panel_history(&mut self, row: usize, label: String) {
        let Some(command) = self.vault.shell_history.get(row) else { return };
        let snippet = Snippet { id: self.vault.next_id(), label, script: command.script.clone(), ..Snippet::default() };
        self.vault.save_snippet(snippet);
        if let Some(panel) = &mut self.side_panel {
            panel.saved.push(row);
        }
        self.save_vault();
    }

    pub fn render_side_panel(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = self.side_panel.as_ref()?;
        let tab = |id: &str, name: &str, which: Tab, cx: &mut Context<Self>| {
            let active = panel.tab == which;
            div()
                .id(SharedString::from(id.to_owned()))
                .flex_1()
                .h(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .when(active, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(svg().path(icon(name)).size(px(16.)).text_color(rgb(if active { theme::TEXT } else { theme::TEXT_MUTED })))
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(panel) = &mut shell.side_panel {
                        panel.tab = which;
                        panel.menu = None;
                        panel.saving = None;
                    }
                    cx.notify();
                }))
        };
        let tabs = div().flex().gap_1().p_1().rounded_lg().bg(rgb(theme::SURFACE)).child(tab("panel-tab-snippets", "snippets", Tab::Snippets, cx)).child(tab("panel-tab-history", "logs", Tab::History, cx));
        let body = match panel.tab {
            Tab::Snippets => self.render_panel_snippets(panel, &self.panel_focus, window, cx),
            Tab::History => self.render_panel_history(panel, cx),
        };
        let menu = panel.menu.map(|(id, at)| self.render_panel_menu(id, at, cx));
        Some(
            div()
                .id("side-panel")
                .track_focus(&self.panel_focus)
                .on_key_down(cx.listener(Self::on_side_panel_key))
                .w(px(WIDTH))
                .h_full()
                .flex_none()
                .p(px(10.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .bg(rgb(theme::SIDEBAR))
                .border_l_1()
                .border_color(rgb(theme::TITLE_BAR))
                .text_sm()
                .child(tabs)
                .child(body)
                .children(menu)
                .into_any_element(),
        )
    }

    fn render_panel_snippets(&self, panel: &SidePanel, focus: &FocusHandle, window: &Window, cx: &mut Context<Self>) -> Div {
        let needle = panel.search.trim().to_lowercase();
        let focused = focus.is_focused(window) && panel.saving.is_none();
        let new = div()
            .id("panel-new-snippet")
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(theme::TEXT))
            .child(svg().path(icon("snippets")).size(px(14.)).text_color(rgb(theme::TEXT)))
            .child("New Snippet")
            .on_click(cx.listener(|shell, _, window, cx| shell.panel_edit_snippet(None, window, cx)));
        let empty = panel.search.is_empty();
        let search = div()
            .id("panel-search")
            .flex_1()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded_md()
            .bg(rgb(theme::INPUT))
            .border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
            .child(svg().path(icon("search")).size(px(12.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .text_color(rgb(if empty { theme::TEXT_MUTED } else { theme::TEXT }))
                    .when(focused && empty, |d| d.child(caret()))
                    .child(div().truncate().child(if empty { "Search".to_owned() } else { panel.search.clone() }))
                    .when(focused && !empty, |d| d.child(caret())),
            )
            .on_click(cx.listener(|shell, _, window, cx| {
                window.focus(&shell.panel_focus);
                cx.notify();
            }));
        let header = div().flex().gap_2().when(empty, |d| d.child(new)).child(search);

        let mut list = div().id("panel-snippets").flex_1().overflow_y_scroll().flex().flex_col().gap(px(4.));
        let mut any = false;
        for package in &self.vault.packages {
            let snippets: Vec<&Snippet> = self.vault.snippets.iter().filter(|s| s.package == Some(package.id) && matches(s, &needle)).collect();
            if !needle.is_empty() && snippets.is_empty() && !package.label.to_lowercase().contains(&needle) {
                continue;
            }
            any = true;
            let id = package.id;
            let open = panel.expanded.contains(&id) || (!needle.is_empty() && !snippets.is_empty());
            let count = self.vault.package_snippet_count(id);
            list = list.child(
                div()
                    .id(SharedString::from(format!("panel-package-{id}")))
                    .px_2()
                    .py(px(8.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .hover(|s| s.bg(rgb(theme::SURFACE)))
                    .child(svg().path(icon("snippets_package")).size(px(16.)).text_color(rgb(theme::TEXT)))
                    .child(div().flex_1().min_w_0().flex().flex_col().child(div().truncate().text_color(rgb(theme::TEXT)).child(package.label.clone())).child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(format!("{count} {}", if count == 1 { "snippet" } else { "snippets" }))))
                    .child(
                        svg()
                            .path(icon("chevron"))
                            .size(px(10.))
                            .text_color(rgb(theme::TEXT_MUTED))
                            .when(open, |d| d.with_transformation(gpui::Transformation::rotate(gpui::radians(std::f32::consts::PI)))),
                    )
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(panel) = &mut shell.side_panel {
                            if !panel.expanded.remove(&id) {
                                panel.expanded.insert(id);
                            }
                        }
                        cx.notify();
                    })),
            );
            if open {
                for snippet in snippets {
                    list = list.child(self.render_panel_snippet(panel, snippet, true, cx));
                }
            }
        }
        for snippet in self.vault.snippets.iter().filter(|s| s.package.is_none() && matches(s, &needle)) {
            any = true;
            list = list.child(self.render_panel_snippet(panel, snippet, false, cx));
        }
        let body = if any {
            list.into_any_element()
        } else if needle.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .text_center()
                .child(div().text_color(rgb(theme::TEXT)).child("Create snippets from your commands"))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Store your most used commands to reuse them in one click."))
                .into_any_element()
        } else {
            div().flex_1().pt_5().text_center().text_color(rgb(theme::TEXT_MUTED)).child("No results").into_any_element()
        };
        div().flex_1().min_h_0().flex().flex_col().gap(px(12.)).child(header).child(body)
    }

    fn render_panel_snippet(&self, panel: &SidePanel, snippet: &Snippet, nested: bool, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let id = snippet.id;
        let selected = panel.selected == Some(id);
        div()
            .id(SharedString::from(format!("panel-snippet-{id}")))
            .when(nested, |d| d.ml(px(16.)))
            .px_2()
            .py(px(8.))
            .flex()
            .flex_col()
            .gap_1()
            .rounded_md()
            .when(selected, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
            .hover(|s| s.bg(rgb(theme::SURFACE)))
            .child(div().truncate().text_color(rgb(theme::TEXT)).child(snippet.display_label().to_owned()))
            .child(
                div()
                    .font_family("Menlo")
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .when(!selected, |d| d.truncate())
                    .child(if selected { snippet.script.clone() } else { snippet.script.lines().next().unwrap_or("").to_owned() }),
            )
            .on_click(cx.listener(move |shell, event: &ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    if let Some(script) = shell.vault.snippet(id).map(|s| s.script.clone()) {
                        shell.panel_send(&script, true, window, cx);
                    }
                    return;
                }
                if let Some(panel) = &mut shell.side_panel {
                    panel.selected = if panel.selected == Some(id) { None } else { Some(id) };
                }
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                if let Some(panel) = &mut shell.side_panel {
                    panel.menu = Some((id, event.position));
                }
                cx.notify();
            }))
    }

    fn render_panel_menu(&self, id: u64, at: Point<Pixels>, cx: &mut Context<Self>) -> AnyElement {
        let script = self.vault.snippet(id).map(|s| s.script.clone()).unwrap_or_default();
        let (run, all, paste, copy) = (script.clone(), script.clone(), script.clone(), script);
        let items = menu_panel("panel-snippet-menu")
            .child(menu_row("panel-menu-run".into(), "Run", false, false).on_click(cx.listener(move |shell, _, window, cx| shell.panel_send(&run, true, window, cx))))
            .child(menu_row("panel-menu-edit".into(), "Edit", false, false).on_click(cx.listener(move |shell, _, window, cx| shell.panel_edit_snippet(Some(id), window, cx))))
            .child(menu_row("panel-menu-all".into(), "Run in all tabs", false, false).on_click(cx.listener(move |shell, _, _, cx| shell.run_in_all_tabs(&all, cx))))
            .child(menu_row("panel-menu-paste".into(), "Paste", false, false).on_click(cx.listener(move |shell, _, window, cx| shell.panel_send(&paste, false, window, cx))))
            .child(menu_row("panel-menu-copy".into(), "Copy", false, false).on_click(cx.listener(move |shell, _, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                if let Some(panel) = &mut shell.side_panel {
                    panel.menu = None;
                }
                cx.notify();
            })))
            .child(
                menu_row("panel-menu-more".into(), "View more", false, false)
                    .child(div().flex_1())
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Click"))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(panel) = &mut shell.side_panel {
                            panel.selected = if panel.selected == Some(id) { None } else { Some(id) };
                            panel.menu = None;
                        }
                        cx.notify();
                    })),
            );
        gpui::deferred(
            div()
                .id("panel-menu-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                    if let Some(panel) = &mut shell.side_panel {
                        panel.menu = None;
                    }
                    cx.notify();
                }))
                .child(gpui::anchored().position(at).child(items)),
        )
        .into_any_element()
    }

    fn render_panel_history(&self, panel: &SidePanel, cx: &mut Context<Self>) -> Div {
        let commands: &[ShellCommand] = &self.vault.shell_history;
        if commands.is_empty() {
            return div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .text_center()
                .child(div().text_color(rgb(theme::TEXT)).child("You don't have shell history yet"))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Connect to a host via SSH to see your shell history here."));
        }
        let button = |id: String, text: &str, accent: bool| {
            div()
                .id(SharedString::from(id))
                .h(px(22.))
                .w(px(52.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .bg(rgb(if accent { theme::ACCENT } else { theme::SURFACE_ACTIVE }))
                .text_xs()
                .font_weight(FontWeight::BLACK)
                .text_color(rgb(0xffffff))
                .child(text.to_owned())
        };
        let rows = commands.iter().enumerate().map(|(i, command)| {
            let hover = SharedString::from(format!("panel-history-{i}"));
            let saving = panel.saving.as_ref().filter(|(row, _)| *row == i).map(|(_, l)| l.clone());
            let script = command.script.clone();
            let actions: AnyElement = if saving.is_some() {
                button(format!("panel-history-done-{i}"), "Done", true)
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        let label = shell.side_panel.as_mut().and_then(|p| p.saving.take()).map(|(_, l)| l).unwrap_or_default();
                        shell.save_panel_history(i, label);
                        cx.notify();
                    }))
                    .into_any_element()
            } else if panel.saved.contains(&i) {
                div().text_xs().text_color(rgb(theme::ACCENT)).child("saved").into_any_element()
            } else {
                div()
                    .flex()
                    .gap_1()
                    .invisible()
                    .group_hover(hover.clone(), |s| s.visible())
                    .child(button(format!("panel-history-save-{i}"), "SAVE", true).on_click(cx.listener(move |shell, _, window, cx| {
                        if let Some(panel) = &mut shell.side_panel {
                            panel.saving = Some((i, String::new()));
                        }
                        window.focus(&shell.panel_focus);
                        cx.notify();
                    })))
                    .child(button(format!("panel-history-paste-{i}"), "PASTE", false).on_click(cx.listener(move |shell, _, window, cx| shell.panel_send(&script, false, window, cx))))
                    .into_any_element()
            };
            let title = match saving {
                Some(label) => div().flex().items_center().text_color(rgb(if label.is_empty() { theme::TEXT_MUTED } else { theme::TEXT })).child(if label.is_empty() { "Set a label".to_owned() } else { label }).child(caret()),
                None => div().truncate().font_family("Menlo").text_xs().text_color(rgb(theme::TEXT)).child(command.script.clone()),
            };
            div().id(hover.clone()).group(hover).px_2().py(px(8.)).flex().items_center().gap_2().rounded_md().hover(|s| s.bg(rgb(theme::SURFACE))).child(div().flex_1().min_w_0().child(title)).child(actions)
        });
        div().flex_1().min_h_0().child(div().id("panel-history").size_full().overflow_y_scroll().flex().flex_col().children(rows))
    }
}
