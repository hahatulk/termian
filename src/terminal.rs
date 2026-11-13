//! Terminal emulation (alacritty_terminal) reduced to what the UI draws:
//! feed bytes in, read styled text runs out, turn key presses into bytes.

use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Boundary, Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::search::{Match, RegexSearch};
use alacritty_terminal::term::{viewport_to_point, Config, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor};
use alacritty_terminal::Term;

use crate::terminal_settings::{self, TerminalSettings};

/// Colours and colour rules of a terminal, from Settings → Terminal: the
/// theme as legacy hands it to xterm (`Ale`), bright bold and highlighting.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub palette: [u32; 16],
    pub background: u32,
    pub foreground: u32,
    pub cursor: u32,
    /// The theme's selection colour laid over the background at its alpha.
    pub selection: u32,
    pub brighten_bold: bool,
    /// Keyword highlighting colours, when it is on.
    pub highlights: Option<[u32; 6]>,
}

impl Look {
    pub fn new(settings: &TerminalSettings) -> Self {
        let theme = settings.theme();
        Self {
            palette: theme.palette,
            background: theme.background,
            foreground: theme.foreground,
            cursor: theme.cursor,
            selection: blend(theme.selection, theme.background, theme.selection_alpha),
            brighten_bold: settings.brighten_bold,
            highlights: settings.keyword_highlighting.then(|| terminal_settings::highlight_colors(&theme.name)),
        }
    }
}

impl Default for Look {
    fn default() -> Self {
        Self::new(&TerminalSettings::default())
    }
}

/// `color` at `alpha` over `under`.
fn blend(color: u32, under: u32, alpha: u8) -> u32 {
    let mix = |shift: u32| {
        let (c, u) = ((color >> shift) & 0xff, (under >> shift) & 0xff);
        ((c * alpha as u32 + u * (255 - alpha as u32)) / 255) << shift
    };
    mix(16) | mix(8) | mix(0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: String,
    pub fg: u32,
    pub bg: u32,
    pub bold: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub lines: Vec<Vec<Run>>,
    /// Row and column of the cursor, when it is inside the viewport.
    pub cursor: Option<(usize, usize)>,
}

impl Snapshot {
    pub fn line_text(&self, row: usize) -> String {
        self.lines[row].iter().map(|run| run.text.as_str()).collect()
    }
}

#[derive(Clone, Copy)]
struct Size {
    columns: usize,
    lines: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn columns(&self) -> usize {
        self.columns
    }
}

/// Collects what the emulator wants to say back to the program (device
/// status replies and the like) and title changes.
#[derive(Clone, Default)]
struct Listener(Arc<Mutex<Outbox>>);

#[derive(Default)]
struct Outbox {
    replies: Vec<u8>,
    title: Option<String>,
    bell: bool,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let mut outbox = self.0.lock().unwrap();
        match event {
            Event::PtyWrite(text) => outbox.replies.extend(text.into_bytes()),
            Event::Title(title) => outbox.title = Some(title),
            Event::Bell => outbox.bell = true,
            _ => {}
        }
    }
}

pub struct Terminal {
    term: Term<Listener>,
    parser: Processor,
    listener: Listener,
    look: Look,
    /// «Search terminal»: the query's compiled search and the current match.
    search: Option<(String, RegexSearch)>,
    found: Option<Match>,
}

