//! Connection screen shown while an SSH login runs, as in the legacy client:
//! host header, progress (connect, the host key point while it is checked,
//! terminal), the pending question and the connection log.

use std::time::Duration;

use gpui::{
    div, percentage, prelude::*, px, rgb, svg, Animation, AnimationExt, AnyElement, Context, Div, FocusHandle, KeyDownEvent, SharedString, Stateful,
    Transformation, Window,
};
use termian_native::login::{Answer, HostKeyDecision, IdentityChoice, LoginState, LoginUi, Question, Stage};
use termian_native::theme;

use super::text_input::{self, Mode};
use super::assets::icon;
use super::dialog::{self, Kind};
use super::hosts::os_tile;
use super::pointer::PointerFeedback;

/// Blue of the progress bar in the legacy screen.
const PROGRESS: u32 = 0x3d8af7;
/// Red of a failed connection: bar, points and the log heading.
const FAILED: u32 = 0xe5484d;
const WIDTH: f32 = 384.;
/// Diameter of a progress point.
const STEP: f32 = 24.;

pub struct LoginTarget<'a> {
    pub label: &'a str,
    /// `SSH 192.168.1.180:22`
    pub subtitle: String,
    pub os: Option<&'a str>,
    /// Offered on the Username question; empty hides the list.
    pub identities: &'a [IdentityChoice],
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
    /// «Edit host» after a failure, for saved hosts.
    pub edit_host: Option<fn(&mut V, &mut Window, &mut Context<V>)>,
    pub verbose: Option<fn(&mut V, &mut Window, &mut Context<V>)>,
    /// «Continue & Save» on the Username question: keep the typed username
    /// in the host, then continue.
    pub continue_save: Option<fn(&mut V, &mut Window, &mut Context<V>)>,
    /// Identity search and the Continue dropdown; `None` without identities.
    pub ui: Option<fn(&mut V, LoginUi, &mut Context<V>)>,
}

fn button(id: &str, label: &str, primary: bool, enabled: bool) -> Stateful<Div> {
    let background = if primary { CONNECTED } else { theme::SURFACE_ACTIVE };
    div()
        .id(SharedString::from(id.to_owned()))
        .px_4()
        .h(px(36.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(0xffffff))
        .bg(rgb(background))
        .when(enabled, |d| d.surface_feedback(background))
        .child(label.to_owned())
}

/// A progress point; the active one is circled by a spinning arc, as in the
/// legacy connection screen.
fn step(name: &str, color: u32, active: bool) -> Div {
    div()
        .size(px(STEP))
        .flex_none()
        .relative()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(color))
        .child(svg().path(icon(name)).size(px(12.)).text_color(rgb(0xffffff)))
        .when(active, |d| {
            d.child(
                div().absolute().top(px(-6.)).left(px(-6.)).size(px(STEP + 12.)).child(
                    svg().path(icon("spinner_arc")).size(px(STEP + 12.)).text_color(rgb(PROGRESS)).with_animation(
                        SharedString::from(format!("step-spin-{name}")),
                        Animation::new(Duration::from_millis(900)).repeat(),
                        |arc, delta| arc.with_transformation(Transformation::rotate(percentage(delta))),
                    ),
                ),
            )
        })
}

fn paragraph(text: impl Into<SharedString>) -> Div {
    div().text_color(rgb(theme::TEXT)).child(text.into())
}

/// A log line with the legacy marker: ⚙️ for `debug1:` lines, 👤 otherwise.
fn log_line(line: &str) -> Div {
    let marker = if line.starts_with("debug1:") { "⚙️" } else { "👤" };
    div().child(format!("{marker} {line}"))
}

/// Text field for passwords and other typed answers.
fn input_field(id: &str, focus: &FocusHandle, focused: bool) -> Stateful<Div> {
    div().id(SharedString::from(id.to_owned())).track_focus(focus).w_full().h(px(40.)).px_3()
        .flex().items_center().rounded_md().bg(rgb(theme::INPUT)).border_1()
        .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
}

