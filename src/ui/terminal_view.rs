//! A terminal tab: draws the emulator grid and forwards keys to the session.

use futures::StreamExt;
use gpui::{
    div, font, prelude::*, px, rgb, App, ClipboardItem, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, SharedString, Window,
};
use termian_native::login::{Answer, LoginEvent, LoginState, Stage};
use termian_native::session::{self, Session, SessionEvent, SshTarget};
use termian_native::terminal::{self, key_bytes, paste_bytes, Modifiers, Terminal};

use super::login_view::{self, LoginActions, LoginTarget};

/// Where an SSH tab connects to; kept so the connection can be retried.
#[derive(Clone)]
pub struct Connection {
    pub label: String,
    pub os: Option<String>,
    pub target: SshTarget,
}

/// Requests from the tab to the window.
pub enum TerminalEvent {
    Close,
}

impl gpui::EventEmitter<TerminalEvent> for TerminalView {}

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
    selecting: bool,
    /// SSH tabs show the connection screen until the shell is ready.
    connection: Option<Connection>,
    login: Option<LoginState>,
    login_focus: FocusHandle,
    /// Bumped on retry; events from the replaced session are ignored.
    generation: u64,
}

impl TerminalView {
    pub fn new(session: Session, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::build(session, None, window, cx)
    }

    pub fn ssh(connection: Connection, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let session = session::ssh(connection.target.clone(), 80, 24);
        Self::build(session, Some(connection), window, cx)
    }

    fn build(mut session: Session, connection: Option<Connection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let login_focus = cx.focus_handle();
        window.focus(if connection.is_some() { &login_focus } else { &focus });
        Self::listen(&mut session, 0, window, cx);
        Self {
            terminal: Terminal::new(80, 24),
            session,
            focus,
            closed: false,
            cell_width: None,
            selecting: false,
            login: connection.as_ref().map(|_| LoginState::default()),
            connection,
            login_focus,
            generation: 0,
        }
    }

    fn listen(session: &mut Session, generation: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mut events) = session.take_events() {
            cx.spawn_in(window, async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update_in(cx, |view, window, cx| {
                        if view.generation != generation {
                            return;
                        }
                        view.handle(event, window);
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
    }

    fn handle(&mut self, event: SessionEvent, window: &mut Window) {
        match event {
            SessionEvent::Login(LoginEvent::Stage(Stage::Ready)) => {
                self.login = None;
                window.focus(&self.focus);
            }
            SessionEvent::Login(event) => {
                let asks_text = matches!(event, LoginEvent::Prompt { .. });
                if let Some(login) = self.login.as_mut() {
                    login.apply(event);
                }
                if asks_text {
                    window.focus(&self.login_focus);
                }
            }
            SessionEvent::Closed(reason) if self.login.is_some() => {
                self.closed = true;
                if let Some(login) = self.login.as_mut() {
                    login.fail(reason.unwrap_or_else(|| "The connection was closed.".into()));
                }
            }
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
            if keystroke.key == "c" {
                if let Some(text) = self.terminal.selection_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            } else if keystroke.key == "v" {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.session.write(&paste_bytes(&text, self.terminal.bracketed_paste()));
                }
            }
            return; // other Cmd shortcuts belong to the app
        }
        let mods = Modifiers { control: mods.control, alt: mods.alt, shift: mods.shift };
        if let Some(bytes) = key_bytes(&keystroke.key, keystroke.key_char.as_deref(), mods, self.terminal.app_cursor()) {
            self.terminal.scroll(i32::MIN / 2); // typing jumps back to the prompt
            self.terminal.clear_selection();
            self.session.write(&bytes);
            cx.stop_propagation();
            cx.notify();
        }
    }

    /// Grid cell under a window position.
    fn cell_at(&self, position: Point<Pixels>) -> (usize, usize) {
        let x = (f32::from(position.x) - PADDING).max(0.);
        let y = (f32::from(position.y) - TOP_INSET - PADDING).max(0.);
        ((y / LINE_HEIGHT) as usize, (x / self.cell_width.unwrap_or(8.)).round() as usize)
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        let (row, column) = self.cell_at(event.position);
        self.terminal.start_selection(row, column, event.click_count == 2);
        if event.click_count == 2 {
            self.terminal.extend_selection(row, column);
        }
        self.selecting = true;
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting && event.pressed_button == Some(MouseButton::Left) {
            let (row, column) = self.cell_at(event.position);
            self.terminal.extend_selection(row, column);
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.selecting = false;
        if self.terminal.selection_text().is_none() {
            self.terminal.clear_selection();
        }
        cx.notify();
    }

    /// Types `text` into the session as if pasted, followed by Enter.
    pub fn run(&mut self, text: &str) {
        if !self.closed {
            self.terminal.scroll(i32::MIN / 2);
            self.session.write(&paste_bytes(text, false));
            self.session.write(b"\r");
        }
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(if self.login.is_some() { &self.login_focus } else { &self.focus });
    }

    fn login_answer(&mut self, answer: Answer, _: &mut Window, cx: &mut Context<Self>) {
        let Some(login) = self.login.as_mut() else { return };
        let cancels = matches!(answer, Answer::Text(None) | Answer::HostKey(termian_native::login::HostKeyDecision::Reject));
        if let Some(answer) = login.answer(answer) {
            self.session.answer(answer);
        }
        if cancels {
            cx.emit(TerminalEvent::Close);
        }
        cx.notify();
    }

    fn login_submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(answer) = self.login.as_mut().and_then(LoginState::submit_text) {
            self.login_answer(answer, window, cx);
        }
    }

    fn login_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(login) = self.login.as_mut() else { return };
        if let Some(answer) = login_view::edit(login, event, paste) {
            if let Some(answer) = answer {
                self.login_answer(answer, window, cx);
            }
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn login_close(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TerminalEvent::Close);
    }

    fn login_retry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(connection) = self.connection.clone() else { return };
        self.generation += 1;
        self.session = session::ssh(connection.target, 80, 24);
        Self::listen(&mut self.session, self.generation, window, cx);
        self.terminal = Terminal::new(80, 24);
        self.closed = false;
        let show_logs = self.login.as_ref().is_some_and(|l| l.show_logs);
        self.login = Some(LoginState { show_logs, ..LoginState::default() });
        window.focus(&self.login_focus);
        cx.notify();
    }

    fn login_toggle_logs(&mut self, cx: &mut Context<Self>) {
        if let Some(login) = self.login.as_mut() {
            login.show_logs = !login.show_logs;
        }
        cx.notify();
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
        if let (Some(login), Some(connection)) = (&self.login, &self.connection) {
            let target = LoginTarget {
                label: &connection.label,
                subtitle: format!("SSH {}:{}", connection.target.host, connection.target.port),
                os: connection.os.as_deref(),
            };
            let actions = LoginActions {
                answer: Self::login_answer,
                close: Self::login_close,
                retry: Self::login_retry,
                toggle_logs: Self::login_toggle_logs,
                key: Self::login_key,
                submit: Self::login_submit,
            };
            return login_view::render(target, login, &self.login_focus, window, &actions, cx).into_any_element();
        }
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
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
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
            .into_any_element()
    }
}
