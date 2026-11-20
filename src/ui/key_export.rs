//! Key Export slider, as in the legacy Keychain: the key, "Export to" host,
//! "Location – $1", "Filename – $2", the UNIX hint and the Advanced script;
//! "Export and Attach" logs in (the connection screen asks what the login
//! needs), runs the script and, on success, shows "Exported" and makes the
//! key the host's key.

use std::time::Duration;

use futures::StreamExt;
use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, FontWeight, KeyDownEvent, SharedString, Window};
use termian_native::key_export::{self, ExportEvent, KeyExport};
use termian_native::login::{Answer, HostKeyDecision, LoginEvent, LoginState};
use termian_native::session::{SshOptions, SshTarget};
use termian_native::theme;
use termian_native::vault::HostId;

use super::pointer::PointerFeedback;
use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::host_editor::{field_box, menu_row};
use super::login_view::{self, LoginActions, LoginTarget};
use super::text_input::{self, Mode};
use crate::Shell;

const WIDTH: f32 = 420.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Location,
    Filename,
    Script,
}

pub struct ExportForm {
    key: u64,
    host: Option<HostId>,
    location: String,
    filename: String,
    script: String,
    field: Field,
    hosts_open: bool,
    advanced: bool,
    /// The running export and its connection screen.
    running: Option<(KeyExport, LoginState)>,
    exported: bool,
    generation: u64,
}