pub fn edit<V: 'static>(state: &mut LoginState, event: &KeyDownEvent, cx: &mut Context<V>) -> Option<Option<Answer>> {
    use super::dialog::Key;
    let Some(Question::Text { echo, .. }) = state.question.as_ref() else { return None };
    if state.searching_identities {
        return match text_input::edit("login-identities", Some(&mut state.identity_query), &event.keystroke, Mode::Line, cx) {
            Key::Enter | Key::Escape => {
                state.searching_identities = false;
                Some(None)
            }
            Key::Edited => Some(None),
            Key::Ignored => None,
        };
    }
    let mode = if *echo { Mode::Line } else { Mode::Secret };
    match text_input::edit("login-input", Some(&mut state.input), &event.keystroke, mode, cx) {
        Key::Enter => Some(state.submit_text()),
        Key::Escape => Some(state.answer(Answer::Text(None))),
        Key::Edited => Some(None),
        Key::Ignored => None,
    }
}

/// The progress bar. Past the connect stage a point (the fingerprint while
/// the host key is checked) slides from the start to its place with the blue
/// fill following it; the arc spins until a question or error stops it. A
/// failure paints the whole bar red.
fn progress_bar(state: &LoginState) -> Div {
    if state.error.is_some() {
        return div().h(px(STEP)).flex().items_center()
            .child(step("connect", FAILED, false))
            .child(div().flex_1().min_w_0().h(px(3.)).bg(rgb(FAILED)))
            .child(step("terminal", FAILED, false));
    }
    let track = WIDTH - STEP;
    let place = |d: Div, x: f32| d.absolute().top_0().left(px(x));
    let line = || div().absolute().top(px((STEP - 3.) / 2.)).left(px(STEP / 2.)).h(px(3.));
    let bar = div().relative().w(px(WIDTH)).h(px(STEP));
    let progress = state.progress();
    let spinning = state.question.is_none();
    let head = match state.stage {
        _ if state.asks_username() => Some("person"),
        Stage::Connecting => None,
        Stage::Verifying => Some("known_hosts"),
        _ => Some("connect"),
    };
    let mut bar = bar
        .child(line().w(px(track)).bg(rgb(theme::SURFACE_ACTIVE)))
        .child(place(step("terminal", if progress >= 1. { PROGRESS } else { theme::SURFACE_ACTIVE }, false), track))
        .child(place(step("connect", PROGRESS, spinning && head.is_none()), 0.));
    if let Some(name) = head {
        let target = (progress.min(1.) * track).min(track / 2.);
        let slide = || Animation::new(Duration::from_millis(600)).with_easing(gpui::ease_in_out);
        bar = bar
            .child(line().bg(rgb(PROGRESS)).with_animation(SharedString::from(format!("login-fill-{name}")), slide(), move |d, t| d.w(px(t * target))))
            .child(place(step(name, PROGRESS, spinning), 0.).with_animation(
                SharedString::from(format!("login-head-{name}")),
                slide(),
                move |d, t| d.left(px(t * target)),
            ));
    }
    bar
}

/// Green of the legacy bar once the shell is ready.
const CONNECTED: u32 = 0x4cae63;
/// The legacy hand-over to the terminal: the bar fills green, then the
/// screen fades away over the shell.
pub const CONNECTED_FILL: Duration = Duration::from_millis(300);
pub const CONNECTED_TOTAL: Duration = Duration::from_millis(550);

