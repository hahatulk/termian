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

use super::text_input::{self, Mode};
use super::login_view::{self, LoginActions, LoginTarget};
#[path = "serial_form.rs"]
mod serial_form;

use super::assets::icon;
use super::shortcut_actions::{ClearTerminalBuffer, CopyFromTerminal, OpenTerminalSearch, PasteToTerminal, SelectAllInTerminal};

/// Where an SSH tab connects to; kept so the connection can be retried.
#[derive(Clone)]
pub struct Connection {
    pub label: String,
    pub os: Option<String>,
    pub target: SshTarget,
    /// Offered on the Username question.
    pub identities: Vec<termian_native::login::IdentityChoice>,
    /// A saved host: «Continue & Save» can keep the typed username in it.
    pub saved_host: bool,
}

/// Requests from the tab to the window.
pub enum TerminalEvent {
    Close,
    /// The session ended (the tab may stay open).
    Ended,
    /// «Edit host» on the failed connection screen.
    EditHost,
    /// «Continue & Save»: the typed username goes into the saved host.
    SaveUsername(String),
    /// «Detect OS» found the host's system.
    Os(String),
    /// A serial port opened (its label); the window records the session.
    SerialConnected(String),
    /// Bytes the user typed or pasted (for Broadcast input).
    Input(Vec<u8>),
    /// Shell integration saw a command finish.
    Command(String),
    /// Existing shell history extracted outside the terminal stream.
    History(Vec<String>),
}

/// Shell integration of one session: installed once the output is quiet,
/// commands reported after the hooks say they are ready.
#[derive(Default)]
struct Integration {
    scanner: termian_native::shell_integration::Scanner,
    /// Wanted for this tab (a shell, and Settings → Terminal allows it).
    wanted: bool,
    waiting: bool,
    answered: bool,
    ready: bool,
    command: Option<String>,
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
    /// The shell became ready: the connection screen hands over (legacy animation).
    connected: Option<(std::time::Instant, u64)>,
    /// The host's «Backspace: Ctrl+H»: Backspace sends ^H instead of DEL.
    ctrl_h_backspace: bool,
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
    /// «Search terminal»: the query while the bar is open, and whether it matched.
    search: Option<String>,
    search_found: bool,
    search_focus: FocusHandle,
    /// The pane's place in the window (x, y, width, height); the whole area
    /// under the tab bar until the window sets it.
    area: Option<(f32, f32, f32, f32)>,
    /// The Serial tab's form, until it connects.
    serial_form: Option<serial_form::SerialForm>,
    integration: Integration,
    /// The screen as it was before the install line: shown instead of the
    /// alternate screen the line is typed on, until the shell leaves it.
    frozen: Option<termian_native::terminal::Snapshot>,
    last_output: std::time::Instant,
    autocomplete: Autocomplete,
}

/// The inline autocomplete popup of this terminal.
#[derive(Default)]
struct Autocomplete {
    /// Settings allow it and the pane is not split (legacy keeps it off there).
    enabled: bool,
    candidates: std::sync::Arc<termian_native::autocomplete::Candidates>,
    line: termian_native::autocomplete::Line,
    shown: Vec<termian_native::autocomplete::Suggestion>,
    selected: Option<usize>,
    dismissed: bool,
}

type Faces = ((SharedString, f32), (SharedString, f32));

/// Used when a bundled font's files cannot be found.
const FALLBACK_FONT: &str = "Menlo";

impl TerminalView {
    /// The login screen is up and has not failed: the tab is shown green.
    pub fn is_connecting(&self) -> bool {
        self.login.as_ref().is_some_and(|login| login.error.is_none())
    }

