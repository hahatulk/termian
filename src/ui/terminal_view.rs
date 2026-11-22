//! A terminal tab: draws the emulator grid and forwards keys to the session.

use termian_native::language::text as tr;
use alacritty_terminal::index::Side;
use futures::StreamExt;
use gpui::{
    canvas, div, fill, font, outline, point, prelude::*, px, rgb, size, App, BorderStyle, Bounds, ClipboardItem,
    Context, DispatchPhase, FocusHandle, Focusable, Font, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, ScrollDelta, ScrollWheelEvent, SharedString, TextRun, Window,
};
use std::{cell::Cell, rc::Rc};
use termian_native::login::{Answer, LoginEvent, LoginProgress, LoginState, Stage};
use termian_native::session::{self, Session, SessionEvent, SshTarget};
use termian_native::terminal::mouse::{Button as TerminalButton, Event as TerminalMouse};
use termian_native::terminal::{paste_bytes, Modifiers, Run, Terminal};
use termian_native::terminal_settings::{font_face, TerminalSettings};

use super::pointer::PointerFeedback;
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
    Connected,
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

#[derive(Clone, Copy)]
struct CellGrid {
    origin: Point<Pixels>,
    cell_width: Pixels,
    line_height: Pixels,
}

impl CellGrid {
    fn span(self, row: usize, column: usize, columns: usize) -> Bounds<Pixels> {
        Bounds::new(
            self.origin + point(self.cell_width * column, self.line_height * row),
            size(self.cell_width * columns, self.line_height),
        )
    }

    fn contains(self, position: Point<Pixels>, rows: usize, columns: usize) -> bool {
        let offset = position - self.origin;
        offset.x >= px(0.) && offset.y >= px(0.)
            && offset.x < self.cell_width * columns && offset.y < self.line_height * rows
    }

    fn edge_scroll_lines(self, y: Pixels, rows: usize) -> i32 {
        let y = f32::from(y - self.origin.y);
        let line_height = f32::from(self.line_height);
        let edge = line_height * 0.25;
        let bottom = line_height * rows as f32 - edge;
        let (distance, direction) = if y < edge {
            (edge - y, 1)
        } else if y >= bottom {
            (y - bottom, -1)
        } else {
            return 0;
        };
        direction * (1 + (distance / 8.).min(23.) as i32)
    }

    fn hit(self, position: Point<Pixels>, rows: usize, columns: usize) -> (usize, usize, Side) {
        let local = position - self.origin;
        let row = ((local.y / self.line_height).floor() as usize).min(rows - 1);
        let x = local.x / self.cell_width;
        let column = (x.floor() as usize).min(columns - 1);
        let side = if x - column as f32 >= 0.5 { Side::Right } else { Side::Left };
        (row, column, side)
    }
}

fn paint_run(run: &Run, origin: Point<Pixels>, grid: CellGrid, size: Pixels, face: &Font, window: &mut Window) {
    let text = run.text.trim_end_matches(' ');
    if text.is_empty() {
        return;
    }
    let color = rgb(run.fg).into();
    let layout = window.text_system().shape_line(
        SharedString::from(text.to_owned()), size,
        &[TextRun { len: text.len(), font: face.clone(), color, background_color: None, underline: None, strikethrough: None }],
        None,
    );
    let baseline = origin.y + (grid.line_height + layout.ascent - layout.descent) / 2.;
    let offsets = (!text.is_ascii()).then(|| text.char_indices().map(|(offset, _)| offset).collect::<Vec<_>>());
    let mut cluster = None;
    for shaped in &layout.runs {
        for glyph in &shaped.glyphs {
            let (offset, cluster_x) = *cluster.get_or_insert((glyph.index, glyph.position.x));
            let cluster_x = if offset == glyph.index {
                cluster_x
            } else {
                cluster = Some((glyph.index, glyph.position.x));
                glyph.position.x
            };
            let column = offsets.as_ref().map_or(Some(glyph.index), |offsets| offsets.binary_search(&glyph.index).ok());
            let Some(column) = column else { continue };
            let origin = point(
                origin.x + grid.cell_width * (column * run.cell_columns) + (glyph.position.x - cluster_x),
                baseline,
            );
            if glyph.is_emoji {
                let _ = window.paint_emoji(origin, shaped.font_id, glyph.id, size);
            } else {
                let _ = window.paint_glyph(origin, shaped.font_id, glyph.id, size, color);
            }
        }
    }
}

