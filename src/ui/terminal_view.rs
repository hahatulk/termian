//! A terminal tab: draws the emulator grid and forwards keys to the session.

use futures::StreamExt;
use gpui::{
    div, font, prelude::*, px, rgb, App, ClipboardItem, Context, FocusHandle, Focusable, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, SharedString, Window,
};
use termian_native::login::{Answer, LoginEvent, LoginState, Stage};
use termian_native::session::{self, Session, SessionEvent, SshTarget};
use termian_native::terminal::{key_bytes, paste_bytes, Modifiers, Terminal};
use termian_native::terminal_settings::{font_face, TerminalSettings};

use super::login_view::{self, LoginActions, LoginTarget};
use super::shortcut_actions::{ClearTerminalBuffer, CopyFromTerminal, PasteToTerminal, SelectAllInTerminal};

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
    /// The session ended (the tab may stay open).
    Ended,
    /// «Detect OS» found the host's system.
    Os(String),
}

impl gpui::EventEmitter<TerminalEvent> for TerminalView {}

const PADDING: f32 = 10.;
/// Line height per pixel of font size (18 px at the old fixed 13 px).
const LINE_SPACING: f32 = 1.38;

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
    /// Where the output is recorded; dropping it finishes the log.
    log: Option<termian_native::session_log::SessionLog>,
    /// Set when the session closes; reported as [`TerminalEvent::Ended`].
    ended: bool,
    /// Settings → Terminal, as of the last change.
    settings: TerminalSettings,
    last_bell: Option<std::time::Instant>,
    /// Font family the faces below were worked out for, and the faces.
    faces: Option<(String, Faces)>,
    /// The shell was ready at least once (only then a lost session reconnects).
    was_ready: bool,
    /// «Autoreconnect» is retrying a lost session.
    reconnect: Option<session::Backoff>,
    /// A snippet typed once the shell is first ready (legacy `startupSnippet`).
    startup: Option<String>,
}

type Faces = ((SharedString, f32), (SharedString, f32));

/// Used when a bundled font's files cannot be found.
const FALLBACK_FONT: &str = "Menlo";

impl TerminalView {
    pub fn new(session: Session, settings: &TerminalSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::build(session, None, settings, window, cx)
    }

    pub fn ssh(connection: Connection, settings: &TerminalSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let session = session::ssh(connection.target.clone(), 80, 24);
        Self::build(session, Some(connection), settings, window, cx)
    }

    fn build(mut session: Session, connection: Option<Connection>, settings: &TerminalSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let login_focus = cx.focus_handle();
        window.focus(if connection.is_some() { &login_focus } else { &focus });
        Self::listen(&mut session, 0, window, cx);
        Self {
            terminal: Terminal::with_settings(80, 24, settings),
            session,
            focus,
            closed: false,
            cell_width: None,
            selecting: false,
            login: connection.as_ref().map(|_| LoginState::default()),
            connection,
            login_focus,
            generation: 0,
            log: None,
            ended: false,
            settings: settings.clone(),
            last_bell: None,
            faces: None,
            was_ready: false,
            reconnect: None,
            startup: None,
        }
    }

    /// New Settings → Terminal values: colours, font and scrollback change in place.
    pub fn apply_settings(&mut self, settings: &TerminalSettings, cx: &mut Context<Self>) {
        if settings.font_family != self.settings.font_family || settings.font_size != self.settings.font_size {
            self.cell_width = None;
        }
        self.terminal.apply(settings);
        self.settings = settings.clone();
        cx.notify();
    }

    fn font_size(&self) -> f32 {
        self.settings.font_size as f32
    }

    fn line_height(&self) -> f32 {
        (self.font_size() * LINE_SPACING).round()
    }

    /// Family and weight of regular and bold text.
    fn faces(&mut self) -> Faces {
        let family = &self.settings.font_family;
        if let Some((cached, faces)) = &self.faces {
            if cached == family {
                return faces.clone();
            }
        }
        let face = font_face(family);
        let faces = if face.files.is_empty() {
            let family = SharedString::from(family.clone());
            ((family.clone(), 400.), (family, 700.))
        } else if !termian_native::terminal_settings::font_available(&face) {
            ((FALLBACK_FONT.into(), 400.), (FALLBACK_FONT.into(), 700.))
        } else {
            ((face.regular.0.into(), face.regular.1 as f32), (face.bold.0.into(), face.bold.1 as f32))
        };
        self.faces = Some((family.clone(), faces.clone()));
        faces
    }

