//! Connection screen shown while an SSH login runs, as in the legacy client:
//! host header, three-step progress (connect, host key, terminal), the
//! pending question and an optional log. Shared by terminal and SFTP tabs.

use gpui::{div, prelude::*, px, rgb, svg, Context, Div, FocusHandle, KeyDownEvent, SharedString, Stateful, Window};
use termian_native::login::{Answer, HostKeyDecision, LoginState, Question};
use termian_native::theme;

use super::assets::icon;
use super::hosts::os_tile;

/// Blue of the progress bar in the legacy screen.
const PROGRESS: u32 = 0x3d8af7;
const WIDTH: f32 = 490.;

pub struct LoginTarget<'a> {
    pub label: &'a str,
    /// `SSH 192.168.1.180:22`
    pub subtitle: String,
    pub os: Option<&'a str>,
}

/// What the screen's buttons do in the owning view.
pub struct LoginActions<V: 'static> {
    pub answer: fn(&mut V, Answer, &mut Window, &mut Context<V>),
    pub close: fn(&mut V, &mut Window, &mut Context<V>),
    pub retry: fn(&mut V, &mut Window, &mut Context<V>),
    pub toggle_logs: fn(&mut V, &mut Context<V>),
    pub key: fn(&mut V, &KeyDownEvent, &mut Window, &mut Context<V>),
    /// Sends the typed answer (the Continue button of a text question).
    pub submit: fn(&mut V, &mut Window, &mut Context<V>),
}

fn button(id: &str, label: &str, primary: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .px_4()
        .h(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
        .bg(rgb(if primary { 0x4cae63 } else { theme::SURFACE_ACTIVE }))
        .child(label.to_owned())
}

fn step(name: &str, done: bool) -> Div {
    div()
        .size(px(28.))
        .flex_none()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(if done { PROGRESS } else { theme::SURFACE_ACTIVE }))
        .child(svg().path(icon(name)).size(px(14.)).text_color(rgb(0xffffff)))
}

fn segment(done: bool) -> Div {
    div().flex_1().h(px(4.)).bg(rgb(if done { PROGRESS } else { theme::SURFACE_ACTIVE }))
}

fn paragraph(text: impl Into<SharedString>) -> Div {
    div().text_color(rgb(theme::TEXT)).child(text.into())
}

/// Text field for passwords and other typed answers.
pub fn input_field(id: &str, shown: String, focused: bool, focus: &FocusHandle) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .track_focus(focus)
        .w_full()
        .h(px(40.))
        .px_3()
        .flex()
        .items_center()
        .rounded_md()
        .bg(rgb(theme::INPUT))
        .border_1()
        .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
        .text_color(rgb(theme::TEXT))
        .child(shown)
        .when(focused, |d| d.child(div().mx(px(1.)).w(px(1.5)).h(px(16.)).bg(rgb(theme::TEXT))))
}

/// Applies a key press to the typed answer; returns an answer to send on Enter/Escape.
pub fn edit(state: &mut LoginState, event: &KeyDownEvent, paste: Option<String>) -> Option<Option<Answer>> {
    let keystroke = &event.keystroke;
    if !matches!(state.question, Some(Question::Text { .. })) {
        return None;
    }
    match keystroke.key.as_str() {
        _ if keystroke.modifiers.platform && keystroke.key == "v" => {
            state.input.push_str(paste?.lines().next().unwrap_or(""));
        }
        _ if keystroke.modifiers.platform || keystroke.modifiers.control => return None,
        "backspace" => {
            state.input.pop();
        }
        "enter" => return Some(state.submit_text()),
        "escape" => return Some(state.answer(Answer::Text(None))),
        _ => state.input.push_str(keystroke.key_char.as_deref().filter(|t| !t.chars().any(char::is_control))?),
    }
    Some(None)
}