/// Regex metacharacters escaped, so the query is matched literally.
fn literal(query: &str) -> String {
    let mut out = String::new();
    for c in query.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl Terminal {
    pub fn new(columns: usize, lines: usize) -> Self {
        Self::with_settings(columns, lines, &TerminalSettings::default())
    }

    pub fn with_settings(columns: usize, lines: usize, settings: &TerminalSettings) -> Self {
        let listener = Listener::default();
        let config = Config { scrolling_history: settings.scrollback_lines(), ..Config::default() };
        let size = Size { columns: columns.max(2), lines: lines.max(1) };
        Self { term: Term::new(config, &size, listener.clone()), parser: Processor::new(), listener, look: Look::new(settings), search: None, found: None }
    }

    /// New Settings → Terminal values for an open terminal.
    pub fn apply(&mut self, settings: &TerminalSettings) {
        self.look = Look::new(settings);
        self.term.set_options(Config { scrolling_history: settings.scrollback_lines(), ..Config::default() });
    }

    pub fn look(&self) -> &Look {
        &self.look
    }

    /// Whether the program rang the bell since the last call.
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.listener.0.lock().unwrap().bell)
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn resize(&mut self, columns: usize, lines: usize) {
        self.term.resize(Size { columns: columns.max(2), lines: lines.max(1) });
    }

    pub fn size(&self) -> (usize, usize) {
        (self.term.columns(), self.term.screen_lines())
    }

    /// Bytes the emulator must send back to the program.
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.listener.0.lock().unwrap().replies)
    }

    pub fn title(&self) -> Option<String> {
        self.listener.0.lock().unwrap().title.clone()
    }

    /// Scrolls the viewport through history; positive values go back in time.
    pub fn scroll(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn app_cursor(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }

    pub fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    /// Starts a selection at a viewport cell; `word` selects whole words (double-click).
    pub fn start_selection(&mut self, row: usize, column: usize, word: bool) {
        let point = self.viewport_point(row, column);
        let kind = if word { SelectionType::Semantic } else { SelectionType::Simple };
        self.term.selection = Some(Selection::new(kind, point, Side::Left));
    }

    pub fn extend_selection(&mut self, row: usize, column: usize) {
        let point = self.viewport_point(row, column);
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(point, Side::Right);
        }
    }

    /// «Select All in Terminal»: history and screen.
    pub fn select_all(&mut self) {
        let start = Point::new(alacritty_terminal::index::Line(-(self.term.history_size() as i32)), Column(0));
        let end = Point::new(alacritty_terminal::index::Line(self.term.screen_lines() as i32 - 1), Column(self.term.columns() - 1));
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        self.term.selection = Some(selection);
    }

    /// «Clear Terminal Buffer», as xterm's `clear()`: the cursor line moves to
    /// the top and everything else, history included, goes.
    pub fn clear_buffer(&mut self) {
        let row = self.term.grid().cursor.point.line.0.max(0);
        self.term.scroll_display(Scroll::Bottom);
        if row > 0 {
            self.feed(format!("\x1b[{row}S\x1b[{row}A").as_bytes());
        }
        self.feed(b"\x1b[3J");
        self.term.selection = None;
    }

    /// xterm's search addon as legacy uses it: case-insensitive literal text,
    /// `previous` looks up from the last match (or the bottom), otherwise
    /// down; it wraps around. `incremental` (typing) keeps the current match
    /// when it still fits. The match is selected and scrolled into view.
    pub fn find(&mut self, query: &str, previous: bool, incremental: bool) -> bool {
        if query.is_empty() {
            self.clear_search();
            return false;
        }
        if self.search.as_ref().is_none_or(|(q, _)| q != query) {
            let Ok(regex) = RegexSearch::new(&literal(&query.to_lowercase())) else { return false };
            self.search = Some((query.to_owned(), regex));
        }
        let top = Point::new(Line(-(self.term.history_size() as i32)), Column(0));
        let bottom = Point::new(Line(self.term.screen_lines() as i32 - 1), Column(self.term.columns() - 1));
        // Typing keeps the match where it is while the longer query still fits there.
        if let (true, Some(found)) = (incremental, self.found.clone()) {
            let regex = &mut self.search.as_mut().expect("compiled above").1;
            if let Some(kept) = self.term.search_next(regex, *found.start(), Direction::Right, Side::Left, Some(0)).filter(|m| m.start() == found.start()) {
                self.found = Some(kept);
                return self.select_found();
            }
        }
        let (origin, direction, side) = match (&self.found, previous, incremental) {
            (Some(found), true, true) => (found.start().sub(&self.term, Boundary::None, 1), Direction::Left, Side::Right),
            (Some(found), true, false) => (found.start().sub(&self.term, Boundary::None, 1), Direction::Left, Side::Right),
            (None, true, _) => (bottom, Direction::Left, Side::Right),
            (Some(found), false, _) => (found.end().add(&self.term, Boundary::None, 1), Direction::Right, Side::Left),
            (None, false, _) => (top, Direction::Right, Side::Left),
        };
        let regex = &mut self.search.as_mut().expect("compiled above").1;
        self.found = self.term.search_next(regex, origin, direction, side, None);
        self.select_found()
    }

    fn select_found(&mut self) -> bool {
        match self.found.clone() {
            Some(found) => {
                let mut selection = Selection::new(SelectionType::Simple, *found.start(), Side::Left);
                selection.update(*found.end(), Side::Right);
                self.term.selection = Some(selection);
                self.term.scroll_to_point(*found.start());
                true
            }
            None => {
                self.term.selection = None;
                false
            }
        }
    }

    pub fn clear_search(&mut self) {
        self.search = None;
        self.found = None;
    }

    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    /// Selected text; `None` when nothing (or only an empty cell range) is selected.
    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|text| !text.is_empty())
    }

    fn viewport_point(&self, row: usize, column: usize) -> Point {
        let (columns, rows) = self.size();
        let point = Point::new(row.min(rows - 1), Column(column.min(columns - 1)));
        viewport_to_point(self.term.grid().display_offset(), point)
    }

    pub fn snapshot(&self) -> Snapshot {
        let content = self.term.renderable_content();
        let selection = content.selection;
        let offset = content.display_offset as i32;
        let (columns, rows) = self.size();
        let look = &self.look;
        // Cells first, so highlighting can recolour by character position.
        let mut cells: Vec<Vec<(char, u32, u32, bool)>> = vec![Vec::new(); rows];
        for indexed in content.display_iter {
            let row = indexed.point.line.0 + offset;
            if row < 0 || row as usize >= rows || indexed.point.column.0 >= columns {
                continue;
            }
            let cell = indexed.cell;
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let bold = cell.flags.contains(Flags::BOLD);
            let fg_color = if bold && look.brighten_bold { brighter(cell.fg) } else { cell.fg };
            let (mut fg, mut bg) = (color(fg_color, look, look.foreground), color(cell.bg, look, look.background));
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if selection.is_some_and(|range| range.contains(indexed.point)) {
                bg = look.selection;
            }
            let ch = if cell.flags.contains(Flags::HIDDEN) { ' ' } else { cell.c };
            cells[row as usize].push((ch, fg, bg, bold));
        }
        let mut lines: Vec<Vec<Run>> = vec![Vec::new(); rows];
        for (row, mut cells) in cells.into_iter().enumerate() {
            if let Some(colors) = &look.highlights {
                let text: String = cells.iter().map(|c| c.0).collect();
                for (start, end, color) in terminal_settings::highlights(&text, colors) {
                    for cell in cells.iter_mut().take(end).skip(start) {
                        cell.1 = color;
                    }
                }
            }
            let line = &mut lines[row];
            for (ch, fg, bg, bold) in cells {
                match line.last_mut() {
                    Some(run) if run.fg == fg && run.bg == bg && run.bold == bold => run.text.push(ch),
                    _ => line.push(Run { text: ch.to_string(), fg, bg, bold }),
                }
            }
        }
        let point = content.cursor.point;
        let cursor_row = point.line.0 + offset;
        let cursor = (cursor_row >= 0 && (cursor_row as usize) < rows).then_some((cursor_row as usize, point.column.0));
        Snapshot { lines, cursor }
    }
}

