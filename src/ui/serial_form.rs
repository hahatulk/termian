//! The legacy «Serial» tab before it connects: Serial Port (the system's
//! ports, refreshed every 3 s, the first one picked), Baud rate with its
//! suggestions, «Advanced» (Charset, Data bits, Stop bits, Flow Control,
//! Parity), Close and Connect (Enter).

use std::time::Duration;

use gpui::{div, prelude::*, px, rgb, svg, Context, Div, FontWeight, KeyDownEvent, SharedString, Stateful, Window};
use termian_native::serial::{self, FlowControl, Parity, SerialOptions, BAUD_RATES, DATA_BITS, STOP_BITS};
use termian_native::session_log::SessionLog;
use termian_native::theme;

use crate::ui::pointer::PointerFeedback;
use super::{TerminalEvent, TerminalView};
use super::text_input::{self, Mode};
use crate::ui::assets::icon;
use crate::ui::dialog::{self, Key, Kind};
use crate::ui::host_editor::menu_row;

const WIDTH: f32 = 420.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Path,
    Baud,
    Charset,
}

pub struct SerialForm {
    options: SerialOptions,
    ports: Vec<String>,
    field: Field,
    baud_text: String,
    charset_text: String,
    advanced: bool,
    error: Option<String>,
}

impl SerialForm {
    pub fn new() -> Self {
        let options = SerialOptions::default();
        Self { baud_text: options.baud_rate.to_string(), charset_text: options.charset.clone(), options, ports: Vec::new(), field: Field::Path, advanced: false, error: None }
    }

    fn set_ports(&mut self, ports: Vec<String>) {
        if self.options.path.is_empty() {
            if let Some(first) = ports.first() {
                self.options.path = first.clone();
            }
        }
        self.ports = ports;
    }
}