const PADDING: f32 = 10.;
/// Line height per pixel of font size (18 px at the old fixed 13 px).
const LINE_SPACING: f32 = 1.38;

pub struct TerminalView {
    terminal: Terminal,
    session: Session,
    focus: FocusHandle,
    _keyboard: gpui::Subscription,
    _selection_blur: gpui::Subscription,
    closed: bool,
    cell_width: Option<f32>,
    grid_origin: Rc<Cell<Point<Pixels>>>,
    selection_drag: Option<SelectionDrag>,
    mouse_scroll: Point<f32>,
    local_scroll: f32,
    copied: Option<gpui::Task<()>>,
    /// SSH tabs show the connection screen until the shell is ready.
    connection: Option<Connection>,
    login: Option<LoginState>,
    /// The shell became ready: the connection screen hands over (legacy animation).
    connected: Option<LoginProgress>,
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
    area: Option<(f32, f32)>,
    /// The Serial tab's form, until it connects.
    serial_form: Option<serial_form::SerialForm>,
    integration: Integration,
    /// The screen as it was before the install line: shown instead of the
    /// alternate screen the line is typed on, until the shell leaves it.
    frozen: Option<termian_native::terminal::Snapshot>,
    last_output: std::time::Instant,
    autocomplete: Autocomplete,
}

#[derive(Default)]
struct SelectionDrag {
    autoscroll: Option<gpui::Task<()>>,
}

/// The inline autocomplete popup of this terminal.
#[derive(Default)]
struct Autocomplete {
    /// Settings allow it and the pane is not split (legacy keeps it off there).
    enabled: bool,
    candidates: std::sync::Arc<termian_native::autocomplete::Candidates>,
    line: termian_native::autocomplete::Line,
    shown: Vec<termian_native::autocomplete::Suggestion>,
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
        let keyboard_focus = focus.clone();
        let view = cx.weak_entity();
        let keyboard = cx.intercept_keystrokes(move |event, window, cx| {
            if keyboard_focus.is_focused(window) && !event.keystroke.modifiers.platform
                && event.context_stack.iter().any(|context| context.contains("Terminal"))
            {
                let _ = view.update(cx, |view, cx| view.input_key(&event.keystroke, cx));
            }
        });
        let selection_blur = cx.on_blur(&focus, window, |view, _, _| {
            view.selection_drag = None;
        });
        Self {
            terminal: Terminal::with_settings(80, 24, settings),
            session,
            focus,
            _keyboard: keyboard,
            _selection_blur: selection_blur,
            closed: false,
            cell_width: None,
            grid_origin: Rc::default(),
            selection_drag: None,
            mouse_scroll: point(0., 0.),
            local_scroll: 0.,
            copied: None,
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
    pub fn stop_log(&mut self) {
        if let Some(log) = self.log.take() { log.finish(); }
    }

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
        let mut view = Self::build(session::replay(Vec::new()), None, settings, window, cx);
        view.closed = true;
        let note = note.map(str::to_owned);
        cx.spawn_in(window, async move |this, cx| {
            for bytes in data.chunks(32 * 1024) {
                if this.update(cx, |view, cx| { view.terminal.feed(bytes); cx.notify(); }).is_err() { return; }
                cx.background_executor().timer(std::time::Duration::from_millis(1)).await;
            }
            if let Some(note) = note {
                let _ = this.update(cx, |view, cx| {
                    view.terminal.feed(format!("\r\n\x1b[31m{note}\x1b[0m\r\n").as_bytes());
                    cx.notify();
                });
            }
        }).detach();
        view
    }

