//! Logs screen, as in the legacy client: one row per recorded session with
//! Date, User, Host and Saved columns; Date and Saved sort the list, the
//! Saved cell edits its comment, and a click elsewhere opens the recording read-only.

use std::ops::Range;

use gpui::{canvas, div, fill, prelude::*, px, rgb, rgba, size, svg, Bounds, ClipboardItem, Context, Div, ElementInputHandler, EntityInputHandler, FontWeight, HighlightStyle, InteractiveText, KeyDownEvent, MouseButton, Pixels, Point, ScrollHandle, SharedString, StyledText, Subscription, TextLayout, UnderlineStyle, UTF16Selection, Window};
use termian_native::log_comment::Draft;
use termian_native::session_log::{date_label, device_name, times_label};
use termian_native::vault::HistoryEntry;
use termian_native::theme;

use super::assets::icon;
use super::hosts::{os_tile, tile};
use super::sections::{bare_page, empty_state};
use super::text_input::{self, Mode};
use crate::Shell;

const DATE: f32 = 140.;
const USER: f32 = 300.;
const HOST: f32 = 180.;

/// Sorting and the one Saved cell being edited.
#[derive(Default)]
pub struct LogsView {
    oldest_first: bool,
    saved_first: bool,
    editor: Option<CommentEditor>,
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

fn header_cell(label: &str, width: Option<f32>) -> Div {
    div()
        .when_some(width, |d, w| d.w(px(w)).flex_none())
        .when(width.is_none(), |d| d.flex_1())
        .px(px(15.))
        .flex()
        .items_center()
        .gap(px(6.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(theme::TEXT))
        .child(label.to_owned())
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
            .hover(|s| s.bg(rgb(theme::INPUT)))
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
            .flex_1().min_w(px(140.)).px(px(15.)).py(px(10.)).flex().gap(px(8.)).rounded_md()
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

    pub fn render_logs(&self, cx: &mut Context<Self>) -> Div {
        let mut logs = self.vault.logs(self.logs_view.saved_first);
        if self.logs_view.oldest_first && !self.logs_view.saved_first {
            logs.reverse();
        }
        if logs.is_empty() {
            return bare_page("logs-scroll", empty_state("logs", "Session logs", "Sessions you open are recorded here, encrypted. They are kept for 60 days."));
        }
        // Legacy «sorting» bars: longest at the bottom for newest first, flipped for oldest.
        let arrow = |up: bool| svg().path(icon("sorting")).size(px(12.)).text_color(rgb(theme::TEXT_MUTED)).when(up, |s| s.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5))));
        let header = div()
            .h(px(40.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(theme::SURFACE_ACTIVE))
            .child(
                header_cell("Date", Some(DATE)).child(
                    div()
                        .id("logs-sort-date")
                        .px_1()
                        .rounded_sm()
                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                        .child(arrow(self.logs_view.oldest_first))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.logs_view.oldest_first = !shell.logs_view.oldest_first;
                            shell.logs_view.saved_first = false;
                            cx.notify();
                        })),
                ),
            )
            .child(header_cell("User", Some(USER)))
            .child(header_cell("Host", Some(HOST)))
            .child(
                header_cell("Saved", None).child(
                    div()
                        .id("logs-sort-saved")
                        .px_1()
                        .rounded_sm()
                        .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                        .child(svg().path(icon(if self.logs_view.saved_first { "bookmark_filled" } else { "bookmark" })).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                        .on_click(cx.listener(|shell, _, _, cx| {
                            shell.logs_view.saved_first = !shell.logs_view.saved_first;
                            cx.notify();
                        })),
                ),
            );

        let device = device_name();
        let rows = logs.into_iter().map(|entry| {
            let id = entry.id;
            let host = entry.host.and_then(|h| self.vault.host(h));
            let icon_tile = match (host, entry.host) {
                (Some(host), _) => os_tile(host.os.as_deref()),
                (None, None) => tile("local", theme::TILE_BLUE),
                (None, Some(_)) => tile("new_host", theme::TILE_BLUE),
            };
            let times = entry.disconnected_at.map(|end| times_label(entry.connected_at, end)).unwrap_or_default();
            div()
                .id(SharedString::from(format!("log-{id}")))
                .group(SharedString::from(format!("log-row-{id}")))
                .min_h(px(60.))
                .flex()
                .items_start()
                // Legacy rows are cards separated by a background line.
                .bg(rgb(theme::SURFACE))
                .border_b_1()
                .border_color(rgb(theme::BACKGROUND))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(
                    div()
                        .w(px(DATE))
                        .flex_none()
                        .px(px(15.))
                        .py(px(10.))
                        .child(div().text_color(rgb(theme::TEXT)).child(date_label(entry.connected_at)))
                        .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(times)),
                )
                // The native client has no account: the legacy "No account" cell with this device's name.
                .child(
                    div()
                        .w(px(USER))
                        .flex_none()
                        .px(px(15.))
                        .py(px(10.))
                        .flex()
                        .gap(px(10.))
                        .child(div().size(px(24.)).flex_none().rounded_full().bg(rgb(theme::SURFACE_ACTIVE)).flex().items_center().justify_center().child(svg().path(icon("person")).size(px(12.)).text_color(rgb(theme::TEXT_MUTED))))
                        .child(div().min_w_0().child(div().text_color(rgb(theme::TEXT_MUTED)).child("No account")).child(div().text_sm().truncate().text_color(rgb(theme::TEXT_MUTED)).child(device.clone()))),
                )
                .child(
                    div()
                        .w(px(HOST))
                        .flex_none()
                        .min_w_0()
                        .px(px(15.))
                        .py(px(10.))
                        .flex()
                        .gap(px(10.))
                        .child(icon_tile.size(px(24.)).rounded(px(6.)))
                        .child(div().min_w_0().child(div().truncate().text_color(rgb(theme::TEXT)).child(entry.label.clone())).child(div().text_sm().truncate().text_color(rgb(theme::TEXT_MUTED)).child(entry.subtitle.clone()))),
                )
                .child(self.render_log_saved_cell(entry, cx))
                .on_click(cx.listener(move |shell, _, window, cx| shell.open_log(id, window, cx)))
        });
        bare_page("logs-scroll", div().pt_3().child(header).children(rows))
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