/// The connection screen as the shell becomes ready, laid over the terminal.
pub fn render_connected(label: &str, subtitle: String, os: Option<&str>, key: u64) -> AnyElement {
    let track = WIDTH - STEP;
    let fill = CONNECTED_FILL.as_secs_f32() / CONNECTED_TOTAL.as_secs_f32();
    let header = div()
        .w_full()
        .flex()
        .items_center()
        .gap_3()
        .child(os_tile(os).size(px(40.)))
        .child(div().flex_1().flex().flex_col().child(div().text_color(rgb(theme::TEXT)).child(label.to_owned())).child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(subtitle)))
        .child(button("connected-logs", "Show logs", false, false));
    let line = || div().absolute().top(px((STEP - 3.) / 2.)).left(px(STEP / 2.)).h(px(3.));
    let bar = div()
        .relative()
        .w(px(WIDTH))
        .h(px(STEP))
        .child(line().w(px(track)).bg(rgb(theme::SURFACE_ACTIVE)))
        .child(line().bg(rgb(CONNECTED)).with_animation(SharedString::from(format!("connected-fill-{key}")), Animation::new(CONNECTED_TOTAL), move |d, t| d.w(px((t / fill).min(1.) * track))))
        .child(div().absolute().top_0().left_0().child(step("connect", CONNECTED, false)))
        .child(div().absolute().top_0().left(px(track)).child(step("terminal", CONNECTED, false)));
    div()
        .absolute()
        .inset_0()
        .flex()
        .justify_center()
        .bg(rgb(theme::TITLE_BAR))
        .child(div().w(px(WIDTH)).pt(px(110.)).flex().flex_col().child(header).child(div().w_full().pt_5().child(bar)))
        .with_animation(SharedString::from(format!("connected-out-{key}")), Animation::new(CONNECTED_TOTAL), move |d, t| d.opacity(if t < fill { 1. } else { 1. - (t - fill) / (1. - fill) }))
        .into_any_element()
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

    // After a failure the log is always shown and the header button copies it.
    let header_button = if let Some(error) = &state.error {
        let text = state.logs.iter().chain(std::iter::once(error)).cloned().collect::<Vec<_>>().join("\n");
        button("copy-logs", "Copy logs", false, true)
            .flex_none()
            .on_click(move |_, _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone())))
    } else {
        button("show-logs", if state.show_logs { "Hide logs" } else { "Show logs" }, false, true)
            .on_click(cx.listener(move |view, _, _, cx| toggle_logs(view, cx)))
    };
    let header = div()
        .w_full()
        .flex()
        .items_center()
        .gap_3()
        .child(os_tile(target.os).size(px(40.)).flex_none())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().text_color(rgb(theme::TEXT)).child(target.label.to_owned()))
                .child(div().truncate().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(target.subtitle.clone())),
        )
        .when(!state.asks_username(), |d| d.child(header_button));

    let bar = progress_bar(state).mt_5();

    let log_box = |extra: Option<&String>| {
        div()
            .id("login-logs")
            .w_full()
            .max_h(px(260.))
            .overflow_y_scroll()
            .px(px(24.))
            .py(px(20.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .text_sm()
            .text_color(rgb(theme::TEXT))
            .flex()
            .flex_col()
            .gap_1()
            .children(state.logs.iter().map(|line| log_line(line)))
            .children(extra.map(|error| div().child(format!("😨 {error}"))))
    };
    let close_button = |id: &str| button(id, "Close", false, true).on_click(cx.listener(move |view, _, window, cx| close(view, window, cx)));
    let start_over = || button("login-retry", "Start over", true, true).on_click(cx.listener(move |view, _, window, cx| retry(view, window, cx)));

    let mut body = div().w_full().pt_6().flex().flex_col().gap_4();
    let title = |text: &str| div().text_lg().font_weight(gpui::FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT_MUTED)).child(text.to_owned());

    if let Some(error) = &state.error {
        if let Some(info) = &state.changed_key {
            body = body
                .child(title("Host key has changed"))
                .child(paragraph(format!("The host key of {} is different from the one saved in known hosts.", info.host)))
                .child(paragraph(format!("{} fingerprint is SHA256:", info.algorithm)))
                .child(paragraph(info.fingerprint.clone()))
                .child(paragraph("The connection was refused to protect you from a possible man-in-the-middle attack."));
        } else {
            body = body
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(FAILED))
                        .child("Connection failed with connection log:"),
                )
                .child(log_box(Some(error)));
        }
        let main_actions = div().flex().gap_3()
            .child(close_button("login-close").flex_1().min_w_0().h(px(42.)).px_2())
            .child(button("login-retry", "Retry", true, true)
                .flex_1().min_w_0().h(px(42.)).px_2().gap_2().flex_row_reverse()
                .child(svg().path(icon("sync")).size(px(14.)).flex_none().text_color(rgb(0xffffff)))
                .on_click(cx.listener(move |view, _, window, cx| retry(view, window, cx))));
        let secondary = |id, label| dialog::button(id, label, Kind::Text, false)
            .h(px(32.)).px_2().text_sm().text_color(rgb(theme::TEXT_MUTED));
        let mut footer = div().flex().flex_col().gap_1().child(main_actions);
        if actions.edit_host.is_some() || actions.verbose.is_some() {
            let secondary_actions = div().flex().flex_wrap().justify_center().gap_x_2()
                .children(actions.edit_host.map(|edit_host| secondary("login-edit-host", "Edit host")
                    .on_click(cx.listener(move |view, _, window, cx| edit_host(view, window, cx)))))
                .children(actions.verbose.map(|verbose| secondary("login-verbose", "Retry with verbose logs")
                    .on_click(cx.listener(move |view, _, window, cx| verbose(view, window, cx)))));
            footer = footer.child(secondary_actions);
        }
        body = body.child(footer);
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
                            .child(button("login-close", "Close", false, true).on_click(cx.listener(move |view, _, window, cx| {
                                answer(view, Answer::HostKey(HostKeyDecision::Reject), window, cx)
                            })))
                            .child(div().flex_1())
                            .child(button("login-once", "Continue", false, true).on_click(cx.listener(move |view, _, window, cx| {
                                answer(view, Answer::HostKey(HostKeyDecision::Once), window, cx)
                            })))
                            .child(button("login-save", "Add and continue", true, true).on_click(cx.listener(
                                move |view, _, window, cx| answer(view, Answer::HostKey(HostKeyDecision::Save), window, cx),
                            ))),
                    );
                if state.show_logs {
                    body = body.child(log_box(None));
                }
            }
            Some(Question::Text { .. }) if state.asks_username() && actions.ui.is_some() => {
                body = body.child(username_question(state, &target, focus, window, actions, cx));
            }
            Some(Question::Text { prompt, echo }) => {
                let focused = focus.is_focused(window);
                let mode = if *echo { Mode::Line } else { Mode::Secret };
                let field = input_field("login-input", focus, focused)
                    .child(text_input::text("login-input", &state.input, "", focused, mode, cx))
                    .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| key(view, event, window, cx)));
                let handle = focus.clone();
                let field = text_input::field(field, "login-input", mode, move |_, window, _| window.focus(&handle), cx);
                body = body.child(title(prompt)).child(field).child(
                    div()
                        .flex()
                        .gap_3()
                        .child(button("login-close", "Close", false, true).on_click(cx.listener(move |view, _, window, cx| {
                            answer(view, Answer::Text(None), window, cx)
                        })))
                        .child(div().flex_1())
                        .child(button("login-submit", "Continue", true, true).on_click(cx.listener(move |view, _, window, cx| {
                            submit(view, window, cx)
                        }))),
                );
                if state.show_logs {
                    body = body.child(log_box(None));
                }
            }
            // Legacy shows only the bar and Close while connecting; with the
            // log open, the log and Close … Start over.
            None if state.show_logs => {
                body = body.child(log_box(None)).child(div().flex().child(close_button("login-cancel")).child(div().flex_1()).child(start_over()));
            }
            None => {
                body = body.child(div().flex().child(close_button("login-cancel")));
            }
        }
    }

    let failed = state.error.is_some();
    let content = div()
        .when(failed, |d| d.w_full().max_w(px(WIDTH)).min_w_0().pb_4())
        .when(!failed, |d| d.w(px(WIDTH)))
        .pt(px(110.)).flex().flex_col().child(header).child(bar).child(body).with_animation(
            "login-in",
            Animation::new(Duration::from_millis(250)).with_easing(gpui::ease_out_quint()),
            |d, t| d.opacity(t),
        );
    let screen = div().size_full().bg(rgb(theme::TITLE_BAR));
    if failed {
        screen.child(div().id("login-failure-scroll").size_full().overflow_y_scroll().px_4().flex().items_start().justify_center().child(content))
    } else {
        screen.flex().justify_center().child(content)
    }
}

