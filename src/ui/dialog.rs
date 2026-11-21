//! Modal dialogs in the legacy style: a 76 px header with the title and a
//! close cross, 30 px of padding, text, then right-aligned buttons. Escape
//! closes and Enter confirms, as in the legacy `MessageDialog`.

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, FocusHandle, FontWeight, SharedString, Stateful};
use termian_native::theme;

use super::assets::icon;
use super::motion;
use super::pointer::PointerFeedback;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Accent,
    Error,
    Regular,
    /// Borderless text button.
    Text,
}

pub fn button(id: &str, label: &str, kind: Kind, disabled: bool) -> Stateful<Div> {
    let (bg, fg) = match kind {
        Kind::Accent => (Some(theme::ACCENT), 0xffffff),
        Kind::Error => (Some(theme::ERROR), 0xffffff),
        Kind::Regular => (Some(theme::SURFACE_ACTIVE), theme::TEXT),
        Kind::Text => (None, theme::TEXT),
    };
    div()
        .id(SharedString::from(id.to_owned()))
        .h(px(36.))
        .px_4()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(fg))
        .when_some(bg, |d, bg| d.bg(rgb(bg)))
        .when(!disabled, |d| match bg {
            Some(color) => d.surface_feedback(color),
            None => d.icon_feedback(),
        })
        .when(disabled, |d| d.opacity(0.5))
        .child(label.to_owned())
}

/// The cross in the header.
pub fn close_button(id: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .icon_feedback()
        .child(svg().path(icon("close")).size(px(12.)).text_color(rgb(0xffffff)))
}

/// The legacy slider's «→|» button: hides the slider to the right.
pub fn collapse_button(id: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .icon_feedback()
        .child(svg().path(icon("collapse_right")).size(px(16.)).text_color(rgb(0xffffff)))
}

/// Dialog card. `error` paints the header red, as `titleColor: "error"` does.
pub fn card(title: &str, error: bool, close: Stateful<Div>, body: Div, buttons: Vec<AnyElement>) -> Div {
    div()
        .w_full()
        .flex_none()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(76.))
                .flex_none()
                .px(px(28.))
                .flex()
                .items_center()
                .bg(rgb(if error { theme::ERROR } else { theme::DIALOG_HEADER }))
                .child(div().flex_1().truncate().text_size(px(18.)).text_color(rgb(0xffffff)).child(title.to_owned()))
                .child(close),
        )
        .child(
            div()
                .p(px(30.))
                .flex()
                .flex_col()
                .text_color(rgb(theme::TEXT))
                .child(body)
                .when(!buttons.is_empty(), |d| d.child(div().mt(px(20.)).flex().flex_wrap().gap(px(10.)).justify_end().children(buttons))),
        )
}

/// Dims the window and centres the card; clicks do not reach what is below.
pub fn backdrop(card: Div) -> Stateful<Div> {
    div()
        .id("dialog-backdrop")
        .absolute()
        .inset_0()
        .occlude()
        .p_4()
        .flex()
        .items_center()
        .justify_center()
        .child(motion::scrim(0x00000099))
        .child(motion::modal(
            div()
                .id("dialog-scroll")
                .w(px(520.))
                .max_w_full()
                .max_h_full()
                .min_h_0()
                .overflow_y_scroll()
                .rounded_lg()
                .bg(rgb(theme::BACKGROUND))
                .shadow_lg()
                .child(card),
        ))
}

/// Slides a right-hand editor panel in from the window edge, like the legacy
/// sliders. The animation runs once, when the panel first appears.
pub fn slide_in(panel: Stateful<Div>, width: f32) -> AnyElement {
    motion::slide_in(panel, "slide-in", width)
}

/// Text with line breaks kept, like the legacy `pre-line` dialog text.
pub fn text(content: &str) -> Div {
    div().flex().flex_col().gap_1().children(content.lines().map(|line| div().child(line.to_owned())))
}

pub fn checkbox(id: &str, label: &str, checked: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .size(px(16.))
                .rounded_sm()
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(rgb(if checked { theme::ACCENT } else { theme::TEXT_MUTED }))
                .when(checked, |d| d.bg(rgb(theme::ACCENT)))
                .when(checked, |d| d.child(svg().path(icon("check")).size(px(10.)).text_color(rgb(0xffffff)))),
        )
        .child(label.to_owned())
}

/// Labelled text field with the validation message under it.
pub fn field<T: 'static>(label: &str, value: &str, error: Option<&str>, focus: &FocusHandle, focused: bool, cx: &Context<T>) -> Div {
    use super::text_input::{self, Mode};
    let input = div().id("dialog-input").h(px(40.)).px_3().flex().items_center()
        .rounded_md().bg(rgb(theme::INPUT)).border_1()
        .border_color(rgb(match (error.is_some(), focused) {
            (true, _) => theme::ERROR,
            (false, true) => theme::ACCENT,
            (false, false) => theme::INPUT,
        }))
        .child(text_input::text("dialog-input", value, "", focused, Mode::Line, cx));
    let focus = focus.clone();
    div().flex().flex_col().gap_1()
        .child(div().text_xs().text_color(rgb(if error.is_some() { theme::ERROR } else { theme::TEXT_MUTED })).child(label.to_owned()))
        .child(text_input::field(input, "dialog-input", Mode::Line, move |_, window, _| window.focus(&focus), cx))
        .child(div().h(px(18.)).text_xs().text_color(rgb(theme::ERROR)).child(error.unwrap_or("").to_owned()))
}

pub enum Key {
    Edited,
    Enter,
    Escape,
    Ignored,
}