/// Bold text in one of the eight basic colours takes its bright variant
/// («Use bright colours for bold text»).
fn brighter(color: Color) -> Color {
    match color {
        Color::Indexed(index @ 0..=7) => Color::Indexed(index + 8),
        Color::Named(named) if (named as usize) < 8 => Color::Indexed(named as u8 + 8),
        Color::Named(NamedColor::Foreground) => Color::Indexed(15),
        other => other,
    }
}

fn color(color: Color, look: &Look, default: u32) -> u32 {
    match color {
        Color::Spec(rgb) => (rgb.r as u32) << 16 | (rgb.g as u32) << 8 | rgb.b as u32,
        Color::Indexed(index) => indexed_color(index, &look.palette),
        Color::Named(named) => match named as usize {
            index @ 0..=15 => look.palette[index],
            _ if named == NamedColor::Background => look.background,
            _ if named == NamedColor::Foreground => look.foreground,
            _ => default,
        },
    }
}

/// xterm 256-colour palette.
fn indexed_color(index: u8, palette: &[u32; 16]) -> u32 {
    match index {
        0..=15 => palette[index as usize],
        16..=231 => {
            let i = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v as u32 };
            level(i / 36) << 16 | level(i / 6 % 6) << 8 | level(i % 6)
        }
        _ => {
            let gray = 8 + 10 * (index - 232) as u32;
            gray << 16 | gray << 8 | gray
        }
    }
}

