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

mod keyboard;
pub mod mouse;

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
pub fn blend(color: u32, under: u32, alpha: u8) -> u32 {
    let mix = |shift: u32| {
        let (c, u) = ((color >> shift) & 0xff, (under >> shift) & 0xff);
        ((c * alpha as u32 + u * (255 - alpha as u32)) / 255) << shift
    };
    mix(16) | mix(8) | mix(0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub text: String,
    pub columns: usize,
    pub cell_columns: usize,
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
    mouse: mouse::State,
    pointer_selection: bool,
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
        Self { term: Term::new(config, &size, listener.clone()), parser: Processor::new(), listener, look: Look::new(settings), mouse: mouse::State::default(), pointer_selection: false, search: None, found: None }
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
        let selection = if self.pointer_selection {
            self.term.selection.clone().map(|selection| (selection, self.selection_space()))
        } else {
            None
        };
        self.parser.advance(&mut self.term, bytes);
        if let Some((selection, space)) = selection {
            if self.term.selection.is_none() && space == self.selection_space() {
                self.term.selection = Some(selection);
            }
        }
        if !self.term.mode().intersects(TermMode::MOUSE_MODE) {
            self.mouse = mouse::State::default();
        }
    }

    fn selection_space(&self) -> (bool, usize, usize, *const alacritty_terminal::term::cell::Cell) {
        let grid = self.term.grid();
        let live_row = self.term.bottommost_line();
        (
            self.alt_screen(),
            grid.history_size(),
            grid.display_offset(),
            std::ptr::from_ref(&grid[live_row][Column(0)]),
        )
    }

    pub fn resize(&mut self, columns: usize, lines: usize) {
        self.term.resize(Size { columns: columns.max(2), lines: lines.max(1) });
        self.mouse.cell = None;
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
    pub fn scroll(&mut self, lines: i32) -> bool {
        let offset = self.term.grid().display_offset() as i32;
        let lines = lines.clamp(-offset, self.term.history_size() as i32 - offset);
        if lines == 0 {
            return false;
        }
        self.term.scroll_display(Scroll::Delta(lines));
        true
    }

    pub fn app_cursor(&self) -> bool {
        self.term.mode().contains(TermMode::APP_CURSOR)
    }

    /// The program switched to the alternate screen (`?1049h`).
    pub fn alt_screen(&self) -> bool {
        self.term.mode().contains(TermMode::ALT_SCREEN)
    }

    pub fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
    }

    /// Starts a selection at a viewport cell; `word` selects whole words (double-click).
    pub fn start_selection(&mut self, row: usize, column: usize, side: Side, word: bool) {
        let (point, side) = self.selection_edge(row, column, side);
        let kind = if word { SelectionType::Semantic } else { SelectionType::Simple };
        self.term.selection = Some(Selection::new(kind, point, side));
        self.pointer_selection = true;
    }

    pub fn extend_selection(&mut self, row: usize, column: usize, side: Side) -> bool {
        if !self.pointer_selection {
            return false;
        }
        let (point, side) = self.selection_edge(row, column, side);
        let Some(selection) = self.term.selection.as_mut() else { return false };
        selection.update(point, side);
        true
    }

    pub fn scroll_selection(&mut self, lines: i32, row: usize, column: usize, side: Side) -> bool {
        if !self.scroll(lines) {
            return false;
        }
        self.extend_selection(row, column, side);
        true
    }

    fn selection_edge(&self, row: usize, column: usize, side: Side) -> (Point, Side) {
        let point = self.viewport_point(row, column);
        let flags = self.term.grid()[point].flags;
        if flags.contains(Flags::WIDE_CHAR) && point.column.0 + 1 < self.term.columns() {
            (point, Side::Left)
        } else if flags.contains(Flags::WIDE_CHAR_SPACER) {
            (point, Side::Right)
        } else {
            (point, side)
        }
    }

    /// «Select All in Terminal»: history and screen.
    pub fn select_all(&mut self) {
        self.pointer_selection = false;
        let start = Point::new(alacritty_terminal::index::Line(-(self.term.history_size() as i32)), Column(0));
        let end = Point::new(alacritty_terminal::index::Line(self.term.screen_lines() as i32 - 1), Column(self.term.columns() - 1));
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        self.term.selection = Some(selection);
    }

    /// «Clear Terminal Buffer», as xterm's `clear()`: the cursor line moves to
    /// the top and everything else, history included, goes.
    pub fn clear_buffer(&mut self) {
        self.clear_selection();
        let row = self.term.grid().cursor.point.line.0.max(0);
        self.term.scroll_display(Scroll::Bottom);
        if row > 0 {
            self.feed(format!("\x1b[{row}S\x1b[{row}A").as_bytes());
        }
        self.feed(b"\x1b[3J");
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
        self.pointer_selection = false;
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
        self.pointer_selection = false;
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
        let mut cells: Vec<Vec<(char, u32, u32, bool, usize)>> = vec![Vec::new(); rows];
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
            cells[row as usize].push((ch, fg, bg, bold, if cell.flags.contains(Flags::WIDE_CHAR) { 2 } else { 1 }));
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
            for (ch, fg, bg, bold, width) in cells {
                match line.last_mut() {
                    Some(run) if run.fg == fg && run.bg == bg && run.bold == bold && run.cell_columns == width => {
                        run.text.push(ch);
                        run.columns += width;
                    }
                    _ => line.push(Run { text: ch.to_string(), columns: width, cell_columns: width, fg, bg, bold }),
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
        t.start_selection(0, 6, Side::Left, false);
        t.extend_selection(1, 5, Side::Right);
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
    fn drag_uses_cell_edges_in_both_directions() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"abcd");
        t.start_selection(0, 0, Side::Right, false);
        t.extend_selection(0, 1, Side::Left);
        assert_eq!(t.selection_text(), None);
        t.extend_selection(0, 3, Side::Left);
        assert_eq!(t.selection_text().as_deref(), Some("bc"));

        t.start_selection(0, 3, Side::Left, false);
        t.extend_selection(0, 0, Side::Right);
        assert_eq!(t.selection_text().as_deref(), Some("bc"));
    }

    #[test]
    fn wide_character_is_selected_as_one_unit() {
        let mut t = Terminal::new(10, 2);
        t.feed("a界b".as_bytes());
        t.start_selection(0, 1, Side::Right, false);
        t.extend_selection(0, 2, Side::Left);
        assert_eq!(t.selection_text().as_deref(), Some("界"));
        let snapshot = t.snapshot();
        let selected = snapshot.lines[0].iter().find(|run| run.bg == Look::default().selection).unwrap();
        assert_eq!((selected.text.as_str(), selected.columns, selected.cell_columns), ("界", 2, 2));

        t.start_selection(0, 2, Side::Left, false);
        t.extend_selection(0, 3, Side::Right);
        assert_eq!(t.selection_text().as_deref(), Some("b"));
    }

    #[test]
    fn double_click_selects_a_word() {
        let mut t = Terminal::new(20, 1);
        t.feed(b"cd /opt/docker now");
        t.start_selection(0, 5, Side::Left, true);
        assert_eq!(t.selection_text().as_deref(), Some("/opt/docker"));
    }

    #[test]
    fn selection_follows_scrollback() {
        let mut t = Terminal::new(10, 2);
        t.feed(b"one\r\ntwo\r\nthree");
        t.scroll(1); // viewport now shows "one" and "two"
        t.start_selection(1, 0, Side::Left, false);
        t.extend_selection(1, 2, Side::Right);
        assert_eq!(t.selection_text().as_deref(), Some("two"));
    }

    #[test]
    fn drag_selection_reaches_the_oldest_line_and_can_shrink() {
        let mut t = Terminal::new(12, 4);
        let history = (0..300).map(|i| format!("line {i:03}")).collect::<Vec<_>>();
        t.feed(history.join("\r\n").as_bytes());
        t.start_selection(3, 7, Side::Right, false);

        for _ in (0..history.len() - t.size().1).step_by(11) {
            assert!(t.scroll_selection(11, 0, 0, Side::Left));
        }
        assert!(!t.scroll_selection(11, 0, 0, Side::Left));
        assert_eq!(t.selection_text().as_deref(), Some(history.join("\n").as_str()));
        let snapshot = t.snapshot();
        assert_eq!(snapshot.lines[0][0].bg, t.look().selection);

        assert!(t.scroll_selection(-1, 0, 5, Side::Left));
        let expected = format!("001\n{}", history[2..].join("\n"));
        assert_eq!(t.selection_text().as_deref(), Some(expected.as_str()));
        assert!(t.scroll_selection(i32::MIN, 3, 5, Side::Left));
        assert_eq!(t.selection_text().as_deref(), Some("299"));
        assert!(!t.scroll_selection(-1, 3, 0, Side::Left));
    }

    #[test]
    fn redrawing_the_live_row_keeps_the_selection_anchor() {
        let mut t = Terminal::new(12, 4);
        let history = (0..300).map(|i| format!("line {i:03}")).collect::<Vec<_>>();
        t.feed(history.join("\r\n").as_bytes());
        t.start_selection(3, 7, Side::Right, false);
        assert!(t.scroll_selection(8, 0, 0, Side::Left));
        t.feed(b"\x1b[4;1H\x1b[2Kline 299");
        assert!(t.scroll_selection(288, 0, 0, Side::Left));
        assert_eq!(t.selection_text().as_deref(), Some(history.join("\n").as_str()));

        t.feed(b"\x1b[3J");
        assert!(!t.extend_selection(0, 0, Side::Left));
        assert!(t.selection_text().is_none());
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