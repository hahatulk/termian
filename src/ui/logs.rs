use std::{ops::Range, rc::Rc};

use gpui::{canvas, div, fill, list, prelude::*, px, rgb, rgba, size, svg, Bounds, ClipboardItem, Context, Div, ElementInputHandler, EntityInputHandler, FocusHandle, FontWeight, HighlightStyle, InteractiveText, KeyDownEvent, ListAlignment, ListOffset, ListState, MouseButton, Pixels, Point, ScrollHandle, SharedString, StyledText, Subscription, TextLayout, UnderlineStyle, UTF16Selection, Window};
use termian_native::log_comment::Draft;
use termian_native::session_log::{clock_label, date_label, duration_label};
use termian_native::vault::HistoryEntry;
use termian_native::theme;

use super::assets::icon;
use super::hosts::os_tile_sized;
use super::pointer::PointerFeedback;
use super::sections::{bare_page, empty_state};
use super::text_input::{self, Mode};
use crate::Shell;

const LOG_COLUMNS: [f32; 5] = [0.18, 0.26, 0.13, 0.18, 0.25];

/// Sorting and the one Saved cell being edited.
#[derive(Default)]
pub struct LogsView {
    oldest_first: bool,
    saved_first: bool,
    editor: Option<CommentEditor>,
    list: LogList,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct LogOrder {
    index: usize,
    id: u64,
    connected_at: i64,
    saved: bool,
}

enum LogItem {
    Day(SharedString),
    Session { index: usize, id: u64, stripe: bool, first: bool, last: bool },
}

impl LogItem {
    fn session_id(&self) -> Option<u64> {
        match self {
            Self::Session { id, .. } => Some(*id),
            Self::Day(_) => None,
        }
    }

    fn same_row(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Day(a), Self::Day(b)) => a == b,
            _ => self.session_id().is_some_and(|id| other.session_id() == Some(id)),
        }
    }
}

struct LogList {
    state: ListState,
    order: Vec<LogOrder>,
    items: Rc<[LogItem]>,
    sort: (bool, bool),
    focused: Option<(u64, usize)>,
}

impl Default for LogList {
    fn default() -> Self {
        Self {
            state: ListState::new(0, ListAlignment::Top, px(160.)),
            order: Vec::new(),
            items: Rc::from([]),
            sort: (false, false),
            focused: None,
        }
    }
}

impl LogList {
    fn update(&mut self, history: &[HistoryEntry], sort: (bool, bool), editing: Option<u64>, focus: &FocusHandle) {
        let (oldest_first, saved_first) = sort;
        let order = history.iter().enumerate().filter(|(_, row)| row.log.is_some()).map(|(index, row)| LogOrder {
            index, id: row.id, connected_at: row.connected_at, saved: saved_first && row.saved,
        });
        if self.sort != sort || !order.clone().eq(self.order.iter().copied()) {
            self.order = order.collect();
            let mut sorted = self.order.clone();
            sorted.sort_by_key(|row| std::cmp::Reverse(row.connected_at));
            if oldest_first {
                sorted.reverse();
            }

            let mut days: Vec<(String, Vec<LogOrder>)> = Vec::new();
            for row in sorted {
                let date = date_label(row.connected_at);
                match days.last_mut() {
                    Some((label, rows)) if *label == date => rows.push(row),
                    _ => days.push((date, vec![row])),
                }
            }

            let mut items = Vec::with_capacity(self.order.len() + days.len());
            for (date, mut rows) in days {
                if saved_first {
                    rows.sort_by_key(|row| !row.saved);
                }
                items.push(LogItem::Day(date.into()));
                let len = rows.len();
                items.extend(rows.into_iter().enumerate().map(|(i, row)| LogItem::Session {
                    index: row.index, id: row.id, stripe: i % 2 == 0, first: i == 0, last: i + 1 == len,
                }));
            }

            let top = self.state.logical_scroll_top();
            let new_top = (self.sort == sort).then(|| {
                let anchor = self.items.get(top.item_ix)?;
                let item_ix = items.iter().position(|item| item.same_row(anchor))?;
                Some(ListOffset { item_ix, ..top })
            }).flatten();
            self.state.reset(items.len());
            if let Some(top) = new_top {
                self.state.scroll_to(top);
            }
            self.items = items.into();
            self.sort = sort;
            self.focused = None;
        }

        if self.focused.map(|(id, _)| id) != editing {
            let top = self.state.logical_scroll_top();
            if let Some((_, ix)) = self.focused.take() {
                self.state.splice(ix..ix + 1, 1);
            }
            if let Some(id) = editing {
                if let Some(ix) = self.items.iter().position(|item| item.session_id() == Some(id)) {
                    self.state.splice_focusable(ix..ix + 1, [Some(focus.clone())]);
                    self.focused = Some((id, ix));
                }
            }
            self.state.scroll_to(top);
        }
    }
}