impl TerminalView {
    /// Lists the ports now and every 3 s while the form is shown.
    pub(super) fn watch_serial_ports(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| loop {
            let ports = cx.background_spawn(async { serial::list_ports() }).await;
            let open = this.update(cx, |view, cx| match view.serial_form.as_mut() {
                Some(form) => {
                    form.set_ports(ports);
                    cx.notify();
                    true
                }
                None => false,
            });
            if !matches!(open, Ok(true)) {
                break;
            }
            cx.background_executor().timer(Duration::from_secs(3)).await;
        })
        .detach();
    }

    /// Opens the port chosen in the form; the window then records the session.
    fn start_serial(&mut self, options: SerialOptions, window: &mut Window, cx: &mut Context<Self>) {
        match termian_native::session::serial(&options) {
            Ok(session) => {
                self.serial_form = None;
                self.generation += 1;
                self.session = session;
                Self::listen(&mut self.session, self.generation, window, cx);
                self.closed = false;
                self.terminal.feed(format!("\x1b[2mConnection to \"{}\" established\x1b[0m\r\n", options.path).as_bytes());
                window.focus(&self.focus);
                cx.emit(TerminalEvent::SerialConnected(options.label()));
            }
            Err(error) => {
                if let Some(form) = self.serial_form.as_mut() {
                    form.error = Some(error);
                }
            }
        }
        cx.notify();
    }

    /// The session log the window started for this connection.
    pub fn set_log(&mut self, log: Option<SessionLog>) {
        self.log = log;
    }

    fn serial_connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.serial_form.as_mut() else { return };
        if let Ok(rate) = form.baud_text.trim().parse() {
            form.options.baud_rate = rate;
        }
        form.options.charset = serial::charset_for(&form.charset_text).to_owned();
        form.charset_text = form.options.charset.clone();
        if !form.options.can_connect() {
            return;
        }
        form.error = None;
        let options = form.options.clone();
        self.start_serial(options, window, cx);
    }

    pub(super) fn on_serial_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.serial_form.as_mut() else { return };
        let value = match form.field {
            Field::Path => &mut form.options.path,
            Field::Baud => &mut form.baud_text,
            Field::Charset => &mut form.charset_text,
        };
        match text_input::edit(("serial", form.field), Some(value), &event.keystroke, Mode::Line, cx) {
            Key::Enter => self.serial_connect(window, cx),
            Key::Escape => cx.emit(TerminalEvent::Close),
            Key::Edited => {
                if form.field == Field::Baud {
                    form.baud_text.retain(|c| c.is_ascii_digit());
                    form.options.baud_rate = form.baud_text.parse().unwrap_or(0);
                }
                if form.field == Field::Path {
                    form.options.path = form.options.path.trim().to_owned();
                }
            }
            Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn serial_field(&self, field: Field, label: &str, value: &str, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let form = self.serial_form.as_ref().expect("form is shown");
        let focused = form.field == field && self.login_focus.is_focused(window);
        let key = ("serial", field);
        let input = div().id(SharedString::from(format!("serial-input-{label}")))
            .w(px(WIDTH)).h(px(40.)).px_3().flex().items_center().rounded_md()
            .bg(rgb(theme::INPUT)).border_1().border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
            .child(div().flex_1().child(text_input::text(key, value, "", focused, Mode::Line, cx)))
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)));
        let input = text_input::field(input, key, Mode::Line, move |view, window, _| {
            if let Some(form) = view.serial_form.as_mut() {
                form.field = field;
            }
            window.focus(&view.login_focus);
        }, cx);
        div().id(SharedString::from(format!("serial-{label}"))).flex().flex_col().gap_1()
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(label.to_owned()))
            .child(input)
    }

    /// Suggestions under the focused field.
    fn serial_suggestions(&self, window: &Window, cx: &mut Context<Self>) -> Option<Div> {
        let form = self.serial_form.as_ref()?;
        if !self.login_focus.is_focused(window) {
            return None;
        }
        let items: Vec<String> = match form.field {
            Field::Path => form.ports.clone(),
            Field::Baud => BAUD_RATES.iter().map(u32::to_string).collect(),
            Field::Charset => serial::charsets().into_iter().map(str::to_owned).collect(),
        };
        if items.is_empty() {
            return None;
        }
        let field = form.field;
        let mut list = div().id("serial-suggestions").w(px(WIDTH)).max_h(px(220.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::SURFACE)).border_1().border_color(rgb(theme::SURFACE_ACTIVE));
        for (i, item) in items.into_iter().enumerate() {
            let value = item.clone();
            list = list.child(menu_row(format!("serial-choice-{i}"), &item, false, false).on_click(cx.listener(move |view, _, _, cx| {
                if let Some(form) = view.serial_form.as_mut() {
                    match field {
                        Field::Path => form.options.path = value.clone(),
                        Field::Baud => {
                            form.baud_text = value.clone();
                            form.options.baud_rate = value.parse().unwrap_or(0);
                        }
                        Field::Charset => {
                            form.charset_text = value.clone();
                            form.options.charset = value.clone();
                        }
                    }
                }
                cx.notify();
            })));
        }
        Some(div().mt_1().child(list))
    }

    fn serial_choice<T: Copy + PartialEq + 'static>(&self, id: &str, label: &str, choices: &[(T, String)], current: T, set: fn(&mut SerialOptions, T), cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().gap_1();
        for (i, (value, text)) in choices.iter().enumerate() {
            let value = *value;
            let active = value == current;
            row = row.child(
                div()
                    .id(SharedString::from(format!("{id}-{i}")))
                    .h(px(30.))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_md()
                    .bg(rgb(if active { theme::ACCENT } else { theme::INPUT }))
                    .surface_feedback(if active { theme::ACCENT } else { theme::INPUT })
                    .text_color(rgb(if active { 0xffffff } else { theme::TEXT }))
                    .child(text.clone())
                    .on_click(cx.listener(move |view, _, _, cx| {
                        if let Some(form) = view.serial_form.as_mut() {
                            set(&mut form.options, value);
                        }
                        cx.notify();
                    })),
            );
        }
        div().flex().flex_col().gap_1().child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(label.to_owned())).child(row)
    }

    pub(super) fn render_serial_form(&self, window: &Window, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let form = self.serial_form.as_ref()?;
        let options = &form.options;
        let suggestions = self.serial_suggestions(window, cx);
        let at = |field: Field| (form.field == field).then_some(()).and(suggestions.as_ref()).is_some();
        let (path_list, baud_list, charset_list) = (at(Field::Path), at(Field::Baud), at(Field::Charset));
        let mut suggestions = suggestions;
        let mut body = div().flex().flex_col().gap(px(14.));
        body = body.child(div().w_full().child(self.serial_field(Field::Path, "Serial Port", &options.path, window, cx)).children(path_list.then(|| suggestions.take()).flatten()));
        body = body.child(div().w_full().child(self.serial_field(Field::Baud, "Baud rate", &form.baud_text, window, cx)).children(baud_list.then(|| suggestions.take()).flatten()));
        let advanced = div()
            .id("serial-advanced")
            .w(px(WIDTH))
            .p(px(10.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .surface_feedback(theme::SURFACE)
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::TEXT))
                    .child(div().flex_1().child("Advanced"))
                    .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)).when(!form.advanced, |d| d.with_transformation(gpui::Transformation::rotate(gpui::radians(-std::f32::consts::FRAC_PI_2))))),
            )
            .on_click(cx.listener(|view, _, _, cx| {
                if let Some(form) = view.serial_form.as_mut() {
                    form.advanced = !form.advanced;
                }
                cx.notify();
            }));
        let advanced = if form.advanced {
            let data: Vec<(u8, String)> = DATA_BITS.iter().map(|b| (*b, b.to_string())).collect();
            let stop: Vec<(u8, String)> = STOP_BITS.iter().map(|b| (*b, b.to_string())).collect();
            let flow: Vec<(FlowControl, String)> = FlowControl::ALL.iter().map(|f| (*f, f.label().to_owned())).collect();
            let parity: Vec<(Parity, String)> = Parity::ALL.iter().map(|p| (*p, p.label().to_owned())).collect();
            advanced
                .child(div().w_full().child(self.serial_field(Field::Charset, "Charset", &form.charset_text, window, cx)).children(charset_list.then(|| suggestions.take()).flatten()))
                .child(self.serial_choice("serial-data", "Data bits", &data, options.data_bits, |o, v| o.data_bits = v, cx))
                .child(self.serial_choice("serial-stop", "Stop bits", &stop, options.stop_bits, |o, v| o.stop_bits = v, cx))
                .child(self.serial_choice("serial-flow", "Flow Control", &flow, options.flow_control, |o, v| o.flow_control = v, cx))
                .child(self.serial_choice("serial-parity", "Parity", &parity, options.parity, |o, v| o.parity = v, cx))
        } else {
            advanced
        };
        body = body.child(advanced);
        let error = form.error.clone().map(|e| div().text_sm().text_color(rgb(theme::ERROR)).child(e));
        let disabled = !options.can_connect();
        let actions = div()
            .flex()
            .gap(px(10.))
            .child(dialog::button("serial-close", "Close", Kind::Regular, false).on_click(cx.listener(|_, _, _, cx| cx.emit(TerminalEvent::Close))))
            .child(dialog::button("serial-connect", "Connect", Kind::Accent, disabled).when(!disabled, |d| d.on_click(cx.listener(|view, _, window, cx| view.serial_connect(window, cx)))));
        Some(
            div()
                .id("serial-form")
                .track_focus(&self.login_focus)
                .on_key_down(cx.listener(Self::on_serial_key))
                .size_full()
                .overflow_y_scroll()
                .bg(rgb(theme::BACKGROUND))
                .flex()
                .justify_center()
                .child(div().w(px(WIDTH)).pt(px(60.)).pb(px(30.)).flex().flex_col().gap(px(20.)).text_sm().child(body).children(error).child(actions)),
        )
    }
}
