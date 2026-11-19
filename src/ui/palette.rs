//! Command palette (⌘K): one search field over the window with Hosts, Tabs
//! and Quick connect results; arrows choose, Enter runs, Escape closes.

use gpui::{div, prelude::*, px, rgb, rgba, svg, AnyElement, Context, FontWeight, KeyDownEvent, SharedString, Window};
use termian_native::palette::{flatten, sections, Item};
use termian_native::session::parse_destination;
use termian_native::tabs::TabKind;
use termian_native::theme;

use super::assets::icon;
use super::dialog::Key;
use super::hosts::os_tile;
use super::text_input::{self, Mode};
use crate::Shell;

gpui::actions!(termian, [OpenPalette]);

#[derive(Default)]
pub struct Palette {
    query: String,
    /// Index into the flattened results.
    selected: usize,
}

impl Shell {
    pub fn open_palette(&mut self, query: String, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        self.palette = Some(Palette { query, selected: 0 });
        window.focus(&self.palette_focus);
        cx.notify();
    }

    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = None;
        self.focus_active_tab(window, cx);
        cx.notify();
    }

    /// Gives the keyboard back to the terminal or New Tab under the palette.
    fn focus_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.tabs.active();
        match &active.kind {
            TabKind::Session { .. } => {
                if let Some(view) = self.focused_view(active.id) {
                    view.read(cx).focus(window);
                }
            }
            TabKind::Launcher => window.focus(&self.launcher_focus),
            _ => window.focus(&self.root_focus),
        }
    }

    fn run_palette_item(&mut self, item: Item, window: &mut Window, cx: &mut Context<Self>) {
        self.palette = None;
        // A connection started from a New Tab takes its place, as in the legacy client.
        let launcher = matches!(self.tabs.active().kind, TabKind::Launcher).then(|| self.tabs.active().id);
        match item {
            Item::Host { id, .. } => self.connect_host(id, window, cx),
            Item::QuickConnect(text) => {
                if let Some(target) = parse_destination(&text) {
                    let label = text.trim().trim_start_matches("ssh ").to_owned();
                    self.open_ssh(label, None, target, window, cx);
                }
            }
            Item::LocalTerminal => self.open_local_terminal(window, cx),
            Item::Tab { id, .. } => {
                self.tabs.activate(id);
                self.focus_active_tab(window, cx);
                cx.notify();
                return;
            }
        }
        if let Some(launcher) = launcher.filter(|id| *id != self.tabs.active().id) {
            self.tabs.close(launcher);
        }
        cx.notify();
    }

    fn on_palette_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(palette) = &mut self.palette else { return };
        let count = flatten(&sections(&self.vault, self.tabs.all(), &palette.query)).len();
        match event.keystroke.key.as_str() {
            "down" => palette.selected = (palette.selected + 1).min(count.saturating_sub(1)),
            "up" => palette.selected = palette.selected.saturating_sub(1),
            _ => match text_input::edit("palette", Some(&mut palette.query), &event.keystroke, Mode::Line, cx) {
                Key::Edited => palette.selected = 0,
                Key::Enter => {
                    let found = sections(&self.vault, self.tabs.all(), &palette.query);
                    if let Some(item) = flatten(&found).get(palette.selected).map(|i| (*i).clone()) {
                        self.run_palette_item(item, window, cx);
                    }
                }
                Key::Escape => self.close_palette(window, cx),
                Key::Ignored => return,
            },
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub fn render_palette(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let palette = self.palette.as_ref()?;
        let focused = self.palette_focus.is_focused(window);
        let found = sections(&self.vault, self.tabs.all(), &palette.query);
        let field = div().id("palette-input")
            .h(px(36.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(10.))
            .rounded(px(10.))
            .bg(rgb(theme::INPUT))
            .child(svg().path(icon("search")).size(px(14.)).text_color(rgb(theme::TEXT)))
            .child(div().flex_1().child(text_input::text("palette", &palette.query, "Search hosts or tabs", focused, Mode::Line, cx)))
            .child(div().text_color(rgb(theme::TEXT_MUTED)).child(super::launcher::PALETTE_SHORTCUT));

        let field = text_input::field(field, "palette", Mode::Line, |shell, window, _| window.focus(&shell.palette_focus), cx);

        let mut index = 0;
        let mut list = div().id("palette-results").max_h(px(420.)).overflow_y_scroll().flex().flex_col();
        for (n, section) in found.iter().enumerate() {
            if n > 0 {
                list = list.child(div().h(px(1.)).w_full().my(px(10.)).bg(rgb(theme::SURFACE_ACTIVE)));
            }
            list = list.child(div().px(px(10.)).pb(px(5.)).text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT_MUTED)).child(section.title));
            for item in &section.items {
                let selected = index == palette.selected;
                let at = index;
                let tile = match item {
                    Item::Host { os, .. } => os_tile(os.as_deref()),
                    Item::Tab { .. } => super::hosts::tile("terminal", theme::TILE_BLUE),
                    Item::QuickConnect(_) => super::hosts::tile("connect", theme::TILE_BLUE),
                    Item::LocalTerminal => super::hosts::tile("local", theme::TILE_BLUE),
                };
                let run = item.clone();
                list = list.child(
                    div()
                        .id(SharedString::from(format!("palette-{at}")))
                        .h(px(40.))
                        .flex_none()
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .gap_3()
                        .rounded_md()
                        .when(selected, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                        .child(tile.size(px(24.)).rounded_md())
                        .child(div().flex_1().truncate().text_color(rgb(theme::TEXT)).child(item.title().to_owned()))
                        .on_mouse_move(cx.listener(move |shell, _, _, cx| {
                            if let Some(palette) = &mut shell.palette {
                                if palette.selected != at {
                                    palette.selected = at;
                                    cx.notify();
                                }
                            }
                        }))
                        .on_click(cx.listener(move |shell, _, window, cx| shell.run_palette_item(run.clone(), window, cx))),
                );
                index += 1;
            }
        }

        Some(
            div()
                .id("palette-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(80.))
                .bg(rgba(0x00000066))
                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|shell, _, window, cx| shell.close_palette(window, cx)))
                .child(
                    div()
                        .id("palette")
                        .track_focus(&self.palette_focus)
                        .on_key_down(cx.listener(Self::on_palette_key))
                        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .w(px(640.))
                        .max_w_full()
                        .h_auto()
                        .p(px(10.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .rounded_lg()
                        .bg(rgb(theme::BACKGROUND))
                        .border_1()
                        .border_color(rgb(theme::SURFACE_ACTIVE))
                        .shadow_lg()
                        .child(field)
                        .child(list),
                )
                .into_any_element(),
        )
    }
}