struct CommentEditor {
    draft: Draft,
    marked: Option<Range<usize>>,
    layout: Option<TextLayout>,
    selecting: bool,
    scroll: ScrollHandle,
    reveal_cursor: bool,
    _blur: Subscription,
}

fn log_column(index: usize) -> Div {
    div().w(gpui::relative(LOG_COLUMNS[index])).flex_none().min_w_0()
        .px_2().flex().items_center().gap_2().text_size(px(12.))
        .when(index != 0, |d| d.border_l_1().border_color(rgba(0xffffff12)))
}

fn log_tile(glyph: &str, background: u32, color: u32) -> Div {
    div().size(px(34.)).flex_none().rounded(px(10.)).bg(rgb(background))
        .flex().items_center().justify_center()
        .child(svg().path(icon(glyph)).size(px(17.)).text_color(rgb(color)))
}

fn log_value(text: &str, detail: Option<&str>) -> Div {
    div().flex_1().min_w_0()
        .child(div().truncate().text_color(rgb(theme::TEXT)).child(text.to_owned()))
        .when_some(detail.filter(|s| !s.is_empty()), |d, detail| {
            d.child(div().truncate().text_size(px(10.)).text_color(rgb(theme::TEXT_MUTED)).child(detail.to_owned()))
        })
}

