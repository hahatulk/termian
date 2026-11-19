use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use gpui::{
    App, ClipboardItem, Context, Div, EntityId, Global, Keystroke, Modifiers, MouseButton, Pixels,
    Point, SharedString, Stateful, Window, anchored, deferred, div, prelude::*, px, rgb, rgba,
};
use termian_native::theme;

use super::dialog::Key;
use super::host_editor::menu_panel;

#[derive(Clone, Copy)]
pub enum Mode {
    Line,
    Secret,
    Multiline,
    PrivateKey,
}

#[derive(Clone, PartialEq, Eq)]
struct Id(EntityId, String);

impl Id {
    fn new<T: 'static>(key: impl Debug, cx: &Context<T>) -> Self {
        Self(cx.entity_id(), format!("{key:?}"))
    }
}

#[derive(Default)]
struct Inputs {
    id: Option<Id>,
    editor: Editor,
    menu: Option<(Id, Point<Pixels>)>,
}

impl Global for Inputs {}

pub fn reset(cx: &mut App) {
    if cx.try_global::<Inputs>().is_some_and(|state| state.id.is_some() || state.menu.is_some()) {
        cx.set_global(Inputs::default());
    }
}

#[derive(Default)]
struct Editor {
    fingerprint: u64,
    anchor: usize,
    cursor: usize,
    undo: Vec<(String, usize, usize)>,
    redo: Vec<(String, usize, usize)>,
}