pub fn render<V: 'static>(
    target: LoginTarget,
    state: &LoginState,
    focus: &FocusHandle,
    window: &Window,
    actions: &LoginActions<V>,
    cx: &mut Context<V>,
) -> Div {
    let (answer, close, retry, toggle_logs, key, submit) =
        (actions.answer, actions.close, actions.retry, actions.toggle_logs, actions.key, actions.submit);
    let progress = state.progress();

    let header = div()
        .w_full()
        .flex()
        .items_center()
        .gap_3()
        .child(os_tile(target.os).size(px(52.)))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(div().text_color(rgb(theme::TEXT)).child(target.label.to_owned()))
                .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(target.subtitle)),
        )
        .child(
            button("show-logs", if state.show_logs { "Hide logs" } else { "Show logs" }, false)
                .on_click(cx.listener(move |view, _, _, cx| toggle_logs(view, cx))),
        );

    let bar = div()
        .w_full()
        .pt_6()
        .flex()
        .items_center()
        .child(step("connect", true))
        .child(segment(progress >= 0.5))
        .child(step("known_hosts", progress >= 0.5))
        .child(segment(progress >= 1.))
        .child(step("terminal", progress >= 1.));

    let mut body = div().w_full().pt_8().flex().flex_col().gap_4();
    let title = |text: &str| div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT_MUTED)).child(text.to_owned());

    if let Some(error) = &state.error {
        let heading = if state.changed_key.is_some() { "Host key has changed" } else { "Connection failed" };
        body = body.child(title(heading));
        if let Some(info) = &state.changed_key {
            body = body
                .child(paragraph(format!("The host key of {} is different from the one saved in known hosts.", info.host)))
                .child(paragraph(format!("{} fingerprint is SHA256:", info.algorithm)))
                .child(paragraph(info.fingerprint.clone()))
                .child(paragraph("The connection was refused to protect you from a possible man-in-the-middle attack."));
        } else {
            body = body.child(paragraph(error.clone()));
        }
        body = body.child(
            div()
                .flex()
                .gap_3()
                .child(button("login-close", "Close", false).on_click(cx.listener(move |view, _, window, cx| close(view, window, cx))))
                .child(div().flex_1())
                .child(button("login-retry", "Retry", true).on_click(cx.listener(move |view, _, window, cx| retry(view, window, cx)))),
        );
    } else {
        match &state.question {
            Some(Question::HostKey(info)) => {
                body = body
                    .child(title("Are you sure you want to connect?"))
                    .child(paragraph(format!("The authenticity of {} can not be established.", info.host)))
                    .child(paragraph(format!("{} fingerprint is SHA256:", info.algorithm)))
                    .child(paragraph(info.fingerprint.clone()))
                    .child(paragraph("Do you want to add it to the list of known hosts?"))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(button("login-close", "Close", false).on_click(cx.listener(move |view, _, window, cx| {
                                answer(view, Answer::HostKey(HostKeyDecision::Reject), window, cx)
                            })))
                            .child(div().flex_1())
                            .child(button("login-once", "Continue", false).on_click(cx.listener(move |view, _, window, cx| {
                                answer(view, Answer::HostKey(HostKeyDecision::Once), window, cx)
                            })))
                            .child(button("login-save", "Add and continue", true).on_click(cx.listener(
                                move |view, _, window, cx| answer(view, Answer::HostKey(HostKeyDecision::Save), window, cx),
                            ))),
                    );
            }
            Some(Question::Text { prompt, .. }) => {
                let focused = focus.is_focused(window);
                let field = input_field("login-input", state.shown_input(), focused, focus)
                    .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| key(view, event, window, cx)))
                    .on_click({
                        let focus = focus.clone();
                        cx.listener(move |_, _, window, cx| {
                            window.focus(&focus);
                            cx.notify();
                        })
                    });
                body = body.child(title(prompt)).child(field).child(
                    div()
                        .flex()
                        .gap_3()
                        .child(button("login-close", "Close", false).on_click(cx.listener(move |view, _, window, cx| {
                            answer(view, Answer::Text(None), window, cx)
                        })))
                        .child(div().flex_1())
                        .child(button("login-submit", "Continue", true).on_click(cx.listener(move |view, _, window, cx| {
                            submit(view, window, cx)
                        }))),
                );
            }
            None => {
                let text = match state.stage {
                    termian_native::login::Stage::Connecting => "Connecting…",
                    termian_native::login::Stage::Verifying => "Checking the host key…",
                    _ => "Authenticating…",
                };
                body = body.child(title(text)).child(
                    div().flex().child(
                        button("login-cancel", "Close", false).on_click(cx.listener(move |view, _, window, cx| close(view, window, cx))),
                    ),
                );
            }
        }
    }

    if state.show_logs {
        body = body.child(
            div()
                .id("login-logs")
                .w_full()
                .max_h(px(220.))
                .overflow_y_scroll()
                .p_3()
                .rounded_md()
                .bg(rgb(theme::TITLE_BAR))
                .font_family("Menlo")
                .text_xs()
                .text_color(rgb(theme::TEXT_MUTED))
                .children(state.logs.iter().map(|line| div().child(line.clone()))),
        );
    }

    div()
        .size_full()
        .flex()
        .justify_center()
        .bg(rgb(theme::TITLE_BAR))
        .child(div().w(px(WIDTH)).pt(px(110.)).flex().flex_col().child(header).child(bar).child(body))
}