/// A key press as the UI reports it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Bytes to send for a key; `key` uses GPUI key names (`enter`, `up`, `a`).
/// The QWERTY key under a ЙЦУКЕН letter, so control shortcuts work in any layout.
fn latin_key(c: char) -> char {
    const RU: &str = "йцукенгшщзхъфывапролджэячсмитьбю";
    const EN: &str = "qwertyuiop[]asdfghjkl;'zxcvbnm,.";
    let lower = c.to_lowercase().next().unwrap_or(c);
    match RU.chars().position(|r| r == lower) {
        Some(at) => EN.chars().nth(at).unwrap_or(lower),
        None => lower.to_ascii_lowercase(),
    }
}

pub fn key_bytes(key: &str, typed: Option<&str>, mods: Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    let arrow = |code: u8| {
        if mods.shift || mods.control || mods.alt {
            let m = 1 + mods.shift as u8 + 2 * mods.alt as u8 + 4 * mods.control as u8;
            format!("\x1b[1;{m}{}", code as char).into_bytes()
        } else if app_cursor {
            vec![0x1b, b'O', code]
        } else {
            vec![0x1b, b'[', code]
        }
    };
    let bytes = match key {
        "enter" => vec![b'\r'],
        "backspace" => vec![0x7f],
        "tab" if mods.shift => b"\x1b[Z".to_vec(),
        "tab" => vec![b'\t'],
        "escape" => vec![0x1b],
        "up" => arrow(b'A'),
        "down" => arrow(b'B'),
        "right" => arrow(b'C'),
        "left" => arrow(b'D'),
        "home" => b"\x1b[H".to_vec(),
        "end" => b"\x1b[F".to_vec(),
        "pageup" => b"\x1b[5~".to_vec(),
        "pagedown" => b"\x1b[6~".to_vec(),
        "delete" => b"\x1b[3~".to_vec(),
        "space" if mods.control => vec![0],
        _ if mods.control && key.chars().count() == 1 => {
            // With a Russian layout Ctrl+C arrives as Ctrl+С: use the Latin key in that place.
            let c = latin_key(key.chars().next().unwrap_or(' ')) as u8;
            match c {
                b'a'..=b'z' => vec![c - b'a' + 1],
                b'[' => vec![0x1b],
                b'\\' => vec![0x1c],
                b']' => vec![0x1d],
                _ => return None,
            }
        }
        _ => typed.filter(|t| !t.is_empty())?.as_bytes().to_vec(),
    };
    Some(if mods.alt && !matches!(key, "up" | "down" | "left" | "right") {
        [vec![0x1b], bytes].concat()
    } else {
        bytes
    })
}

