//! Terminal emulation (alacritty_terminal) reduced to what the UI draws:
//! feed bytes in, read styled text runs out, turn key presses into bytes.

use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{viewport_to_point, Config, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor};
use alacritty_terminal::Term;

/// ANSI palette of the legacy "Termius Dark" theme; background and
/// foreground are sampled from the legacy terminal screenshots.
pub const PALETTE: [u32; 16] = [
    0x343851, 0xf24e50, 0x008463, 0xeca855, 0x08639f, 0xc13282, 0x2091f6, 0xe2e3e8, //
    0x8d91a5, 0xff7375, 0x3ed7be, 0xfdc47d, 0x6ba0c3, 0xff7dc5, 0x44a7ff, 0xffffff,
];
pub const BACKGROUND: u32 = 0x0d1b2e;
pub const FOREGROUND: u32 = 0xe2e3e8;
pub const CURSOR: u32 = 0x21b568;
/// Selection highlight (accent at half strength over the background).
pub const SELECTION: u32 = 0x175a4a;

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
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let mut outbox = self.0.lock().unwrap();
        match event {
            Event::PtyWrite(text) => outbox.replies.extend(text.into_bytes()),
            Event::Title(title) => outbox.title = Some(title),
            _ => {}
        }
    }
}

pub struct Terminal {
    term: Term<Listener>,
    parser: Processor,
    listener: Listener,
}

impl Terminal {
    pub fn new(columns: usize, lines: usize) -> Self {
        let listener = Listener::default();
        let config = Config { scrolling_history: 10_000, ..Config::default() };
        let size = Size { columns: columns.max(2), lines: lines.max(1) };
        Self { term: Term::new(config, &size, listener.clone()), parser: Processor::new(), listener }
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
        let mut lines: Vec<Vec<Run>> = vec![Vec::new(); rows];
        for indexed in content.display_iter {
            let row = indexed.point.line.0 + offset;
            if row < 0 || row as usize >= rows || indexed.point.column.0 >= columns {
                continue;
            }
            let cell = indexed.cell;
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }
            let (mut fg, mut bg) = (color(cell.fg, FOREGROUND), color(cell.bg, BACKGROUND));
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            if selection.is_some_and(|range| range.contains(indexed.point)) {
                bg = SELECTION;
            }
            let bold = cell.flags.contains(Flags::BOLD);
            let ch = if cell.flags.contains(Flags::HIDDEN) { ' ' } else { cell.c };
            let line = &mut lines[row as usize];
            match line.last_mut() {
                Some(run) if run.fg == fg && run.bg == bg && run.bold == bold => run.text.push(ch),
                _ => line.push(Run { text: ch.to_string(), fg, bg, bold }),
            }
        }
        let point = content.cursor.point;
        let cursor_row = point.line.0 + offset;
        let cursor = (cursor_row >= 0 && (cursor_row as usize) < rows).then_some((cursor_row as usize, point.column.0));
        Snapshot { lines, cursor }
    }
}

fn color(color: Color, default: u32) -> u32 {
    match color {
        Color::Spec(rgb) => (rgb.r as u32) << 16 | (rgb.g as u32) << 8 | rgb.b as u32,
        Color::Indexed(index) => indexed_color(index),
        Color::Named(named) => match named as usize {
            index @ 0..=15 => PALETTE[index],
            _ if named == NamedColor::Background => BACKGROUND,
            _ if named == NamedColor::Foreground => FOREGROUND,
            _ => default,
        },
    }
}

/// xterm 256-colour palette.
fn indexed_color(index: u8) -> u32 {
    match index {
        0..=15 => PALETTE[index as usize],
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
        assert_eq!((runs[0].text.as_str(), runs[0].fg), ("a", FOREGROUND));
        assert_eq!((runs[1].text.as_str(), runs[1].fg), ("b", PALETTE[1]));
        assert_eq!((runs[2].text.as_str(), runs[2].fg, runs[2].bold), ("c", PALETTE[2], true));
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
        let highlighted = runs.iter().find(|r| r.bg == SELECTION).unwrap();
        assert!(highlighted.text.starts_with("world"), "{highlighted:?}");
        assert!(runs[0].text.starts_with("hello") && runs[0].bg != SELECTION);
        t.clear_selection();
        assert_eq!(t.selection_text(), None);
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
}