    /// A local shell; it gets the shell integration once it is quiet.
    pub fn new(session: Session, settings: &TerminalSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut view = Self::build(session, None, settings, window, cx);
        if settings.shell_integration && !cfg!(windows) {
            view.integration.wanted = true;
            view.schedule_integration(window, cx);
        }
        view
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
            connected: None,
            ctrl_h_backspace: false,
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
            search: None,
            search_found: true,
            search_focus: cx.focus_handle(),
            area: None,
            serial_form: None,
            integration: Integration::default(),
            frozen: None,
            last_output: std::time::Instant::now(),
            autocomplete: Autocomplete::default(),
        }
    }

    /// A Serial tab: the legacy form, then the port's session.
    pub fn serial(settings: &TerminalSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut view = Self::build(session::replay(Vec::new()), None, settings, window, cx);
        view.serial_form = Some(serial_form::SerialForm::new());
        view.closed = true;
        window.focus(&view.login_focus);
        view.watch_serial_ports(window, cx);
        view
    }

    /// New Settings → Terminal values: colours, font and scrollback change in place.
    pub fn apply_settings(&mut self, settings: &TerminalSettings, cx: &mut Context<Self>) {
        if settings.font_family != self.settings.font_family || settings.font_size != self.settings.font_size {
            self.cell_width = None;
        }
        self.terminal.apply(settings);
        self.settings = settings.clone();
        if let Some(connection) = self.connection.as_mut() {
            connection.target.options.import_history = settings.shell_history_import;
        }
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

    /// A startup snippet for a local shell that is already open: typed after
    /// the shell integration line, or at once without it.
    pub fn set_local_startup(&mut self, script: String) {
        if self.integration.wanted {
            self.startup = Some(script);
        } else {
            self.run(&script);
        }
    }

    pub fn with_ctrl_h_backspace(mut self, on: bool) -> Self {
        self.ctrl_h_backspace = on;
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
                // A first connection or a reconnect from the connection screen hands over with the legacy animation.
                if self.login.take().is_some() {
                    let key = self.generation;
                    self.connected = Some((std::time::Instant::now(), key));
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(login_view::CONNECTED_TOTAL).await;
                        let _ = this.update(cx, |view, cx| {
                            if view.connected.is_some_and(|(_, k)| k == key) {
                                view.connected = None;
                                cx.notify();
                            }
                        });
                    })
                    .detach();
                }
                self.closed = false;
                self.was_ready = true;
                if self.reconnect.take().is_some() {
                    self.terminal.feed(b"\r\n\x1b[2m[Reconnected]\x1b[0m\r\n");
                }
                if self.settings.shell_integration {
                    // A new shell (also after a reconnect) gets the hooks; a
                    // startup snippet is typed after them.
                    self.integration = Integration { wanted: true, ..Integration::default() };
                    self.schedule_integration(window, cx);
                } else if let Some(script) = self.startup.take() {
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
                self.last_output = std::time::Instant::now();
                let (bytes, events) = self.integration.scanner.scan(&bytes);
                for event in events {
                    self.shell_event(event, cx);
                }
                if let Some(log) = &self.log {
                    log.append(&bytes);
                }
                self.terminal.feed(&bytes);
                if self.frozen.is_some() && !self.terminal.alt_screen() {
                    self.frozen = None;
                }
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
            SessionEvent::History(commands) => {
                if self.settings.shell_history_import && !self.closed {
                    cx.emit(TerminalEvent::History(commands));
                }
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

    /// Legacy search field keys: typing searches up from the last match,
    /// Enter finds the previous match, Shift+Enter the next, Escape closes.
    fn on_search_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let Some(query) = self.search.as_mut() else { return };
        use super::dialog::Key;
        match text_input::edit("terminal-search", Some(query), keystroke, Mode::Line, cx) {
            Key::Escape => self.close_search(window),
            Key::Enter => {
                let query = query.clone();
                self.search_found = self.terminal.find(&query, !keystroke.modifiers.shift, false);
            }
            Key::Edited => {
                let query = query.clone();
                self.search_found = self.terminal.find(&query, true, true) || query.is_empty();
            }
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Opens «Search terminal» (⌘F, or the side panel's search row).
    pub fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search.get_or_insert_with(String::new);
        window.focus(&self.search_focus);
        cx.notify();
    }

    /// The terminal theme's background and text, for tinting the window around it.
    pub fn colors(&self) -> (u32, u32) {
        let look = self.terminal.look();
        (look.background, look.foreground)
    }

    pub fn step_search(&mut self, previous: bool, cx: &mut Context<Self>) {
        if let Some(query) = self.search.clone() {
            self.search_found = self.terminal.find(&query, previous, false);
            cx.notify();
        }
    }

    fn close_search(&mut self, window: &mut Window) {
        self.search = None;
        self.search_found = true;
        self.terminal.clear_search();
        self.terminal.clear_selection();
        window.focus(&self.focus);
    }

    fn render_search(&self, window: &Window, cx: &mut Context<Self>) -> Option<gpui::Stateful<gpui::Div>> {
        let query = self.search.as_ref()?;
        let focused = self.search_focus.is_focused(window);
        let arrow = |id: &str, up: bool| {
            div()
                .id(SharedString::from(id.to_owned()))
                .size(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .hover(|s| s.bg(rgb(termian_native::theme::SURFACE_ACTIVE)))
                .child(
                    gpui::svg()
                        .path(icon("caret_down"))
                        .size(px(10.))
                        .text_color(rgb(termian_native::theme::TEXT))
                        .when(up, |d| d.with_transformation(gpui::Transformation::rotate(gpui::radians(std::f32::consts::PI)))),
                )
        };
        let empty = query.is_empty();
        let field = div()
                .id("terminal-search")
                .track_focus(&self.search_focus)
                .on_key_down(cx.listener(Self::on_search_key))
                .on_mouse_down(MouseButton::Left, cx.listener(|view, _, window, cx| {
                    cx.stop_propagation();
                    window.focus(&view.search_focus);
                }))
                .absolute()
                .top(px(8.))
                .right(px(16.))
                .w(px(300.))
                .h(px(30.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded(px(5.))
                .bg(rgb(termian_native::theme::INPUT))
                .border_1()
                .border_color(rgb(if !self.search_found { termian_native::theme::ERROR } else if focused { termian_native::theme::ACCENT } else { termian_native::theme::INPUT }))
                .font_family(".SystemUIFont")
                .text_xs()
                .child(gpui::svg().path(icon("search")).size(px(12.)).text_color(rgb(if focused || !empty { termian_native::theme::TEXT } else { termian_native::theme::TEXT_MUTED })))
                .child(div().flex_1().min_w_0().child(text_input::text("terminal-search", query, "Search terminal", focused, Mode::Line, cx)))
                .child(arrow("terminal-search-prev", true).on_click(cx.listener(|view, _, _, cx| view.step_search(true, cx))))
                .child(arrow("terminal-search-next", false).on_click(cx.listener(|view, _, _, cx| view.step_search(false, cx))));
        Some(text_input::field(field, "terminal-search", Mode::Line, |view, window, _| window.focus(&view.search_focus), cx))
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
                    let bytes = paste_bytes(&text, self.terminal.bracketed_paste());
                    self.session.write(&bytes);
                    cx.emit(TerminalEvent::Input(bytes));
                    self.autocomplete.line.insert(&text);
                    self.refresh_suggestions();
                }
            }
            return; // other Cmd shortcuts belong to the app
        }
        if !mods.control && !mods.alt && self.autocomplete_key(&keystroke.key, cx) {
            cx.stop_propagation();
            return;
        }
        // Without «Use Option as Meta key» Option types the character macOS composes.
        let arrow = matches!(keystroke.key.as_str(), "up" | "down" | "left" | "right");
        let alt = mods.alt && (self.settings.option_is_meta || arrow || !cfg!(target_os = "macos"));
        let mods = Modifiers { control: mods.control, alt, shift: mods.shift };
        if let Some(mut bytes) = key_bytes(&keystroke.key, keystroke.key_char.as_deref(), mods, self.terminal.app_cursor()) {
            if self.ctrl_h_backspace && bytes == [0x7f] {
                bytes = vec![0x08];
            }
            self.terminal.scroll(i32::MIN / 2); // typing jumps back to the prompt
            self.terminal.clear_selection();
            self.session.write(&bytes);
            cx.emit(TerminalEvent::Input(bytes));
            self.autocomplete.line.key(&keystroke.key, keystroke.key_char.as_deref(), mods.control || mods.alt);
            self.refresh_suggestions();
            cx.stop_propagation();
            cx.notify();
        }
    }

    /// Grid cell under a window position.
    fn cell_at(&self, position: Point<Pixels>) -> (usize, usize) {
        let (left, top) = self.area.map_or((0., TOP_INSET), |(x, y, _, _)| (x, y));
        let x = (f32::from(position.x) - left - PADDING).max(0.);
        let y = (f32::from(position.y) - top - PADDING).max(0.);
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
            let bytes = paste_bytes(&text, self.terminal.bracketed_paste());
            self.session.write(&bytes);
            cx.emit(TerminalEvent::Input(bytes));
        }
        cx.stop_propagation();
    }

    /// The pane's place in the window; the grid refits when it renders.
    pub fn set_area(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.area = Some((x, y, width, height));
    }

    /// Input from another pane while broadcasting, as if typed here.
    pub fn write_input(&mut self, bytes: &[u8]) {
        if !self.closed && self.serial_form.is_none() && self.login.is_none() {
            self.terminal.scroll(i32::MIN / 2);
            self.session.write(bytes);
        }
    }

    /// Waits until the shell has been quiet for a moment (a login banner is
    /// done), then installs the hooks.
    fn schedule_integration(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.integration.waiting = true;
        let generation = self.generation;
        cx.spawn_in(window, async move |this, cx| {
            let begun = std::time::Instant::now();
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(100)).await;
                let done = this.update(cx, |view, cx| {
                    if view.generation != generation || !view.integration.waiting {
                        return true;
                    }
                    let quiet = view.last_output.elapsed() >= std::time::Duration::from_millis(300);
                    if quiet || begun.elapsed() >= std::time::Duration::from_secs(3) {
                        view.install_integration(cx);
                        return true;
                    }
                    false
                });
                if !matches!(done, Ok(false)) {
                    break;
                }
            }
        })
        .detach();
    }

    /// Types the install line behind the alternate screen; the line leaves
    /// it, and if the shell cannot run it the terminal leaves it anyway.
    fn install_integration(&mut self, cx: &mut Context<Self>) {
        self.integration.waiting = false;
        if self.closed {
            return;
        }
        const ENTER_ALT: &[u8] = b"\x1b[?1049h";
        self.frozen = Some(self.terminal.snapshot());
        self.terminal.feed(ENTER_ALT);
        if let Some(log) = &self.log {
            log.append(ENTER_ALT);
        }
        // A local shell has drawn only its prompt: the line clears the screen
        // so that prompt is not shown twice.
        let line = termian_native::shell_integration::install_line(self.connection.is_none());
        self.session.write(&paste_bytes(line.trim_end_matches('\r'), self.terminal.bracketed_paste()));
        self.session.write(b"\r");
        if let Some(script) = self.startup.take() {
            self.run(&script);
        }
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_secs(4)).await;
            let _ = this.update(cx, |view, cx| {
                if view.generation == generation && !view.integration.answered {
                    const LEAVE_ALT: &[u8] = b"\x1b[?1049l\r\x1b[K";
                    view.terminal.feed(LEAVE_ALT);
                    view.frozen = None;
                    if let Some(log) = &view.log {
                        log.append(LEAVE_ALT);
                    }
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn shell_event(&mut self, event: termian_native::shell_integration::ShellEvent, cx: &mut Context<Self>) {
        use termian_native::shell_integration::ShellEvent;
        let state = &mut self.integration;
        match event {
            ShellEvent::Installing => state.answered = true,
            ShellEvent::Ready => {
                state.answered = true;
                state.ready = true;
            }
            // Events before Ready come from the install line itself.
            ShellEvent::CommandStarted(command) if state.ready => {
                state.command = Some(command);
                self.autocomplete.line.stop();
                self.refresh_suggestions();
            }
            ShellEvent::PromptEnds if state.ready => {
                self.autocomplete.line.reset();
                self.autocomplete.dismissed = false;
                self.refresh_suggestions();
            }
            ShellEvent::CommandExited(_) => {
                if let Some(command) = state.command.take() {
                    cx.emit(TerminalEvent::Command(command));
                }
            }
            _ => {}
        }
    }

    /// Autocomplete is allowed here, with these candidates (the window keeps them current).
    pub fn set_autocomplete(&mut self, enabled: bool, candidates: std::sync::Arc<termian_native::autocomplete::Candidates>) {
        let changed = !std::sync::Arc::ptr_eq(&self.autocomplete.candidates, &candidates) || self.autocomplete.enabled != enabled;
        self.autocomplete.enabled = enabled;
        self.autocomplete.candidates = candidates;
        if changed {
            self.refresh_suggestions();
        }
    }

    fn refresh_suggestions(&mut self) {
        let a = &mut self.autocomplete;
        a.shown = match a.line.text() {
            Some(typed) if a.enabled && !a.dismissed => a.candidates.matching(typed),
            _ => Vec::new(),
        };
        if a.selected.is_some_and(|i| i >= a.shown.len()) {
            a.selected = None;
        }
    }

    /// Keys the open popup takes: ↑/↓ choose, Tab or → insert, Enter runs a
    /// chosen suggestion, Escape closes it until the next prompt.
    fn autocomplete_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let a = &mut self.autocomplete;
        if a.shown.is_empty() {
            return false;
        }
        let count = a.shown.len();
        match key {
            "down" => a.selected = Some(a.selected.map_or(0, |i| (i + 1) % count)),
            "up" => a.selected = Some(a.selected.map_or(count - 1, |i| (i + count - 1) % count)),
            "escape" => {
                a.dismissed = true;
                a.selected = None;
                a.shown.clear();
            }
            "tab" | "right" => {
                let index = a.selected.unwrap_or(0);
                self.accept_suggestion(index, false, cx);
            }
            "enter" if a.selected.is_some() => {
                let index = a.selected.unwrap_or(0);
                self.accept_suggestion(index, true, cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    fn accept_suggestion(&mut self, index: usize, run: bool, cx: &mut Context<Self>) {
        let a = &mut self.autocomplete;
        let (Some(typed), Some(suggestion)) = (a.line.text().map(str::to_owned), a.shown.get(index).cloned()) else { return };
        let rest = suggestion.rest(&typed).to_owned();
        a.line.insert(&rest);
        a.selected = None;
        let mut bytes = rest.into_bytes();
        if run {
            bytes.push(b'\r');
            self.autocomplete.line.stop();
        }
        self.terminal.scroll(i32::MIN / 2);
        self.session.write(&bytes);
        cx.emit(TerminalEvent::Input(bytes));
        self.refresh_suggestions();
    }

    /// The popup under the cursor (above it near the bottom), as legacy draws it.
    fn render_autocomplete(&self, cursor: Option<(usize, usize)>, cell_width: f32, line_height: f32, cx: &mut Context<Self>) -> Option<gpui::Div> {
        let a = &self.autocomplete;
        let (row, column) = cursor?;
        let typed = a.line.text()?;
        if a.shown.is_empty() {
            return None;
        }
        let (_, rows) = self.terminal.size();
        let start = column.saturating_sub(typed.chars().count());
        let left = PADDING + start as f32 * cell_width;
        let height = a.shown.len() as f32 * 26. + 8.;
        let below = PADDING + (row + 1) as f32 * line_height;
        let top = if row + 1 + (height / line_height).ceil() as usize > rows { (PADDING + row as f32 * line_height - height).max(0.) } else { below };
        let rows = a.shown.iter().enumerate().map(|(i, s)| {
            let selected = a.selected == Some(i);
            let source = match &s.source {
                termian_native::autocomplete::Source::History => "history".to_owned(),
                termian_native::autocomplete::Source::Snippet(label) => label.clone(),
            };
            div()
                .id(SharedString::from(format!("suggestion-{i}")))
                .h(px(26.))
                .px_2()
                .flex()
                .items_center()
                .gap_3()
                .rounded_sm()
                .when(selected, |d| d.bg(rgb(termian_native::theme::SURFACE_ACTIVE)))
                .child(div().flex_1().min_w_0().truncate().child(div().flex().child(div().text_color(rgb(termian_native::theme::TEXT)).child(typed.to_owned())).child(div().text_color(rgb(termian_native::theme::TEXT_MUTED)).child(s.rest(typed).to_owned()))))
                .child(div().flex_none().text_xs().font_family(".SystemUIFont").text_color(rgb(termian_native::theme::TEXT_MUTED)).child(source))
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                    cx.stop_propagation();
                    view.accept_suggestion(i, false, cx);
                    cx.notify();
                }))
        });
        Some(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(420.))
                .p_1()
                .rounded_md()
                .bg(rgb(termian_native::theme::SURFACE))
                .border_1()
                .border_color(rgb(termian_native::theme::SURFACE_ACTIVE))
                .shadow_lg()
                .children(rows),
        )
    }

    /// Pastes `text` into the session without running it (legacy «Paste»).
    pub fn paste(&mut self, text: &str) {
        if !self.closed {
            self.terminal.scroll(i32::MIN / 2);
            self.session.write(&paste_bytes(text, self.terminal.bracketed_paste()));
        }
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
        window.focus(if self.login.is_some() || self.serial_form.is_some() { &self.login_focus } else { &self.focus });
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
        let Some(login) = self.login.as_mut() else { return };
        if let Some(answer) = login_view::edit(login, event, cx) {
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

    fn login_continue_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(username) = self.login.as_ref().map(|l| l.input.trim().to_owned()).filter(|u| !u.is_empty()) {
            cx.emit(TerminalEvent::SaveUsername(username));
        }
        self.login_submit(window, cx);
    }

    fn login_ui(&mut self, ui: termian_native::login::LoginUi, cx: &mut Context<Self>) {
        if let Some(login) = self.login.as_mut() {
            login.apply_ui(ui);
        }
        cx.notify();
    }

    fn login_edit_host(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TerminalEvent::EditHost);
    }

    /// «Verbose (-v)»: starts over with `debug1:` lines and the log open.
    fn login_verbose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(connection) = self.connection.as_mut() {
            connection.target.options.verbose = true;
        }
        if let Some(login) = self.login.as_mut() {
            login.show_logs = true;
        }
        self.login_retry(window, cx);
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
                identities: &connection.identities,
            };
            let actions = LoginActions {
                answer: Self::login_answer,
                close: Self::login_close,
                retry: Self::login_retry,
                toggle_logs: Self::login_toggle_logs,
                key: Self::login_key,
                submit: Self::login_submit,
                edit_host: Some(Self::login_edit_host),
                verbose: Some(Self::login_verbose),
                continue_save: connection.saved_host.then_some(Self::login_continue_save as fn(&mut Self, &mut Window, &mut Context<Self>)),
                ui: Some(Self::login_ui),
            };
            return login_view::render(target, login, &self.login_focus, window, &actions, cx).into_any_element();
        }
        if let Some(form) = self.render_serial_form(window, cx) {
            return form.into_any_element();
        }
        let viewport = window.viewport_size();
        let (width, height) = self.area.map_or((viewport.width, viewport.height - px(TOP_INSET)), |(_, _, w, h)| (px(w), px(h)));
        self.fit(width, height, window);
        let snapshot = self.frozen.clone().unwrap_or_else(|| self.terminal.snapshot());
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
                    let bytes = paste_bytes(&text, view.terminal.bracketed_paste());
                    view.session.write(&bytes);
                    cx.emit(TerminalEvent::Input(bytes));
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
            .on_action(cx.listener(|view, _: &OpenTerminalSearch, window, cx| view.open_search(window, cx)))
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
            .children(self.render_autocomplete(snapshot.cursor.filter(|_| !self.closed && focused), cell_width, line_height, cx))
            .children(self.render_search(window, cx))
            .children(self.connected.and_then(|(_, key)| {
                let connection = self.connection.as_ref()?;
                Some(login_view::render_connected(&connection.label, format!("SSH {}:{}", connection.target.host, connection.target.port), connection.os.as_deref(), key))
            }))
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