fn fingerprint(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl Editor {
    fn new(value: &str) -> Self {
        Self {
            fingerprint: fingerprint(value),
            anchor: value.len(),
            cursor: value.len(),
            ..Self::default()
        }
    }

    fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    fn move_to(&mut self, cursor: usize, extend: bool) {
        self.cursor = cursor;
        if !extend {
            self.anchor = cursor;
        }
    }

    fn remember(
        history: &mut Vec<(String, usize, usize)>,
        value: &str,
        anchor: usize,
        cursor: usize,
    ) {
        if value.len() > 128 * 1024 {
            history.clear();
            return;
        }
        if history.len() == 24 {
            history.remove(0);
        }
        history.push((value.to_owned(), anchor, cursor));
    }

    fn replace(
        &mut self,
        value: &mut String,
        range: Range<usize>,
        text: &str,
        secret: bool,
    ) -> bool {
        if value[range.clone()] == *text {
            self.move_to(range.start + text.len(), false);
            return false;
        }
        if !secret {
            Self::remember(&mut self.undo, value, self.anchor, self.cursor);
            self.redo.clear();
        }
        let end = range.start + text.len();
        value.replace_range(range, text);
        self.move_to(end, false);
        true
    }

    fn restore(&mut self, value: &mut String, undo: bool) -> bool {
        let (from, to) = if undo {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let Some((text, anchor, cursor)) = from.pop() else {
            return false;
        };
        Self::remember(to, value, self.anchor, self.cursor);
        *value = text;
        self.anchor = anchor;
        self.cursor = cursor;
        true
    }
}

fn step(value: &str, cursor: usize, right: bool, word: bool) -> usize {
    let class = |ch: char| {
        if ch.is_whitespace() {
            0
        } else if ch.is_alphanumeric() || ch == '_' {
            1
        } else {
            2
        }
    };
    let mut chars: Box<dyn Iterator<Item = (usize, char)>> = if right {
        Box::new(
            value[cursor..]
                .char_indices()
                .map(move |(i, c)| (cursor + i + c.len_utf8(), c)),
        )
    } else {
        Box::new(value[..cursor].char_indices().rev())
    };
    let Some((mut end, first)) = chars.next() else {
        return cursor;
    };
    if word {
        let mut group = class(first);
        for (at, ch) in chars {
            let next = class(ch);
            if next != group {
                if group == 0 {
                    group = next;
                } else {
                    break;
                }
            }
            end = at;
        }
    }
    end
}

fn line_edge(value: &str, cursor: usize, right: bool) -> usize {
    if right {
        value[cursor..]
            .find('\n')
            .map_or(value.len(), |i| cursor + i)
    } else {
        value[..cursor].rfind('\n').map_or(0, |i| i + 1)
    }
}

fn vertical(value: &str, cursor: usize, down: bool) -> usize {
    let start = line_edge(value, cursor, false);
    let column = value[start..cursor].chars().count();
    let next = if down {
        let end = line_edge(value, cursor, true);
        if end == value.len() {
            return end;
        }
        end + 1
    } else if start > 0 {
        line_edge(value, start - 1, false)
    } else {
        return 0;
    };
    let end = line_edge(value, next, true);
    next + value[next..end]
        .char_indices()
        .nth(column)
        .map_or(end - next, |(i, _)| i)
}

pub fn command(key: &Keystroke) -> bool {
    (key.modifiers.platform || key.modifiers.control) && !key.modifiers.alt
}

pub fn edit<T: 'static>(
    key: impl Debug,
    value: Option<&mut String>,
    stroke: &Keystroke,
    mode: Mode,
    cx: &mut Context<T>,
) -> Key {
    let id = Id::new(key, cx);
    let mut cmd = command(stroke);
    let shift = stroke.modifiers.shift;
    let multiline = matches!(mode, Mode::Multiline | Mode::PrivateKey);
    let secret = matches!(mode, Mode::Secret);
    let remember = !matches!(mode, Mode::Secret | Mode::PrivateKey);
    let name = match (stroke.key.as_str(), stroke.modifiers.control, shift) {
        ("insert", true, false) => "c",
        ("insert", false, true) => {
            cmd = true;
            "v"
        }
        ("delete", false, true) if !stroke.modifiers.platform => {
            cmd = true;
            "x"
        }
        (name, _, _) => name,
    };
    match name {
        "escape" => {
            reset(cx);
            return Key::Escape;
        }
        "enter" if !multiline => {
            reset(cx);
            return Key::Enter;
        }
        _ => {}
    }
    let Some(value) = value else {
        return Key::Ignored;
    };
    let paste = (cmd && name == "v")
        .then(|| cx.read_from_clipboard().and_then(|item| item.text()))
        .flatten();
    let mut copied = None;
    let state = cx.default_global::<Inputs>();
    let hash = fingerprint(value);
    if state.id.as_ref() != Some(&id) || state.editor.fingerprint != hash {
        state.id = Some(id);
        state.editor = Editor::new(value);
    }
    let editor = &mut state.editor;
    let range = editor.selection();
    let word = stroke.modifiers.alt || (stroke.modifiers.control && !stroke.modifiers.platform);
    let changed = match name {
        "a" if cmd => {
            editor.anchor = 0;
            editor.cursor = value.len();
            false
        }
        "c" | "x" if cmd => {
            if secret || range.is_empty() {
                false
            } else {
                copied = Some(value[range.clone()].to_owned());
                name == "x" && editor.replace(value, range, "", !remember)
            }
        }
        "v" if cmd => {
            if let Some(paste) = paste {
                if multiline {
                    let paste = paste.replace("\r\n", "\n").replace('\r', "\n");
                    editor.replace(value, range, &paste, !remember)
                } else {
                    editor.replace(value, range, paste.lines().next().unwrap_or(""), !remember)
                }
            } else {
                false
            }
        }
        "z" | "y" if cmd => remember && editor.restore(value, name == "z" && !shift),
        "left" | "right" => {
            let right = name == "right";
            let cursor = if stroke.modifiers.platform {
                line_edge(value, editor.cursor, right)
            } else if !shift && !range.is_empty() {
                if right { range.end } else { range.start }
            } else {
                step(value, editor.cursor, right, word)
            };
            editor.move_to(cursor, shift);
            false
        }
        "up" | "down" if multiline => {
            let down = name == "down";
            let cursor = if stroke.modifiers.platform {
                if down { value.len() } else { 0 }
            } else {
                vertical(value, editor.cursor, down)
            };
            editor.move_to(cursor, shift);
            false
        }
        "home" | "end" => {
            let end = name == "end";
            let cursor = if cmd {
                if end { value.len() } else { 0 }
            } else {
                line_edge(value, editor.cursor, end)
            };
            editor.move_to(cursor, shift);
            false
        }
        "backspace" | "delete" => {
            let range = if range.is_empty() {
                let other = if stroke.modifiers.platform {
                    line_edge(value, editor.cursor, name == "delete")
                } else {
                    step(value, editor.cursor, name == "delete", word)
                };
                editor.cursor.min(other)..editor.cursor.max(other)
            } else {
                range
            };
            editor.replace(value, range, "", !remember)
        }
        "enter" if multiline && !cmd => editor.replace(value, range, "\n", !remember),
        _ if !stroke.modifiers.platform && (!stroke.modifiers.control || stroke.modifiers.alt) => {
            let Some(text) = stroke
                .key_char
                .as_deref()
                .filter(|text| !text.chars().any(char::is_control))
            else {
                return Key::Ignored;
            };
            editor.replace(value, range, text, !remember)
        }
        _ => return Key::Ignored,
    };
    editor.fingerprint = fingerprint(value);
    state.menu = None;
    if let Some(copied) = copied {
        cx.write_to_clipboard(ClipboardItem::new_string(copied));
    }
    cx.stop_propagation();
    cx.notify();
    if changed { Key::Edited } else { Key::Ignored }
}

pub fn text<T: 'static>(
    key: impl Debug,
    value: &str,
    placeholder: &str,
    focused: bool,
    mode: Mode,
    cx: &Context<T>,
) -> Div {
    let id = Id::new(key, cx);
    let (range, cursor) = cx
        .try_global::<Inputs>()
        .filter(|state| {
            focused
                && state.id.as_ref() == Some(&id)
                && state.editor.fingerprint == fingerprint(value)
        })
        .map_or((value.len()..value.len(), value.len()), |state| {
            (state.editor.selection(), state.editor.cursor)
        });
    let show = |s: &str| {
        if matches!(mode, Mode::Secret) {
            "•".repeat(s.chars().count())
        } else {
            s.to_owned()
        }
    };
    let caret = || {
        div()
            .mx(px(1.))
            .w(px(1.5))
            .h(px(16.))
            .flex_none()
            .bg(rgb(theme::TEXT))
    };
    if value.is_empty() {
        return div()
            .min_w_0()
            .flex()
            .items_center()
            .when(focused, |d| d.child(caret()))
            .child(
                div()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(placeholder.to_owned()),
            );
    }
    let mut at = 0;
    let mut column = div()
        .min_w_0()
        .flex()
        .flex_col()
        .text_color(rgb(theme::TEXT));
    for line in value.split('\n') {
        let end = at + line.len();
        let mut row = div().min_h(px(16.)).flex().items_center().flex_wrap();
        if focused && range.is_empty() && (at..=end).contains(&cursor) {
            row = row
                .child(show(&value[at..cursor]))
                .child(caret())
                .child(show(&value[cursor..end]));
        } else if focused && !range.is_empty() && range.start <= end && range.end > at {
            let start = range.start.clamp(at, end);
            let stop = range.end.clamp(at, end);
            let selected = if stop == end && range.end > end {
                format!("{} ", show(&value[start..stop]))
            } else {
                show(&value[start..stop])
            };
            row = row
                .child(show(&value[at..start]))
                .child(div().bg(rgba(0x5372e866)).child(selected))
                .child(show(&value[stop..end]));
        } else {
            row = row.child(show(line));
        }
        column = column.child(row);
        at = end + 1;
    }
    column
}