impl Shell {
    /// "Export to host" in the key editor.
    pub fn export_key(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        self.key_export = Some(ExportForm {
            key,
            host: None,
            location: key_export::LOCATION.into(),
            filename: key_export::FILENAME.into(),
            script: key_export::SCRIPT.into(),
            field: Field::Location,
            hosts_open: false,
            advanced: false,
            running: None,
            exported: false,
            generation: 0,
        });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn start_key_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &mut self.key_export else { return };
        let (Some(key), Some(host)) = (self.vault.key(form.key), form.host.and_then(|h| self.vault.host(h))) else { return };
        let Ok(public_key) = key_export::public_key(key) else { return };
        let script = key_export::prepare(&form.script, &form.location, &form.filename, &key.label, &public_key);
        let mut target = SshTarget::for_host(host, &self.vault);
        target.options = SshOptions::from_vault(&self.vault);
        let mut export = KeyExport::start(target, script);
        form.generation += 1;
        let generation = form.generation;
        if let Some(mut events) = export.take_events() {
            cx.spawn_in(window, async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update_in(cx, |shell, window, cx| {
                        if shell.key_export.as_ref().map(|f| f.generation) == Some(generation) {
                            shell.on_export_event(event, window, cx);
                        }
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        form.running = Some((export, LoginState::default()));
        window.focus(&self.forward_focus);
        cx.notify();
    }

    fn on_export_event(&mut self, event: ExportEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &mut self.key_export else { return };
        let Some((_, login)) = &mut form.running else { return };
        match event {
            ExportEvent::Login(event) => {
                if matches!(event, LoginEvent::HostKey(_) | LoginEvent::Prompt { .. } | LoginEvent::HostKeyChanged(_)) {
                    window.focus(&self.forward_focus);
                }
                login.apply(event);
            }
            ExportEvent::Done(Err(reason)) => login.fail(reason),
            ExportEvent::Done(Ok(())) => {
                // "Export and Attach": the key becomes the host's (or its identity's) key.
                form.exported = true;
                let (key, host, generation) = (form.key, form.host, form.generation);
                if let Some(host) = self.vault.hosts.iter_mut().find(|h| Some(h.id) == host) {
                    match host.identity {
                        Some(identity) => {
                            if let Some(identity) = self.vault.identities.iter_mut().find(|i| i.id == identity) {
                                identity.key = Some(key);
                                identity.has_key = true;
                            }
                        }
                        None => host.key = Some(key),
                    }
                }
                self.save_vault();
                // As in legacy, "Exported" stays for a moment, then the slider closes.
                cx.spawn_in(window, async move |this, cx| {
                    cx.background_executor().timer(Duration::from_millis(2500)).await;
                    let _ = this.update(cx, |shell, cx| {
                        if shell.key_export.as_ref().map(|f| f.generation) == Some(generation) {
                            shell.key_export = None;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    fn export_answer(&mut self, answer: Answer) {
        let Some(form) = &mut self.key_export else { return };
        let cancels = matches!(answer, Answer::Text(None) | Answer::HostKey(HostKeyDecision::Reject));
        if let Some((export, login)) = &mut form.running {
            if let Some(answer) = login.answer(answer) {
                export.answer(answer);
            }
        }
        if cancels {
            form.running = None;
        }
    }

    fn on_export_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &mut self.key_export else { return };
        let text = match form.field {
            Field::Location => &mut form.location,
            Field::Filename => &mut form.filename,
            Field::Script => &mut form.script,
        };
        match text_input::edit(("export", form.field), Some(text), &event.keystroke, if form.field == Field::Script { Mode::Multiline } else { Mode::Line }, cx) {
            Key::Edited => {}
            Key::Escape => self.key_export = None,
            Key::Enter => return self.start_key_export(window, cx),
            Key::Ignored => {
                if event.keystroke.key != "tab" {
                    return;
                }
                form.field = match form.field {
                    Field::Location => Field::Filename,
                    Field::Filename if form.advanced => Field::Script,
                    _ => Field::Location,
                };
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn export_input(&self, field: Field, label: &str, window: &Window, cx: &mut Context<Self>) -> Div {
        let Some(form) = &self.key_export else { return div() };
        let value = match field {
            Field::Location => &form.location,
            Field::Filename => &form.filename,
            Field::Script => &form.script,
        };
        let focused = form.field == field && self.editor_focus.is_focused(window);
        let key = ("export", field);
        let mode = if field == Field::Script { Mode::Multiline } else { Mode::Line };
        let input = field_box(focused).id(SharedString::from(format!("export-{label}")))
            .when(field == Field::Script, |d| d.h_auto().min_h(px(110.)).py_2().items_start().font_family("Menlo").text_xs())
            .child(text_input::text(key, value, "", focused, mode, cx));
        let input = text_input::field(input, key, mode, move |shell, window, _| {
            if let Some(form) = &mut shell.key_export {
                form.field = field;
                form.hosts_open = false;
            }
            window.focus(&shell.editor_focus);
        }, cx);
        div().flex().flex_col().gap_1().child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(label.to_owned())).child(input)
    }

    pub fn render_key_export(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let form = self.key_export.as_ref()?;
        let key = self.vault.key(form.key)?;
        let public_key = key_export::public_key(key);
        let host = form.host.and_then(|h| self.vault.host(h));
        let close = |shell: &mut Shell, cx: &mut Context<Shell>| {
            shell.key_export = None;
            cx.notify();
        };
        let header = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("Key Export"))
            .child(dialog::collapse_button("export-close").on_click(cx.listener(move |shell, _, _, cx| close(shell, cx))));
        let key_row = div()
            .p(px(12.))
            .flex()
            .items_center()
            .gap_3()
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .child(svg().path(icon("keychain")).size(px(20.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(div().flex().flex_col().child(div().text_sm().text_color(rgb(theme::TEXT)).child(key.label.clone())).child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(format!("Type {}", key.kind))));

        let body = match &public_key {
            Err(error) => div().flex().flex_col().gap(px(10.)).p(px(15.)).rounded_lg().border_1().border_color(rgb(theme::ERROR)).child(div().text_sm().text_color(rgb(theme::ERROR)).child(error.clone())).child(key_row),
            Ok(_) => {
                let picker = field_box(form.hosts_open)
                    .id("export-host")
                    .surface_feedback(theme::INPUT)
                    .border_dashed()
                    .child(div().flex_1().text_color(rgb(if host.is_some() { theme::TEXT } else { theme::TEXT_MUTED })).child(host.map(|h| h.display_label().to_owned()).unwrap_or_default()))
                    .child(div().text_xs().text_color(rgb(theme::ACCENT)).child("Select Host"))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(form) = &mut shell.key_export {
                            form.hosts_open = !form.hosts_open;
                        }
                        cx.notify();
                    }));
                let hosts = form.hosts_open.then(|| {
                    let mut list = div().id("export-host-list").max_h(px(240.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::INPUT));
                    for host in &self.vault.hosts {
                        let id = host.id;
                        list = list.child(menu_row(format!("export-host-{id}"), host.display_label(), false, false).on_click(cx.listener(move |shell, _, _, cx| {
                            if let Some(form) = &mut shell.key_export {
                                form.host = Some(id);
                                form.hosts_open = false;
                            }
                            cx.notify();
                        })));
                    }
                    list
                });
                let fields = div()
                    .p(px(15.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .rounded_lg()
                    .bg(rgb(theme::SURFACE))
                    .child(div().flex().flex_col().gap_1().child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Export to *")).child(picker).children(hosts))
                    .child(self.export_input(Field::Location, "Location – $1 *", window, cx))
                    .child(self.export_input(Field::Filename, "Filename – $2 *", window, cx))
                    .child(
                        div()
                            .mt(px(5.))
                            .px(px(15.))
                            .py(px(10.))
                            .rounded(px(10.))
                            .bg(rgb(theme::INPUT))
                            .text_xs()
                            .text_color(rgb(theme::TEXT))
                            .child("Key export currently supports only UNIX systems. Use Advanced section to customize the export script."),
                    );
                let advanced = div()
                    .id("export-advanced")
                    .rounded_sm().icon_feedback()
                    .flex()
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .text_sm()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(svg().path(icon("caret_down")).size(px(10.)).when(!form.advanced, |s| s.with_transformation(gpui::Transformation::rotate(gpui::percentage(-0.25)))).text_color(rgb(theme::TEXT_MUTED)))
                    .child("Advanced")
                    .on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(form) = &mut shell.key_export {
                            form.advanced = !form.advanced;
                        }
                        cx.notify();
                    }));
                div()
                    .flex()
                    .flex_col()
                    .gap(px(15.))
                    .child(key_row)
                    .child(fields)
                    .child(advanced)
                    .when(form.advanced, |d| d.child(self.export_input(Field::Script, "Script *", window, cx)))
            }
        };
        let valid = public_key.is_ok() && host.is_some() && !form.location.trim().is_empty() && !form.filename.trim().is_empty() && !form.script.trim().is_empty();
        let footer = div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).flex().justify_end().child(
            dialog::button("export-start", "Export and Attach", Kind::Accent, !valid).when(valid, |b| b.on_click(cx.listener(|shell, _, window, cx| shell.start_key_export(window, cx)))),
        );

        let content: AnyElement = match &form.running {
            Some((_, _)) if form.exported => div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .text_sm()
                .text_color(rgb(theme::TEXT))
                .child(svg().path(icon("check")).size(px(40.)).text_color(rgb(theme::ACCENT)))
                .child("Exported")
                .into_any_element(),
            Some((_, login)) => {
                let target = LoginTarget {
                    label: host.map(|h| h.display_label()).unwrap_or(""),
                    subtitle: format!("SSH {}", host.map(|h| h.destination()).unwrap_or_default()),
                    os: host.and_then(|h| h.os.as_deref()),
                    identities: &[],
                };
                let actions = LoginActions {
                    answer: |shell: &mut Shell, answer, _, cx| {
                        shell.export_answer(answer);
                        cx.notify();
                    },
                    close: |shell: &mut Shell, _, cx| {
                        if let Some(form) = &mut shell.key_export {
                            form.running = None;
                        }
                        cx.notify();
                    },
                    retry: |shell: &mut Shell, window, cx| shell.start_key_export(window, cx),
                    toggle_logs: |shell: &mut Shell, cx| {
                        if let Some((_, login)) = shell.key_export.as_mut().and_then(|f| f.running.as_mut()) {
                            login.show_logs = !login.show_logs;
                        }
                        cx.notify();
                    },
                    key: |shell: &mut Shell, event, _, cx| {
                        let Some((_, login)) = shell.key_export.as_mut().and_then(|f| f.running.as_mut()) else { return };
                        if let Some(answer) = login_view::edit(login, event, cx) {
                            if let Some(answer) = answer {
                                shell.export_answer(answer);
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }
                    },
                    submit: |shell: &mut Shell, _, cx| {
                        let Some((_, login)) = shell.key_export.as_mut().and_then(|f| f.running.as_mut()) else { return };
                        if let Some(answer) = login.submit_text() {
                            shell.export_answer(answer);
                        }
                        cx.notify();
                    },
                    edit_host: None,
                    verbose: None,
                    continue_save: None,
                    ui: None,
                };
                div().id("export-login").flex_1().overflow_y_scroll().child(login_view::render(target, login, &self.forward_focus, window, &actions, cx)).into_any_element()
            }
            None => div().id("export-body").flex_1().overflow_y_scroll().p(px(15.)).child(body).into_any_element(),
        };
        let slider = div()
            .id("key-export")
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_export_key))
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(WIDTH))
            .occlude()
            .flex()
            .flex_col()
            .bg(rgb(theme::BACKGROUND))
            .border_l_1()
            .border_color(rgb(theme::TITLE_BAR))
            .shadow_lg()
            .child(header)
            .child(content)
            .when(form.running.is_none(), |d| d.child(footer));
        Some(div().absolute().inset_0().child(dialog::slide_in(slider, WIDTH)).into_any_element())
    }
}