impl Shell {
    fn edit_log_comment(&mut self, id: u64, bookmark: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.logs_view.editor.as_ref().is_some_and(|e| e.draft.id == id) { return; }
        self.finish_log_comment(true, cx);
        let Some(entry) = self.vault.history.iter().find(|e| e.id == id && e.log.is_some()) else { return };
        let draft = Draft::new(entry, bookmark);
        let blur = cx.on_blur(&self.editor_focus, window, |shell, _, cx| shell.finish_log_comment(true, cx));
        self.logs_view.editor = Some(CommentEditor { draft, marked: None, layout: None, selecting: false, scroll: ScrollHandle::new(), reveal_cursor: true, _blur: blur });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn finish_log_comment(&mut self, save: bool, cx: &mut Context<Self>) {
        let Some(editor) = self.logs_view.editor.take() else { return };
        if save && editor.draft.save(&mut self.vault) { self.save_vault(); }
        cx.notify();
    }

    fn remove_log_bookmark(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.logs_view.editor.as_ref().is_some_and(|e| e.draft.id == id) {
            self.finish_log_comment(false, cx);
        }
        if self.vault.remove_log_bookmark(id) { self.save_vault(); }
        cx.notify();
    }

    fn on_log_comment_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.logs_view.editor.as_mut() else { return };
        let key = &event.keystroke;
        if editor.marked.is_some() { return; }
        let command = text_input::command(key);
        match key.key.as_str() {
            "escape" => {
                self.finish_log_comment(false, cx);
                window.focus(&self.root_focus);
            }
            "enter" if !key.modifiers.shift => {
                self.finish_log_comment(true, cx);
                window.focus(&self.root_focus);
            }
            "tab" => {
                self.finish_log_comment(true, cx);
                window.focus(&self.root_focus);
            }
            "enter" => editor.draft.replace(None, "\n"),
            "a" if command => {
                editor.draft.move_to(0, false);
                editor.draft.move_to(editor.draft.text().len(), true);
            }
            "c" | "x" if command => {
                let selection = editor.draft.selection();
                if !selection.is_empty() {
                    cx.write_to_clipboard(ClipboardItem::new_string(editor.draft.text()[selection].to_owned()));
                    if key.key == "x" { editor.draft.delete(true); }
                }
            }
            "v" if command => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    editor.draft.replace(None, &text.replace("\r\n", "\n").replace('\r', "\n"));
                }
            }
            "z" | "y" if command => editor.draft.undo_redo(key.key == "z" && !key.modifiers.shift),
            "left" | "right" => editor.draft.move_horizontal(key.key == "right", key.modifiers.shift),
            "home" => editor.draft.move_to(0, key.modifiers.shift),
            "end" => editor.draft.move_to(editor.draft.text().len(), key.modifiers.shift),
            "up" | "down" => {
                if let Some(layout) = &editor.layout {
                    if let Some(mut at) = layout.position_for_index(editor.draft.cursor()) {
                        at.y += layout.line_height() * if key.key == "up" { -1. } else { 1. };
                        let offset = layout.index_for_position(at).unwrap_or_else(|i| i);
                        editor.draft.move_to(offset, key.modifiers.shift);
                    }
                }
            }
            "backspace" => editor.draft.delete(true),
            "delete" => editor.draft.delete(false),
            _ => return, // Printable text and IME go through EntityInputHandler.
        }
        if let Some(editor) = &mut self.logs_view.editor { editor.reveal_cursor = true; }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_log_saved_cell(&self, entry: &HistoryEntry, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let id = entry.id;
        let editing = self.logs_view.editor.as_ref().filter(|e| e.draft.id == id);
        let saved = entry.saved;
        let bookmark = div()
            .id(SharedString::from(format!("log-saved-{id}")))
            // A boxed bookmark, as the legacy Saved cell.
            .size(px(24.)).flex_none().flex().items_center().justify_center().rounded_md()
            .bg(rgb(theme::SURFACE_ACTIVE))
            .surface_feedback(theme::SURFACE_ACTIVE)
            .child(svg().path(icon(if saved || editing.is_some() { "bookmark_filled" } else { "bookmark" })).size(px(14.)).text_color(rgb(if saved || editing.is_some() { theme::ACCENT } else { theme::TEXT_MUTED })))
            .on_click(cx.listener(move |shell, _, window, cx| {
                cx.stop_propagation();
                if saved || shell.logs_view.editor.as_ref().is_some_and(|e| e.draft.id == id) {
                    shell.remove_log_bookmark(id, cx);
                    window.focus(&shell.root_focus);
                } else {
                    shell.edit_log_comment(id, true, window, cx);
                }
            }));
        let cell = div().id(SharedString::from(format!("log-comment-{id}")))
            .w_full().min_w_0().flex().items_center().gap_1().rounded_md()
            .when(!saved && editing.is_none(), |d| d.invisible().group_hover(SharedString::from(format!("log-row-{id}")), |s| s.visible()))
            .when(editing.is_some(), |d| d.on_mouse_down_out(cx.listener(move |shell, _, window, cx| {
                if !text_input::menu_open("log-comment", cx) && shell.logs_view.editor.as_ref().is_some_and(|e| e.draft.id == id) {
                    shell.finish_log_comment(true, cx);
                    window.focus(&shell.root_focus);
                }
            })))
            .child(bookmark)
            .on_click(cx.listener(move |shell, _, window, cx| {
                cx.stop_propagation();
                shell.edit_log_comment(id, false, window, cx);
            }));
        let Some(editor) = editing else {
            let links = termian_native::log_comment::links(&entry.comment);
            let ranges: Vec<_> = links.iter().map(|(range, _)| range.clone()).collect();
            let text = StyledText::new(if entry.comment.is_empty() { "Add a comment".to_owned() } else { entry.comment.clone() })
                .with_highlights(ranges.iter().cloned().map(|range| (range, HighlightStyle { color: Some(rgb(theme::ACCENT).into()), ..Default::default() })));
            // Legacy shows «Add a comment» only while the row is hovered.
            return cell.child(div().flex_1().min_w_0().line_clamp(2).text_color(rgb(if entry.comment.is_empty() { theme::TEXT_MUTED } else { theme::TEXT }))
                .when(entry.comment.is_empty(), |d| d.invisible().group_hover(SharedString::from(format!("log-row-{id}")), |s| s.visible()))
                .child(InteractiveText::new(SharedString::from(format!("log-comment-text-{id}")), text).on_click(ranges, move |index, _, cx| {
                    cx.stop_propagation();
                    if let Some((_, url)) = links.get(index) { cx.open_url(url); }
                })));
        };
        let mut highlights = Vec::new();
        let selection = editor.draft.selection();
        if !selection.is_empty() {
            highlights.push((selection.clone(), HighlightStyle { background_color: Some(rgba(0x5372e866).into()), ..Default::default() }));
        } else if let Some(marked) = &editor.marked {
            highlights.push((marked.clone(), HighlightStyle { underline: Some(UnderlineStyle { thickness: px(1.), color: None, wavy: false }), ..Default::default() }));
        }
        // A trailing space gives empty text and a final newline a caret position.
        let text = StyledText::new(format!("{} ", editor.draft.text())).with_highlights(highlights);
        let layout = text.layout().clone();
        let entity = cx.entity();
        let focus = self.editor_focus.clone();
        let cursor = editor.draft.cursor();
        let paint = canvas(|_, _, _| (), move |bounds, _, window, cx| {
            window.handle_input(&focus, ElementInputHandler::new(bounds, entity.clone()), cx);
            let caret = layout.position_for_index(cursor);
            if selection.is_empty() && focus.is_focused(window) {
                if let Some(at) = caret {
                    window.paint_quad(fill(Bounds::new(at, size(px(1.5), layout.line_height())), rgb(theme::TEXT)));
                }
            }
            entity.update(cx, |shell, cx| {
                let Some(editor) = shell.logs_view.editor.as_mut().filter(|e| e.draft.id == id) else { return };
                if editor.reveal_cursor {
                    editor.reveal_cursor = false;
                    if let Some(at) = caret {
                        let viewport = editor.scroll.bounds();
                        let delta = if at.y < viewport.top() { viewport.top() - at.y } else { (viewport.bottom() - at.y - layout.line_height()).min(px(0.)) };
                        if delta != px(0.) {
                            let mut offset = editor.scroll.offset();
                            offset.y += delta;
                            editor.scroll.set_offset(offset);
                            cx.notify();
                        }
                    }
                }
                editor.layout = Some(layout);
            });
        }).absolute().inset_0();
        let input = div().id("log-comment-input").flex_1().min_w_0().relative().cursor_text()
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_log_comment_key))
            .on_mouse_down(MouseButton::Left, cx.listener(|shell, event: &gpui::MouseDownEvent, _, cx| {
                let Some(editor) = &mut shell.logs_view.editor else { return };
                if let Some(layout) = &editor.layout {
                    let offset = layout.index_for_position(event.position).unwrap_or_else(|i| i);
                    editor.draft.move_to(offset, event.modifiers.shift);
                    editor.marked = None;
                    editor.selecting = true;
                    cx.notify();
                }
            }))
            .on_mouse_move(cx.listener(|shell, event: &gpui::MouseMoveEvent, _, cx| {
                let Some(editor) = &mut shell.logs_view.editor else { return };
                if editor.selecting && event.pressed_button == Some(MouseButton::Left) {
                    if let Some(layout) = &editor.layout {
                        editor.draft.move_to(layout.index_for_position(event.position).unwrap_or_else(|i| i), true);
                        editor.reveal_cursor = true;
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|shell, _, _, _| {
                if let Some(editor) = &mut shell.logs_view.editor { editor.selecting = false; }
            }))
            .child(div().id("log-comment-scroll").min_h(px(24.)).max_h(px(96.)).overflow_y_scroll().track_scroll(&editor.scroll)
                .child(div().relative().text_size(px(14.)).line_height(px(24.)).text_color(rgb(theme::TEXT)).child(text).child(paint)))
            .when(editor.draft.text().is_empty(), |d| d.child(div().absolute().top_0().left_0().text_color(rgb(theme::TEXT_MUTED)).child("Add a comment")));
        let input = text_input::field(input, "log-comment", Mode::Multiline, |shell, window, _| window.focus(&shell.editor_focus), cx);
        cell.bg(rgb(theme::INPUT)).border_1().border_color(rgb(theme::ACCENT)).child(input)
    }

    pub fn render_logs(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let editing = self.logs_view.editor.as_ref().map(|editor| editor.draft.id);
        let sort = (self.logs_view.oldest_first, self.logs_view.saved_first);
        self.logs_view.list.update(&self.vault.history, sort, editing, &self.editor_focus);
        if self.logs_view.list.items.is_empty() {
            return bare_page("logs-scroll", empty_state("logs", "Session logs", "Sessions you open are recorded here, encrypted. Recordings are kept for 60 days.")).into_any_element();
        }
        let arrow = || svg().path(icon("sorting")).size(px(12.)).text_color(rgb(theme::TEXT_MUTED))
            .when(self.logs_view.oldest_first, |s| s.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5))));
        let mut header = div().h(px(48.)).flex_none().flex().items_center().border_b_1().border_color(rgb(theme::SURFACE_ACTIVE));
        for (index, label, glyph) in [
            (0, "User", "person"), (1, "Host", "hosts"), (2, "Time", "calendar"),
            (3, "Device", "local"), (4, "Location", "navigate"),
        ] {
            header = header.child(log_column(index)
                .child(svg().path(icon(glyph)).size(px(12.)).flex_none().text_color(rgb(theme::TEXT_MUTED)))
                .child(div().text_color(rgb(theme::TEXT)).child(label))
                .when(index == 2, |d| d.child(div().id("logs-sort-time").px_1().rounded_sm()
                    .icon_feedback().child(arrow())
                    .on_click(cx.listener(|shell, _, _, cx| {
                        shell.logs_view.oldest_first = !shell.logs_view.oldest_first;
                        cx.notify();
                    }))))
                .when(index == 4, |d| d.child(div().id("logs-sort-saved").ml_auto().px_1().rounded_sm()
                    .icon_feedback()
                    .child(svg().path(icon(if self.logs_view.saved_first { "bookmark_filled" } else { "bookmark" })).size(px(13.)).text_color(rgb(theme::TEXT_MUTED)))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        shell.logs_view.saved_first = !shell.logs_view.saved_first;
                        cx.notify();
                    })))));
        }

        let items = self.logs_view.list.items.clone();
        let shell = cx.entity();
        let body = list(self.logs_view.list.state.clone(), move |ix, _, cx| match &items[ix] {
            LogItem::Day(date) => div().w_full().px_2().py(px(10.)).text_xs().font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme::TEXT)).child(date.clone()).into_any_element(),
            LogItem::Session { index, stripe, first, last, .. } => shell.update(cx, |shell, cx| {
                let row = &shell.vault.history[*index];
                let mut element = shell.render_log_row(row, *stripe, cx);
                let corners = &mut element.style().corner_radii;
                if *first {
                    corners.top_left = Some(px(12.).into());
                    corners.top_right = Some(px(12.).into());
                }
                if *last {
                    corners.bottom_left = Some(px(12.).into());
                    corners.bottom_right = Some(px(12.).into());
                }
                element.into_any_element()
            }),
        }).flex_1().min_h_0().pb_3();
        div().id("logs-horizontal").size_full().min_w_0().overflow_x_scroll().px(px(10.)).child(
            div().size_full().min_w(px(800.)).flex().flex_col().child(header).child(body)
        ).into_any_element()
    }

    fn render_log_row(&self, entry: &HistoryEntry, stripe: bool, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let id = entry.id;
        let host = entry.host.and_then(|id| self.vault.host(id));
        let os = entry.log_os.as_deref().or_else(|| host.and_then(|host| host.os.as_deref()));
        let tile = if entry.subtitle == "local" || entry.subtitle == "serial" {
            log_tile(if entry.subtitle == "serial" { "serial" } else { "local" }, theme::TILE_BLUE, theme::TEXT)
        } else {
            os_tile_sized(os, 34., 17.).rounded(px(10.))
        };
        let duration = entry.disconnected_at.map(|end| duration_label(entry.connected_at, end));
        let device = if entry.device.is_empty() { "Unknown device" } else { entry.device.as_str() };
        let background = if stripe { theme::SURFACE_ACTIVE } else { theme::SURFACE };
        div()
            .id(SharedString::from(format!("log-{id}")))
            .group(SharedString::from(format!("log-row-{id}")))
            .w_full()
            .min_h(px(50.))
            .py(px(5.))
            .flex()
            .items_center()
            .bg(rgb(background))
            .surface_feedback(background)
            .child(log_column(0).child(log_tile("person", theme::TITLE_BAR, theme::TEXT_MUTED)).child(log_value("No account", None)))
            .child(log_column(1).child(tile).child(log_value(&entry.label, Some(&entry.subtitle))))
            .child(log_column(2).child(log_value(&clock_label(entry.connected_at), duration.as_deref())))
            .child(log_column(3).child(log_tile("local", theme::TITLE_BAR, theme::TEXT_MUTED)).child(log_value(device, None)))
            .child(log_column(4).flex_col().items_start().gap_0()
                .child(div().w_full().truncate().text_color(rgb(theme::TEXT_MUTED)).child("Location unavailable"))
                .child(self.render_log_saved_cell(entry, cx)))
            .on_click(cx.listener(move |shell, _, window, cx| shell.open_log(id, window, cx)))
    }
}