/// Wraps pasted text in bracketed-paste markers when the program asked for them.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if bracketed {
        format!("\x1b[200~{}\x1b[201~", text.replace("\x1b[201~", "")).into_bytes()
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_text_and_moves_cursor() {
        let mut t = Terminal::new(20, 3);
        t.feed(b"hello\r\nworld");
        let s = t.snapshot();
        assert_eq!(s.line_text(0).trim_end(), "hello");
        assert_eq!(s.line_text(1).trim_end(), "world");
        assert_eq!(s.cursor, Some((1, 5)));
    }

    #[test]
    fn colours_split_runs() {
        let mut t = Terminal::new(10, 1);
        t.feed(b"a\x1b[31mb\x1b[1;32mc\x1b[0m");
        let runs = &t.snapshot().lines[0];
        let look = Look::default();
        assert_eq!((runs[0].text.as_str(), runs[0].fg), ("a", look.foreground));
        assert_eq!((runs[1].text.as_str(), runs[1].fg), ("b", look.palette[1]));
        assert_eq!((runs[2].text.as_str(), runs[2].fg, runs[2].bold), ("c", look.palette[2], true));
    }

    #[test]
    fn settings_change_theme_bold_highlighting_and_bell() {
        let mut settings = TerminalSettings { color_scheme: "termius light".into(), brighten_bold: true, ..TerminalSettings::default() };
        let mut t = Terminal::with_settings(30, 1, &settings);
        t.feed(b"\x1b[1;31mx\x1b[0m [error] 10.0.0.1\x07");
        let light = terminal_settings::theme("termius light");
        let runs = t.snapshot().lines[0].clone();
        assert_eq!((runs[0].text.as_str(), runs[0].fg), ("x", light.palette[9]), "bold red is bright red");
        let colors = terminal_settings::highlight_colors("termius light");
        assert!(runs.iter().any(|r| r.text == "[error]" && r.fg == colors[0]), "{runs:?}");
        assert!(runs.iter().any(|r| r.text == "10.0.0.1" && r.fg == colors[5]), "{runs:?}");
        assert!(t.take_bell() && !t.take_bell());
        settings.keyword_highlighting = false;
        settings.brighten_bold = false;
        t.apply(&settings);
        let runs = t.snapshot().lines[0].clone();
        assert_eq!(runs[0].fg, light.palette[1]);
        assert!(runs.iter().all(|r| r.fg != colors[0]));
        assert_eq!(t.look().background, light.background);
        assert_eq!(blend(0xffffff, 0x000000, 0x80), 0x808080);
    }

    #[test]
    fn truecolor_and_256_colours() {
        let mut t = Terminal::new(4, 1);
        t.feed(b"\x1b[38;2;1;2;3mx\x1b[38;5;196my");
        let runs = &t.snapshot().lines[0];
        assert_eq!(runs[0].fg, 0x010203);
        assert_eq!(runs[1].fg, 0xff0000);
    }

    #[test]
    fn answers_device_status_report() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"\x1b[6n");
        assert_eq!(t.take_replies(), b"\x1b[1;1R");
        assert!(t.take_replies().is_empty());
    }

    #[test]
    fn tracks_title_and_modes() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"\x1b]0;root@termian\x07\x1b[?1h\x1b[?2004h");
        assert_eq!(t.title().as_deref(), Some("root@termian"));
        assert!(t.app_cursor());
        assert!(t.bracketed_paste());
    }

    #[test]
    fn scrollback_keeps_history() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"1\r\n2\r\n3\r\n4");
        assert_eq!(t.snapshot().line_text(0).trim_end(), "3");
        t.scroll(2);
        assert_eq!(t.snapshot().line_text(0).trim_end(), "1");
        assert_eq!(t.snapshot().cursor, None);
    }

    #[test]
    fn resize_changes_grid() {
        let mut t = Terminal::new(10, 2);
        t.resize(30, 5);
        assert_eq!(t.size(), (30, 5));
        assert_eq!(t.snapshot().lines.len(), 5);
    }

    #[test]
    fn selects_and_copies_text() {
        let mut t = Terminal::new(20, 3);
        t.feed(b"hello world\r\nsecond line");
        t.start_selection(0, 6, false);
        t.extend_selection(1, 5);
        assert_eq!(t.selection_text().as_deref(), Some("world\nsecond"));
        let runs = &t.snapshot().lines[0];
        let selection = Look::default().selection;
        let highlighted = runs.iter().find(|r| r.bg == selection).unwrap();
        assert!(highlighted.text.starts_with("world"), "{highlighted:?}");
        assert!(runs[0].text.starts_with("hello") && runs[0].bg != selection);
        t.clear_selection();
        assert_eq!(t.selection_text(), None);
    }

    #[test]
    fn select_all_and_clear_buffer() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"one\r\ntwo\r\nthree\r\n$ ");
        t.select_all();
        assert_eq!(t.selection_text().as_deref().map(str::trim_end), Some("one\ntwo\nthree\n$"));
        t.clear_buffer();
        let s = t.snapshot();
        assert_eq!((s.line_text(0).trim_end(), s.line_text(1).trim_end()), ("$", ""));
        assert_eq!(s.cursor, Some((0, 2)));
        t.scroll(5);
        assert_eq!(t.snapshot().line_text(0).trim_end(), "$", "history is gone");
    }

    #[test]
    fn double_click_selects_a_word() {
        let mut t = Terminal::new(20, 1);
        t.feed(b"cd /opt/docker now");
        t.start_selection(0, 5, true);
        t.extend_selection(0, 5);
        assert_eq!(t.selection_text().as_deref(), Some("/opt/docker"));
    }

    #[test]
    fn selection_follows_scrollback() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"one\r\ntwo\r\nthree");
        t.scroll(1); // viewport now shows "one" and "two"
        t.start_selection(1, 0, false);
        t.extend_selection(1, 2);
        assert_eq!(t.selection_text().as_deref(), Some("two"));
    }

    #[test]
    fn maps_keys_to_bytes() {
        let none = Modifiers::default();
        let ctrl = Modifiers { control: true, ..none };
        let alt = Modifiers { alt: true, ..none };
        assert_eq!(key_bytes("enter", None, none, false).unwrap(), b"\r");
        assert_eq!(key_bytes("up", None, none, false).unwrap(), b"\x1b[A");
        assert_eq!(key_bytes("up", None, none, true).unwrap(), b"\x1bOA");
        assert_eq!(key_bytes("c", Some("c"), ctrl, false).unwrap(), [3]);
        assert_eq!(key_bytes("b", Some("b"), alt, false).unwrap(), b"\x1bb");
        assert_eq!(key_bytes("left", None, ctrl, false).unwrap(), b"\x1b[1;5D");
        assert_eq!(key_bytes("ж", Some("ж"), none, false).unwrap(), "ж".as_bytes());
        // Russian layout: Ctrl+С, Ctrl+В, Ctrl+Л are Ctrl+C, Ctrl+D, Ctrl+K.
        assert_eq!(key_bytes("с", Some("с"), ctrl, false).unwrap(), [3]);
        assert_eq!(key_bytes("В", None, ctrl, false).unwrap(), [4]);
        assert_eq!(key_bytes("л", None, ctrl, false).unwrap(), [11]);
        assert_eq!(key_bytes("х", None, ctrl, false).unwrap(), [0x1b], "Ctrl+[");
        assert_eq!(key_bytes("f13", None, none, false), None);
    }

    #[test]
    fn paste_normalises_newlines_and_brackets() {
        assert_eq!(paste_bytes("a\nb", false), b"a\rb");
        assert_eq!(paste_bytes("x", true), b"\x1b[200~x\x1b[201~");
    }

    #[test]
    fn search_finds_text_upwards_and_wraps() {
        let mut t = Terminal::new(20, 3);
        t.feed(b"alpha ERR one\r\nbeta\r\nerr two\r\nlast line (x)");
        assert!(t.find("err", true, true));
        assert_eq!(t.selection_text().as_deref(), Some("err"), "the bottom-most match first");
        assert!(t.find("err", true, false));
        assert_eq!(t.selection_text().as_deref(), Some("ERR"), "case-insensitive, from history");
        assert!(t.find("err", true, false));
        assert_eq!(t.selection_text().as_deref(), Some("err"), "wraps around");
        assert!(t.find("err", false, false));
        assert_eq!(t.selection_text().as_deref(), Some("ERR"), "next goes down and wraps too");
        t.clear_search();
        assert!(t.find("e", true, true));
        assert!(t.find("er", true, true));
        assert_eq!(t.selection_text().as_deref(), Some("er"));
        assert!(t.find("err", true, true));
        assert_eq!(t.selection_text().as_deref(), Some("err"), "typing stays on the bottom match");
        assert!(t.find("(x)", true, true), "regex characters are literal");
        assert!(!t.find("missing", true, true));
        assert_eq!(t.selection_text(), None);
    }
}