    fn listen(session: &mut Session, generation: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mut events) = session.take_events() {
            cx.spawn_in(window, async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update_in(cx, |view, window, cx| {
                        if view.generation != generation {
                            return None;
                        }
                        let pending = view.handle(event, window, cx);
                        if std::mem::take(&mut view.ended) {
                            cx.emit(TerminalEvent::Ended);
                        }
                        cx.notify();
                        pending
                    });
                    match alive {
                        Err(_) => break,
                        Ok(Some(write)) => cx.background_executor().spawn(async move { write(); }).await,
                        Ok(None) => {}
                    }
                }
            })
            .detach();
        }
    }

    fn handle(&mut self, event: SessionEvent, window: &mut Window, cx: &mut Context<Self>) -> Option<Box<dyn FnOnce() + Send>> {
        match event {
            SessionEvent::Login(LoginEvent::Stage(Stage::Ready)) => {
                if let Some(mut login) = self.login.take() {
                    login.apply(LoginEvent::Stage(Stage::Ready));
                    if super::motion::enabled() {
                        let motion = login.motion;
                        let key = self.generation;
                        self.connected = Some(motion);
                        if let Some(finish) = motion.finish_at() {
                            let timer = cx.background_executor().timer(finish.saturating_duration_since(std::time::Instant::now()));
                            cx.spawn(async move |this, cx| {
                                timer.await;
                                let _ = this.update(cx, |view, cx| {
                                    if view.generation == key && view.connected == Some(motion) {
                                        view.connected = None;
                                        cx.notify();
                                    }
                                });
                            })
                            .detach();
                        }
                    }
                }
                self.closed = false;
                if !self.was_ready {
                    cx.emit(TerminalEvent::Connected);
                }
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
                return self.log.as_ref().and_then(|log| log.queue(bytes));
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
        None
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
                .icon_feedback()
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
                .child(div().flex_1().min_w_0().child(text_input::text("terminal-search", query, tr("Search terminal"), focused, Mode::Line, cx)))
                .child(arrow("terminal-search-prev", true).on_click(cx.listener(|view, _, _, cx| view.step_search(true, cx))))
                .child(arrow("terminal-search-next", false).on_click(cx.listener(|view, _, _, cx| view.step_search(false, cx))));
        Some(text_input::field(field, "terminal-search", Mode::Line, |view, window, _| window.focus(&view.search_focus), cx))
    }

    fn accepts_input(&self) -> bool {
        !self.closed && self.login.is_none() && self.serial_form.is_none()
    }

    fn input_key(&mut self, keystroke: &gpui::Keystroke, cx: &mut Context<Self>) {
        if !self.accepts_input() {
            return;
        }
        cx.stop_propagation();
        let mods = keystroke.modifiers;
        let character = keystroke.key == "space" || keystroke.key.chars().count() == 1;
        let alt = mods.alt && (!cfg!(target_os = "macos") || self.settings.option_is_meta || mods.control || !character);
        let typed = if cfg!(target_os = "macos") && alt && character {
            None
        } else {
            keystroke.key_char.as_deref()
        };
        let mods = Modifiers { control: mods.control, alt, shift: mods.shift };
        if let Some(bytes) = self.terminal.key_input(&keystroke.key, typed, mods, self.ctrl_h_backspace) {
            self.selection_drag = None;
            self.terminal.scroll(i32::MIN / 2);
            self.terminal.clear_selection();
            if bytes.is_ascii() {
                self.session.write_raw(bytes.clone());
            } else {
                self.session.write(&bytes);
            }
            cx.emit(TerminalEvent::Input(bytes));
            self.autocomplete.line.key(&keystroke.key, typed, mods.control || mods.alt);
            if keystroke.key == "escape" {
                self.autocomplete.dismissed = true;
            }
            self.refresh_suggestions();
            cx.notify();
        }
    }

    pub(super) fn copy_selection(&self, cx: &mut Context<Self>) {
        if let Some(text) = self.terminal.selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub(super) fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        if !self.accepts_input() {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.terminal.scroll(i32::MIN / 2);
            let bytes = paste_bytes(&text, self.terminal.bracketed_paste());
            self.session.write(&bytes);
            cx.emit(TerminalEvent::Input(bytes));
            self.autocomplete.line.insert(&text);
            self.refresh_suggestions();
            cx.notify();
        }
    }

    fn grid(&self) -> CellGrid {
        CellGrid {
            origin: self.grid_origin.get(),
            cell_width: px(self.cell_width.unwrap_or(8.)),
            line_height: px(self.line_height()),
        }
    }

    fn selection_cell(&self, position: Point<Pixels>) -> (usize, usize, Side) {
        let (columns, rows) = self.terminal.size();
        self.grid().hit(position, rows, columns)
    }

    fn mouse_tracking(&self) -> bool {
        !self.closed && self.frozen.is_none() && self.terminal.mouse_tracking()
    }

    fn send_mouse(&mut self, event: TerminalMouse, position: Point<Pixels>, modifiers: gpui::Modifiers, clamp: bool) -> bool {
        if self.closed || self.frozen.is_some() {
            return false;
        }
        let (columns, rows) = self.terminal.size();
        let grid = self.grid();
        if !clamp && !grid.contains(position, rows, columns) {
            return false;
        }
        let (row, column, _) = grid.hit(position, rows, columns);
        let mods = Modifiers { control: modifiers.control, alt: modifiers.alt, shift: modifiers.shift };
        if let Some(bytes) = self.terminal.mouse_input(event, (row, column), mods) {
            self.session.write_raw(bytes);
            return true;
        }
        false
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(button) = terminal_button(event.button) else { return };
        window.focus(&self.focus);
        if self.mouse_tracking() && !event.modifiers.shift {
            if self.send_mouse(TerminalMouse::Press(button), event.position, event.modifiers, false) {
                self.selection_drag = None;
                self.terminal.clear_selection();
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }
        if button == TerminalButton::Right {
            self.paste_on_right_click(cx);
            return;
        }
        if button != TerminalButton::Left {
            return;
        }
        let (row, column, side) = self.selection_cell(event.position);
        if !event.modifiers.shift || !self.terminal.extend_selection(row, column, side) {
            self.terminal.start_selection(row, column, side, event.click_count == 2);
        }
        self.selection_drag = Some(SelectionDrag::default());
        self.local_scroll = 0.;
        cx.notify();
    }

    fn extend_selection_at(&mut self, position: Point<Pixels>) {
        let (row, column, side) = self.selection_cell(position);
        self.terminal.extend_selection(row, column, side);
    }

    fn scroll_selection(&mut self, lines: i32, position: Point<Pixels>, cx: &mut Context<Self>) -> bool {
        let (row, column, side) = self.selection_cell(position);
        if self.terminal.scroll_selection(lines, row, column, side) {
            cx.notify();
            true
        } else {
            false
        }
    }

    fn start_selection_autoscroll(&mut self, position: Point<Pixels>, window: &Window, cx: &mut Context<Self>) {
        if self.grid().edge_scroll_lines(position.y, self.terminal.size().1) == 0 {
            return;
        }
        let Some(drag) = self.selection_drag.as_mut().filter(|drag| drag.autoscroll.is_none()) else { return };
        drag.autoscroll = Some(cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(32)).await;
                let scrolling = this.update_in(cx, |view, window, cx| {
                    let active = window.is_window_active() && view.focus.is_focused(window);
                    let position = window.mouse_position();
                    let lines = view.grid().edge_scroll_lines(position.y, view.terminal.size().1);
                    if active && lines != 0 && view.scroll_selection(lines, position, cx) {
                        return true;
                    }
                    if let Some(task) = view.selection_drag.as_mut().and_then(|drag| drag.autoscroll.take()) {
                        task.detach();
                    }
                    if !active {
                        view.selection_drag = None;
                    }
                    false
                });
                if !matches!(scrolling, Ok(true)) {
                    break;
                }
            }
        }));
    }

    fn on_mouse_drag(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_drag.is_some() {
            if event.pressed_button == Some(MouseButton::Left) {
                self.extend_selection_at(event.position);
                self.start_selection_autoscroll(event.position, window, cx);
                cx.notify();
                return;
            }
            self.selection_drag = None;
        }
        if let Some(button) = event.pressed_button.and_then(terminal_button) {
            if self.terminal.mouse_pressed(button) {
                self.send_mouse(TerminalMouse::Move(Some(button)), event.position, event.modifiers, true);
            }
        }
    }

    fn on_mouse_hover(&mut self, event: &MouseMoveEvent, _: &mut Window, _: &mut Context<Self>) {
        if event.pressed_button.is_none() && self.mouse_tracking() {
            self.send_mouse(TerminalMouse::Move(None), event.position, event.modifiers, false);
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(button) = terminal_button(event.button) {
            if self.terminal.mouse_pressed(button) {
                if self.send_mouse(TerminalMouse::Release(button), event.position, event.modifiers, true) {
                    cx.stop_propagation();
                }
                return;
            }
        }
        if event.button != MouseButton::Left || self.selection_drag.take().is_none() {
            return;
        }
        self.extend_selection_at(event.position);
        if let Some(text) = self.terminal.selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.copied = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_millis(1100)).await;
                let _ = this.update(cx, |view, cx| {
                    if let Some(task) = view.copied.take() {
                        task.detach();
                    }
                    cx.notify();
                });
            }));
        }
        cx.notify();
    }

    fn paste_on_right_click(&mut self, cx: &mut Context<Self>) {
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

    pub fn set_area(&mut self, width: f32, height: f32) {
        self.area = Some((width, height));
    }

    /// Input from another pane while broadcasting, as if typed here.
    pub fn write_input(&mut self, bytes: &[u8]) {
        if !self.closed && self.serial_form.is_none() && self.login.is_none() {
            self.terminal.scroll(i32::MIN / 2);
            if bytes.is_ascii() {
                self.session.write_raw(bytes.to_vec());
            } else {
                self.session.write(bytes);
            }
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
    }

    fn accept_suggestion(&mut self, index: usize, cx: &mut Context<Self>) {
        let a = &mut self.autocomplete;
        let (Some(typed), Some(suggestion)) = (a.line.text(), a.shown.get(index)) else { return };
        let rest = suggestion.rest(typed).to_owned();
        a.line.insert(&rest);
        let bytes = rest.into_bytes();
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
                .hover(|style| style.bg(rgb(termian_native::theme::SURFACE_ACTIVE)))
                .child(div().flex_1().min_w_0().truncate().child(div().flex().child(div().text_color(rgb(termian_native::theme::TEXT)).child(typed.to_owned())).child(div().text_color(rgb(termian_native::theme::TEXT_MUTED)).child(s.rest(typed).to_owned()))))
                .child(div().flex_none().text_xs().font_family(".SystemUIFont").text_color(rgb(termian_native::theme::TEXT_MUTED)).child(source))
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                    cx.stop_propagation();
                    view.accept_suggestion(i, cx);
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
            self.session.run_command(text);
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
        self.connected = None;
        self.generation += 1;
        self.session = session::ssh(connection.target, 80, 24);
        Self::listen(&mut self.session, self.generation, window, cx);
        self.selection_drag = None;
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

    fn wheel_delta(&self, event: &ScrollWheelEvent) -> Point<f32> {
        match event.delta {
            ScrollDelta::Lines(delta) => delta,
            ScrollDelta::Pixels(delta) => point(
                f32::from(delta.x) / self.cell_width.unwrap_or(8.),
                f32::from(delta.y) / self.line_height(),
            ),
        }
    }

    fn on_selection_wheel(&mut self, event: &ScrollWheelEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection_drag.is_none() || !window.is_window_active() || !self.focus.is_focused(window) {
            return;
        }
        let rows = self.local_wheel_rows(event, self.wheel_delta(event));
        if rows != 0 {
            self.scroll_selection(rows, event.position, cx);
        }
        cx.stop_propagation();
    }

    fn local_wheel_rows(&mut self, event: &ScrollWheelEvent, delta: Point<f32>) -> i32 {
        self.mouse_scroll = point(0., 0.);
        if matches!(event.touch_phase, gpui::TouchPhase::Started) {
            self.local_scroll = 0.;
        }
        self.local_scroll += if event.modifiers.shift && delta.y == 0. { delta.x } else { delta.y };
        let rows = self.local_scroll as i32;
        self.local_scroll -= rows as f32;
        rows
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = self.wheel_delta(event);
        if self.mouse_tracking() && !event.modifiers.shift {
            self.local_scroll = 0.;
            if matches!(event.touch_phase, gpui::TouchPhase::Started) {
                self.mouse_scroll = point(0., 0.);
            }
            self.mouse_scroll = self.mouse_scroll + delta;
            let (columns, rows) = (self.mouse_scroll.x as i32, self.mouse_scroll.y as i32);
            self.mouse_scroll = self.mouse_scroll - point(columns as f32, rows as f32);
            if columns != 0 || rows != 0 {
                self.send_mouse(TerminalMouse::Scroll { columns, rows }, event.position, event.modifiers, false);
            }
            cx.stop_propagation();
            return;
        }
        let rows = self.local_wheel_rows(event, delta);
        if rows != 0 && self.terminal.scroll(rows) {
            cx.notify();
        }
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
        let (width, height) = self.area.map_or((viewport.width, viewport.height - px(TOP_INSET)), |(w, h)| (px(w), px(h)));
        self.fit(width, height, window);
        let snapshot = self.frozen.clone().unwrap_or_else(|| self.terminal.snapshot());
        let focused = self.focus.is_focused(window);
        let cell_width = self.cell_width.unwrap_or(8.);
        let line_height = self.line_height();
        let ((family, weight), (bold_family, bold_weight)) = self.faces();
        let look = self.terminal.look().clone();

        let terminal_cursor = snapshot.cursor.filter(|_| !self.closed);
        let font_size = px(self.font_size());
        let regular = Font { weight: gpui::FontWeight(weight), ..font(family.clone()) };
        let bold = Font { weight: gpui::FontWeight(bold_weight), ..font(bold_family) };
        let grid_origin = self.grid_origin.clone();
        let selecting = self.selection_drag.is_some();
        let drag = cx.listener(Self::on_mouse_drag);
        let release = cx.listener(Self::on_mouse_up);
        let selection_wheel = cx.listener(Self::on_selection_wheel);
        let background = look.background;
        let cursor_color: gpui::Hsla = rgb(look.cursor).into();
        let grid = canvas(
            move |bounds, _, _| grid_origin.set(bounds.origin),
            move |bounds, (), window, _| {
                let grid = CellGrid { origin: bounds.origin, cell_width: px(cell_width), line_height: px(line_height) };
                for (row, runs) in snapshot.lines.iter().enumerate() {
                    let mut column = 0;
                    for run in runs {
                        let span = grid.span(row, column, run.columns);
                        if run.bg != background {
                            window.paint_quad(fill(span, rgb(run.bg)));
                        }
                        column += run.columns;
                    }
                    let mut column = 0;
                    for run in runs {
                        let face = if run.bold { &bold } else { &regular };
                        paint_run(run, grid.span(row, column, run.columns).origin, grid, font_size, face, window);
                        column += run.columns;
                    }
                }
                if let Some((row, column)) = terminal_cursor {
                    let bounds = grid.span(row, column, 1);
                    window.paint_quad(if focused {
                        fill(bounds, cursor_color.opacity(0.7))
                    } else {
                        outline(bounds, cursor_color, BorderStyle::default())
                    });
                }
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                    if phase == DispatchPhase::Capture && (selecting || event.pressed_button.is_some()) {
                        drag(event, window, cx);
                    }
                });
                window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                    if phase == DispatchPhase::Capture {
                        release(event, window, cx);
                    }
                });
                window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                    if phase == DispatchPhase::Capture {
                        selection_wheel(event, window, cx);
                    }
                });
            },
        ).size_full();

        div()
            .id("terminal")
            .track_focus(&self.focus)
            .key_context("Terminal")
            .on_action(cx.listener(|view, _: &CopyFromTerminal, _, cx| view.copy_selection(cx)))
            .on_action(cx.listener(|view, _: &PasteToTerminal, _, cx| view.paste_clipboard(cx)))
            .on_action(cx.listener(|view, _: &SelectAllInTerminal, _, cx| {
                view.selection_drag = None;
                view.terminal.select_all();
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &ClearTerminalBuffer, _, cx| {
                view.selection_drag = None;
                view.terminal.clear_buffer();
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &OpenTerminalSearch, window, cx| view.open_search(window, cx)))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_hover))
            .relative()
            .size_full()
            .p(px(PADDING))
            .overflow_hidden()
            .bg(rgb(look.background))
            .font_family(family)
            .font_weight(gpui::FontWeight(weight))
            .text_size(px(self.font_size()))
            .line_height(px(line_height))
            .child(grid)
            .children(self.render_autocomplete(terminal_cursor.filter(|_| focused), cell_width, line_height, cx))
            .children(self.render_search(window, cx))
            .when(self.copied.is_some(), |d| d.child(super::motion::toast(
                div().absolute().right(px(12.)).bottom(px(12.)).px_3().py_1()
                    .rounded_md().bg(rgb(termian_native::theme::SURFACE))
                    .text_size(px(12.)).text_color(rgb(termian_native::theme::TEXT))
                    .child(tr("Copied!")),
            )))
            .children(self.connected.as_ref().and_then(|progress| {
                let connection = self.connection.as_ref()?;
                Some(login_view::render_connected(&connection.label, format!("SSH {}:{}", connection.target.host, connection.target.port), connection.os.as_deref(), progress, window))
            }))
            .into_any_element()
    }
}