/// The legacy Username question: an outlined field with a floating label,
/// «Search identities» with the saved identities as cards, and Close …
/// «Continue & Save» with a dropdown holding plain «Continue».
fn username_question<V: 'static>(
    state: &LoginState,
    target: &LoginTarget,
    focus: &FocusHandle,
    window: &Window,
    actions: &LoginActions<V>,
    cx: &mut Context<V>,
) -> Div {
    let (answer, key, submit) = (actions.answer, actions.key, actions.submit);
    let ui = actions.ui.expect("username question needs the ui action");
    let focused = focus.is_focused(window);
    let typing = focused && !state.searching_identities;

    let field = div()
        .id("login-username")
        .track_focus(focus)
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| key(view, event, window, cx)))
        .relative()
        .w_full()
        .h(px(40.))
        .px_3()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(if typing { theme::ACCENT } else { theme::SURFACE_ACTIVE }))
        .text_color(rgb(theme::TEXT))
        .child(
            div()
                .absolute()
                .top(px(-8.))
                .left(px(8.))
                .px_1()
                .bg(rgb(theme::TITLE_BAR))
                .text_xs()
                .text_color(rgb(if typing { theme::ACCENT } else { theme::TEXT_MUTED }))
                .child("Username"),
        )
        .child(text_input::text("login-input", &state.input, "", typing, Mode::Line, cx));
    let handle = focus.clone();
    let field = text_input::field(field, "login-input", Mode::Line, move |view, window, cx| {
        ui(view, LoginUi::TypeUsername, cx);
        window.focus(&handle);
    }, cx);

    let searching = focused && state.searching_identities;
    let search = div()
        .id("login-identity-search")
        .h(px(32.))
        .flex()
        .items_center()
        .gap_2()
        .text_sm()
        .child(svg().path(icon("search")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
        .child(text_input::text("login-identities", &state.identity_query, "Search identities", searching, Mode::Line, cx));
    let handle = focus.clone();
    let search = text_input::field(search, "login-identities", Mode::Line, move |view, window, cx| {
        ui(view, LoginUi::SearchIdentities, cx);
        window.focus(&handle);
    }, cx);

    let cards = target.identities.iter().enumerate().filter(|(_, choice)| choice.matches(&state.identity_query)).map(|(index, choice)| {
        let picked = choice.answer.clone();
        div()
            .id(SharedString::from(format!("login-identity-{index}")))
            .h(px(56.))
            .px_3()
            .flex()
            .items_center()
            .gap_3()
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .hover(|d| d.bg(rgb(theme::SURFACE_ACTIVE)))
            .child(
                div()
                    .size(px(32.))
                    .flex_none()
                    .rounded_md()
                    .bg(rgb(theme::TILE_BLUE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(svg().path(icon("identity")).size(px(16.)).text_color(rgb(0xffffff))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_color(rgb(theme::TEXT)).child(choice.label.clone()))
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(choice.subtitle)),
            )
            .on_click(cx.listener(move |view, _, window, cx| answer(view, picked.clone(), window, cx)))
    });

    let empty = state.input.trim().is_empty();
    let primary = |d: Stateful<Div>| d.when(empty, |d| d.opacity(0.4));
    let continue_button = match actions.continue_save {
        Some(continue_save) => div()
            .relative()
            .flex()
            .child(
                primary(button("login-continue-save", "Continue & Save", true, !empty))
                    .rounded_r_none()
                    .when(!empty, |d| d.on_click(cx.listener(move |view, _, window, cx| continue_save(view, window, cx)))),
            )
            .child(
                primary(button("login-continue-menu", "", true, !empty))
                    .px_2()
                    .rounded_l_none()
                    .border_l_1()
                    .border_color(rgb(0x3d8c50))
                    .child(svg().path(icon("caret_down")).size(px(12.)).text_color(rgb(0xffffff)))
                    .when(!empty, |d| d.on_click(cx.listener(move |view, _, _, cx| ui(view, LoginUi::ToggleContinueMenu, cx)))),
            )
            .when(state.continue_menu && !empty, |d| {
                d.child(
                    div()
                        .absolute()
                        .bottom_full()
                        .right_0()
                        .mb_1()
                        .p_1()
                        .rounded_md()
                        .bg(rgb(theme::DIALOG_ROW))
                        .shadow_lg()
                        .child(
                            div()
                                .id("login-continue")
                                .px_3()
                                .py_1()
                                .rounded_sm()
                                .text_sm()
                                .text_color(rgb(theme::TEXT))
                                .hover(|d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                                .child("Continue")
                                .on_click(cx.listener(move |view, _, window, cx| submit(view, window, cx))),
                        ),
                )
            }),
        None => div().child(
            primary(button("login-submit", "Continue", true, !empty)).when(!empty, |d| d.on_click(cx.listener(move |view, _, window, cx| submit(view, window, cx)))),
        ),
    };

    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(field)
        .when(!target.identities.is_empty(), |d| d.child(search).child(div().flex().flex_col().gap_2().children(cards)))
        .child(
            div()
                .pt_4()
                .flex()
                .items_center()
                .child(button("login-close", "Close", false, true).on_click(cx.listener(move |view, _, window, cx| answer(view, Answer::Text(None), window, cx))))
                .child(div().flex_1())
                .child(continue_button),
        )
}