// Register platform text input only while the Saved cell is painted. This
// supports keyboard layouts and IME instead of treating key names as text.
impl EntityInputHandler for Shell {
    fn text_for_range(&mut self, range: Range<usize>, actual: &mut Option<Range<usize>>, _: &mut Window, _: &mut Context<Self>) -> Option<String> {
        let draft = &self.logs_view.editor.as_ref()?.draft;
        let range = draft.byte_range(range);
        *actual = Some(draft.utf16_range(range.clone()));
        Some(draft.text()[range].to_owned())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        let draft = &self.logs_view.editor.as_ref()?.draft;
        let selection = draft.selection();
        Some(UTF16Selection { reversed: draft.cursor() == selection.start && !selection.is_empty(), range: draft.utf16_range(selection) })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let editor = self.logs_view.editor.as_ref()?;
        Some(editor.draft.utf16_range(editor.marked.clone()?))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.logs_view.editor { editor.marked = None; }
        cx.notify();
    }

    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.logs_view.editor else { return };
        let remember = editor.marked.is_none();
        let range = range.map(|r| editor.draft.byte_range(r)).or(editor.marked.take());
        editor.draft.replace_with_history(range, text, remember);
        editor.marked = None;
        editor.reveal_cursor = true;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(&mut self, range: Option<Range<usize>>, text: &str, selection: Option<Range<usize>>, _: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.logs_view.editor else { return };
        let remember = editor.marked.is_none();
        let range = range.map(|r| editor.draft.byte_range(r)).or(editor.marked.take()).unwrap_or_else(|| editor.draft.selection());
        let start = range.start;
        editor.draft.replace_with_history(Some(range), text, remember);
        let end = editor.draft.cursor();
        editor.marked = (start != end).then_some(start..end);
        if let Some(selection) = selection {
            let prefix = editor.draft.utf16_range(start..start).start;
            let selection = editor.draft.byte_range(prefix + selection.start..prefix + selection.end);
            editor.draft.move_to(selection.start.min(end), false);
            editor.draft.move_to(selection.end.min(end), true);
        }
        editor.reveal_cursor = true;
        cx.notify();
    }

    fn bounds_for_range(&mut self, range: Range<usize>, _: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let editor = self.logs_view.editor.as_ref()?;
        let layout = editor.layout.as_ref()?;
        let range = editor.draft.byte_range(range);
        let start = layout.position_for_index(range.start)?;
        let end = layout.position_for_index(range.end)?;
        let width = if start.y == end.y { (end.x - start.x).max(px(2.)) } else { px(2.) };
        Some(Bounds::new(start, size(width, layout.line_height())))
    }

    fn character_index_for_point(&mut self, at: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let editor = self.logs_view.editor.as_ref()?;
        let offset = editor.layout.as_ref()?.index_for_position(at).unwrap_or_else(|i| i).min(editor.draft.text().len());
        Some(editor.draft.utf16_range(offset..offset).start)
    }
}