    /// The next reconnect attempt of a lost session, after the backoff delay.
    fn schedule_reconnect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(backoff) = self.reconnect.as_mut() else { return };
        let delay = backoff.next_delay();
        let generation = self.generation;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |view, window, cx| {
                if view.generation == generation && view.reconnect.is_some() {
                    view.reconnect_now(window, cx);
                }
            });
        })
        .detach();
    }

    /// Opens a new session in place of the lost one; the screen and log stay.
    fn reconnect_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(connection) = self.connection.clone() else { return };
        let (columns, rows) = self.terminal.size();
        self.generation += 1;
        self.session = session::ssh(connection.target, columns as u16, rows as u16);
        Self::listen(&mut self.session, self.generation, window, cx);
        cx.notify();
    }

    /// «Bell sound»: the legacy bell, at most a few times a second.
    fn ring(&mut self) {
        if !self.settings.bell_sound || self.last_bell.is_some_and(|at| at.elapsed() < std::time::Duration::from_millis(150)) {
            return;
        }
        self.last_bell = Some(std::time::Instant::now());
        play_bell();
    }

    /// Records everything the session prints to an encrypted session log.
    pub fn with_log(mut self, log: Option<termian_native::session_log::SessionLog>) -> Self {
        self.log = log;
        self
    }

    pub fn with_startup(mut self, script: Option<String>) -> Self {
        self.startup = script;
        self
    }

    /// A recorded session, read-only.
    pub fn replay(data: Vec<u8>, note: Option<&str>, settings: &TerminalSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut view = Self::build(session::replay(data), None, settings, window, cx);
        view.closed = true;
        if let Some(note) = note {
            view.terminal.feed(format!("\r\n\x1b[31m{note}\x1b[0m\r\n").as_bytes());
        }
        view
    }

    fn listen(session: &mut Session, generation: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mut events) = session.take_events() {
            cx.spawn_in(window, async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update_in(cx, |view, window, cx| {
                        if view.generation != generation {
                            return;
                        }
                        view.handle(event, window, cx);
                        if std::mem::take(&mut view.ended) {
                            cx.emit(TerminalEvent::Ended);
                        }
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

    fn handle(&mut self, event: SessionEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            SessionEvent::Login(LoginEvent::Stage(Stage::Ready)) => {
                self.login = None;
                self.closed = false;
                self.was_ready = true;
                if self.reconnect.take().is_some() {
                    self.terminal.feed(b"\r\n\x1b[2m[Reconnected]\x1b[0m\r\n");
                }
                if let Some(script) = self.startup.take() {
                    self.run(&script);
                }
                window.focus(&self.focus);
            }
            // A reconnect that needs an answer from the user shows the connection screen.
            SessionEvent::Login(event @ (LoginEvent::Prompt { .. } | LoginEvent::HostKey(_) | LoginEvent::HostKeyChanged(_))) if self.reconnect.is_some() => {
                self.reconnect = None;
                let mut login = LoginState::default();
                login.apply(event);
                self.login = Some(login);
                window.focus(&self.login_focus);
            }
            SessionEvent::Closed(reason) if self.reconnect.is_some() => {
                let reason = reason.unwrap_or_else(|| "The connection was closed.".into());
                self.terminal.feed(format!("\x1b[2m[Reconnect failed: {reason}]\x1b[0m\r\n").as_bytes());
                self.schedule_reconnect(window, cx);
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
                if let Some(log) = &self.log {
                    log.append(&bytes);
                }
                self.terminal.feed(&bytes);
                if self.terminal.take_bell() {
                    self.ring();
                }
                let replies = self.terminal.take_replies();
                if !replies.is_empty() {
                    self.session.write(&replies);
                }
            }
            SessionEvent::Closed(Some(reason))
                if reason.starts_with(session::CONNECTION_LOST) && self.was_ready && self.settings.autoreconnect && self.connection.is_some() =>
            {
                self.closed = true;
                self.terminal.feed(format!("\r\n\x1b[33m[{reason}. Reconnecting…]\x1b[0m\r\n").as_bytes());
                self.reconnect = Some(session::Backoff::default());
                self.schedule_reconnect(window, cx);
            }
            SessionEvent::Os(os) => {
                if let Some(connection) = self.connection.as_mut() {
                    connection.os.get_or_insert_with(|| os.clone());
                }
                cx.emit(TerminalEvent::Os(os));
            }
            SessionEvent::Closed(_) => {
                self.closed = true;
                self.log = None;
                self.ended = true;
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
        // Without «Use Option as Meta key» Option types the character macOS composes.
        let arrow = matches!(keystroke.key.as_str(), "up" | "down" | "left" | "right");
        let alt = mods.alt && (self.settings.option_is_meta || arrow || !cfg!(target_os = "macos"));
        let mods = Modifiers { control: mods.control, alt, shift: mods.shift };
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
        ((y / self.line_height()) as usize, (x / self.cell_width.unwrap_or(8.)).round() as usize)
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
        match self.terminal.selection_text() {
            None => self.terminal.clear_selection(),
            // «Select text to copy & Right click to paste».
            Some(text) if self.settings.copy_on_select => cx.write_to_clipboard(ClipboardItem::new_string(text)),
            Some(_) => {}
        }
        cx.notify();
    }

    fn on_right_click(&mut self, _: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        if self.closed || !self.settings.copy_on_select {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.terminal.scroll(i32::MIN / 2);
            self.session.write(&paste_bytes(&text, self.terminal.bracketed_paste()));
        }
        cx.stop_propagation();
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
        self.reconnect = None;
        self.generation += 1;
        self.session = session::ssh(connection.target, 80, 24);
        Self::listen(&mut self.session, self.generation, window, cx);
        self.terminal = Terminal::with_settings(80, 24, &self.settings);
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
            ScrollDelta::Pixels(delta) => f32::from(delta.y) / self.line_height(),
        };
        self.terminal.scroll(lines.round() as i32);
        cx.notify();
    }

    /// Fits the grid to the space under the tab bar.
    fn fit(&mut self, area_width: Pixels, area_height: Pixels, window: &Window) {
        let ((family, weight), _) = self.faces();
        let size = self.font_size();
        let cell_width = *self.cell_width.get_or_insert_with(|| {
            let text = window.text_system();
            let id = text.resolve_font(&gpui::Font { weight: gpui::FontWeight(weight), ..font(family) });
            text.advance(id, px(size), 'm').map(|size| f32::from(size.width)).unwrap_or(8.)
        });
        let columns = ((f32::from(area_width) - 2. * PADDING) / cell_width).floor().max(2.) as usize;
        let rows = ((f32::from(area_height) - 2. * PADDING) / self.line_height()).floor().max(1.) as usize;
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
        let line_height = self.line_height();
        let ((family, weight), (bold_family, bold_weight)) = self.faces();
        let look = self.terminal.look().clone();

        let lines = snapshot.lines.iter().map(|runs| {
            let mut line = div().h(px(line_height)).flex().whitespace_nowrap();
            for run in runs {
                line = line.child(
                    div()
                        .text_color(rgb(run.fg))
                        .when(run.bg != look.background, |d| d.bg(rgb(run.bg)))
                        .when(run.bold, |d| d.font_family(bold_family.clone()).font_weight(gpui::FontWeight(bold_weight)))
                        .child(SharedString::from(run.text.clone())),
                );
            }
            line
        });

        let cursor = snapshot.cursor.filter(|_| !self.closed).map(|(row, column)| {
            div()
                .absolute()
                .left(px(PADDING + column as f32 * cell_width))
                .top(px(PADDING + row as f32 * line_height))
                .w(px(cell_width))
                .h(px(line_height))
                .when(focused, |d| d.bg(rgb(look.cursor)).opacity(0.7))
                .when(!focused, |d| d.border_1().border_color(rgb(look.cursor)))
        });

        div()
            .id("terminal")
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(|view, _: &CopyFromTerminal, _, cx| {
                if let Some(text) = view.terminal.selection_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }))
            .on_action(cx.listener(|view, _: &PasteToTerminal, _, cx| {
                if let (false, Some(text)) = (view.closed, cx.read_from_clipboard().and_then(|item| item.text())) {
                    view.terminal.scroll(i32::MIN / 2);
                    view.session.write(&paste_bytes(&text, view.terminal.bracketed_paste()));
                }
            }))
            .on_action(cx.listener(|view, _: &SelectAllInTerminal, _, cx| {
                view.terminal.select_all();
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &ClearTerminalBuffer, _, cx| {
                view.terminal.clear_buffer();
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_right_click))
            .relative()
            .size_full()
            .p(px(PADDING))
            .overflow_hidden()
            .bg(rgb(look.background))
            .font_family(family)
            .font_weight(gpui::FontWeight(weight))
            .text_size(px(self.font_size()))
            .line_height(px(line_height))
            .children(lines)
            .children(cursor)
            .into_any_element()
    }
}

/// Plays the legacy bell sound without blocking the window.
fn play_bell() {
    static FILE: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    let path = FILE.get_or_init(|| {
        let path = std::env::temp_dir().join("termian-bell.wav");
        std::fs::write(&path, include_bytes!("../../assets/sounds/bell.wav")).ok().map(|_| path)
    });
    let Some(path) = path.clone() else { return };
    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("/usr/bin/afplay").arg(&path).status();
        #[cfg(windows)]
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path.display())])
            .status();
        #[cfg(not(any(target_os = "macos", windows)))]
        let _ = path;
    });
}
