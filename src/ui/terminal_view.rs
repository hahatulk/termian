//! A terminal tab: draws the emulator grid and forwards keys to the session.

use futures::StreamExt;
use gpui::{
    div, font, prelude::*, px, rgb, App, Context, FocusHandle, Focusable, KeyDownEvent, Pixels, ScrollDelta,
    ScrollWheelEvent, SharedString, Window,
};
use termian_native::session::{Session, SessionEvent};
use termian_native::terminal::{self, key_bytes, paste_bytes, Modifiers, Terminal};

const FONT_FAMILY: &str = "Menlo";
const FONT_SIZE: f32 = 13.;
const LINE_HEIGHT: f32 = 18.;
const PADDING: f32 = 10.;

pub struct TerminalView {
    terminal: Terminal,
    session: Session,
    focus: FocusHandle,
    closed: bool,
    cell_width: Option<f32>,
}

impl TerminalView {
    pub fn new(mut session: Session, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        if let Some(mut events) = session.take_events() {
            cx.spawn(async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update(cx, |view, cx| {
                        view.handle(event);
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self { terminal: Terminal::new(80, 24), session, focus, closed: false, cell_width: None }
    }

    fn handle(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Output(bytes) => {
                self.terminal.feed(&bytes);
                let replies = self.terminal.take_replies();
                if !replies.is_empty() {
                    self.session.write(&replies);
                }
            }
            SessionEvent::Closed(_) => {
                self.closed = true;
                self.terminal.feed(b"\r\n\x1b[2m[Session closed]\x1b[0m\r\n");
            }
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.closed {
            return;
        }
        let keystroke = &event.keystroke;
        let mods = keystroke.modifiers;
        if mods.platform {
            if keystroke.key == "v" {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.session.write(&paste_bytes(&text, self.terminal.bracketed_paste()));
                }
            }
            return; // other Cmd shortcuts belong to the app
        }
        let mods = Modifiers { control: mods.control, alt: mods.alt, shift: mods.shift };
        if let Some(bytes) = key_bytes(&keystroke.key, keystroke.key_char.as_deref(), mods, self.terminal.app_cursor()) {
            self.terminal.scroll(i32::MIN / 2); // typing jumps back to the prompt
            self.session.write(&bytes);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let lines = match event.delta {
            ScrollDelta::Lines(delta) => delta.y,
            ScrollDelta::Pixels(delta) => f32::from(delta.y) / LINE_HEIGHT,
        };
        self.terminal.scroll(lines.round() as i32);
        cx.notify();
    }

    /// Fits the grid to the space under the tab bar.
    fn fit(&mut self, area_width: Pixels, area_height: Pixels, window: &Window) {
        let cell_width = *self.cell_width.get_or_insert_with(|| {
            let text = window.text_system();
            let id = text.resolve_font(&font(FONT_FAMILY));
            text.advance(id, px(FONT_SIZE), 'm').map(|size| f32::from(size.width)).unwrap_or(8.)
        });
        let columns = ((f32::from(area_width) - 2. * PADDING) / cell_width).floor().max(2.) as usize;
        let rows = ((f32::from(area_height) - 2. * PADDING) / LINE_HEIGHT).floor().max(1.) as usize;
        if self.terminal.size() != (columns, rows) {
            self.terminal.resize(columns, rows);
            self.session.resize(columns as u16, rows as u16);
        }
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// Height of the tab bar above the terminal.
pub const TOP_INSET: f32 = 44.;

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        self.fit(viewport.width, viewport.height - px(TOP_INSET), window);
        let snapshot = self.terminal.snapshot();
        let focused = self.focus.is_focused(window);
        let cell_width = self.cell_width.unwrap_or(8.);

        let lines = snapshot.lines.iter().map(|runs| {
            let mut line = div().h(px(LINE_HEIGHT)).flex().whitespace_nowrap();
            for run in runs {
                line = line.child(
                    div()
                        .text_color(rgb(run.fg))
                        .when(run.bg != terminal::BACKGROUND, |d| d.bg(rgb(run.bg)))
                        .when(run.bold, |d| d.font_weight(gpui::FontWeight::BOLD))
                        .child(SharedString::from(run.text.clone())),
                );
            }
            line
        });

        let cursor = snapshot.cursor.filter(|_| !self.closed).map(|(row, column)| {
            div()
                .absolute()
                .left(px(PADDING + column as f32 * cell_width))
                .top(px(PADDING + row as f32 * LINE_HEIGHT))
                .w(px(cell_width))
                .h(px(LINE_HEIGHT))
                .when(focused, |d| d.bg(rgb(terminal::CURSOR)).opacity(0.7))
                .when(!focused, |d| d.border_1().border_color(rgb(terminal::CURSOR)))
        });

        div()
            .id("terminal")
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::on_key))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .on_click(cx.listener(|view, _, window, _| window.focus(&view.focus)))
            .relative()
            .size_full()
            .p(px(PADDING))
            .overflow_hidden()
            .bg(rgb(terminal::BACKGROUND))
            .font_family(FONT_FAMILY)
            .text_size(px(FONT_SIZE))
            .line_height(px(LINE_HEIGHT))
            .children(lines)
            .children(cursor)
    }
}