fn terminal_button(button: MouseButton) -> Option<TerminalButton> {
    match button {
        MouseButton::Left => Some(TerminalButton::Left),
        MouseButton::Middle => Some(TerminalButton::Middle),
        MouseButton::Right => Some(TerminalButton::Right),
        MouseButton::Navigate(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_uses_painted_rows_and_cell_halves() {
        let grid = CellGrid { origin: point(px(317.5), px(82.25)), cell_width: px(8.4), line_height: px(19.) };
        let span = grid.span(2, 41, 2);
        assert_eq!(span.size, size(px(16.8), px(19.)));
        assert_eq!(grid.hit(span.origin + point(px(3.), px(1.)), 8, 80), (2, 41, Side::Left));
        assert_eq!(grid.hit(span.origin + point(px(6.), px(18.)), 8, 80), (2, 41, Side::Right));
        assert_eq!(grid.hit(span.origin + point(px(10.), px(1.)), 8, 80), (2, 42, Side::Left));
        assert_eq!(grid.hit(point(px(200.), px(10.)), 8, 80), (0, 0, Side::Left));
        assert_eq!(grid.hit(point(px(1200.), px(500.)), 8, 80), (7, 79, Side::Right));
        assert!(grid.contains(span.origin + point(px(6.), px(18.)), 8, 80));
        assert!(!grid.contains(point(px(317.), px(83.)), 8, 80));
        assert!(!grid.contains(grid.span(8, 0, 1).origin, 8, 80));
    }

    #[test]
    fn selection_autoscroll_tracks_the_grid_edges() {
        let grid = CellGrid { origin: point(px(200.), px(80.)), cell_width: px(8.), line_height: px(20.) };
        assert_eq!(grid.edge_scroll_lines(px(85.), 10), 0);
        assert_eq!(grid.edge_scroll_lines(px(274.), 10), 0);
        assert_eq!(grid.edge_scroll_lines(px(81.), 10), 1);
        assert_eq!(grid.edge_scroll_lines(px(278.), 10), -1);
        assert_eq!(grid.edge_scroll_lines(px(64.), 10), 3);
        assert_eq!(grid.edge_scroll_lines(px(296.), 10), -3);
        assert_eq!(grid.edge_scroll_lines(px(-500.), 10), 24);
        assert_eq!(grid.edge_scroll_lines(px(800.), 10), -24);
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