pub fn menu_open<T: 'static>(key: impl Debug, cx: &Context<T>) -> bool {
    let id = Id::new(key, cx);
    cx.try_global::<Inputs>()
        .and_then(|state| state.menu.as_ref())
        .is_some_and(|(active, _)| *active == id)
}

pub fn field<T: 'static>(
    target: Stateful<Div>,
    key: impl Debug,
    mode: Mode,
    activate: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
    cx: &Context<T>,
) -> Stateful<Div> {
    let id = Id::new(key, cx);
    let active_menu = cx
        .try_global::<Inputs>()
        .and_then(|state| state.menu.as_ref())
        .filter(|(key, _)| key == &id)
        .map(|(_, at)| *at);
    let activate = std::rc::Rc::new(activate);
    let left = activate.clone();
    let click_id = id.clone();
    let mut field = target
        .on_click(cx.listener(move |view, _, window, cx| {
            let state = cx.default_global::<Inputs>();
            if state.id.as_ref() != Some(&click_id) {
                state.id = None;
                state.editor = Editor::default();
            }
            state.menu = None;
            left(view, window, cx);
            cx.notify();
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |view, event: &gpui::MouseDownEvent, window, cx| {
                activate(view, window, cx);
                cx.default_global::<Inputs>().menu = Some((id.clone(), event.position));
                cx.stop_propagation();
                cx.notify();
            }),
        );
    if let Some(at) = active_menu {
        let mut panel = menu_panel("text-field-menu")
            .occlude()
            .on_mouse_down_out(cx.listener(|_, _, _, cx| {
                cx.default_global::<Inputs>().menu = None;
                cx.notify();
            }));
        let secret = matches!(mode, Mode::Secret);
        let undo = !secret && !matches!(mode, Mode::PrivateKey);
        for (label, key, shifted, enabled) in [
            ("Undo", "z", false, undo),
            ("Redo", "z", true, undo),
            ("Cut", "x", false, !secret),
            ("Copy", "c", false, !secret),
            ("Paste", "v", false, true),
            ("Select All", "a", false, true),
        ] {
            panel = panel.child(
                div()
                    .id(SharedString::from(label))
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .text_color(rgb(if enabled {
                        theme::TEXT
                    } else {
                        theme::TEXT_MUTED
                    }))
                    .when(enabled, |d| {
                        d.hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                            .on_click(cx.listener(move |_, _, window, cx| {
                                cx.default_global::<Inputs>().menu = None;
                                let key = Keystroke {
                                    modifiers: Modifiers {
                                        control: true,
                                        shift: shifted,
                                        ..Default::default()
                                    },
                                    key: key.to_owned(),
                                    key_char: None,
                                };
                                let focus = window.focused(cx);
                                window.defer(cx, move |window, cx| {
                                    if focus.is_some_and(|focus| focus.is_focused(window)) {
                                        window.dispatch_keystroke(key, cx);
                                    }
                                });
                                cx.stop_propagation();
                                cx.notify();
                            }))
                    })
                    .child(label),
            );
        }
        field = field.child(
            deferred(anchored().position(at).snap_to_window().child(super::motion::popover(panel, "field-menu-enter"))).with_priority(3),
        );
    }
    field
}
