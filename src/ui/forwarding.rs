//! Port Forwarding screen, as in the legacy client: rule cards (click starts
//! or stops a rule), the New / Edit Port Forwarding slider with the
//! Local / Remote / Dynamic diagram and fields, and the connection screen
//! when a rule's login asks something.

use std::collections::HashMap;

use futures::StreamExt;
use gpui::{div, img, prelude::*, px, rgb, svg, AnyElement, Context, Div, FontWeight, KeyDownEvent, MouseButton, SharedString, Stateful, Window};
use termian_native::forwarding::{ForwardEvent, Forwarding, Spec};
use termian_native::login::{Answer, HostKeyDecision, LoginEvent, LoginState};
use termian_native::session::SshTarget;
use termian_native::theme;
use termian_native::vault::{ForwardingRule, RuleKind};

use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::host_editor::{caret, card, field_box, menu_panel, menu_row};
use super::hosts::{card as entity_card, tile, tool_button};
use super::login_view::{self, LoginActions, LoginTarget};
use super::sections::{empty_state, grid, page};
use crate::Shell;

const WIDTH: f32 = 420.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Label,
    Port,
    Bind,
    Destination,
    DestinationPort,
}

pub struct RuleEditor {
    rule: ForwardingRule,
    created_here: bool,
    field: Field,
    hosts_open: bool,
    confirm_remove: bool,
}

pub enum RuleStatus {
    Connecting(LoginState),
    Active { port: u16 },
    Failed(String),
}

/// Running rules and what the screen shows about them.
#[derive(Default)]
pub struct Rules {
    pub editor: Option<RuleEditor>,
    running: HashMap<u64, (Forwarding, u64)>,
    status: HashMap<u64, RuleStatus>,
    /// The rule whose connection screen is open.
    login: Option<u64>,
    menu: Option<(u64, gpui::Point<gpui::Pixels>, bool)>,
    generation: u64,
}

fn kind_name(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Local => "Local",
        RuleKind::Remote => "Remote",
        RuleKind::Dynamic => "Dynamic",
    }
}

impl Shell {
    pub fn new_rule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rule = ForwardingRule::new(self.vault.next_id(), RuleKind::Local);
        self.rules.editor = Some(RuleEditor { rule, created_here: true, field: Field::Label, hosts_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn edit_rule(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(rule) = self.vault.rule(id).cloned() else { return };
        self.rules.menu = None;
        self.rules.editor = Some(RuleEditor { rule, created_here: false, field: Field::Label, hosts_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn save_rule_draft(&mut self) {
        let Some(editor) = &self.rules.editor else { return };
        self.vault.save_rule(editor.rule.clone());
        self.save_vault();
    }

    fn remove_rule(&mut self, id: u64, cx: &mut Context<Self>) {
        self.stop_rule(id);
        self.vault.remove_rule(id);
        self.save_vault();
        self.rules.editor = None;
        self.rules.menu = None;
        cx.notify();
    }

    fn stop_rule(&mut self, id: u64) {
        self.rules.running.remove(&id); // dropping it stops the rule
        self.rules.status.remove(&id);
        if self.rules.login == Some(id) {
            self.rules.login = None;
        }
    }

    /// A click on a rule card: start it, or stop it when it runs.
    fn toggle_rule(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.rules.running.contains_key(&id) {
            self.stop_rule(id);
            cx.notify();
            return;
        }
        let Some(rule) = self.vault.rule(id).cloned() else { return };
        let Some(host) = rule.host.and_then(|h| self.vault.host(h)) else {
            self.rules.status.insert(id, RuleStatus::Failed("Choose a host for this rule.".into()));
            cx.notify();
            return;
        };
        if !rule.is_valid() {
            self.rules.status.insert(id, RuleStatus::Failed("Fill in the ports and the destination.".into()));
            cx.notify();
            return;
        }
        let target = SshTarget::for_host(host, host.key.and_then(|k| self.vault.key(k)));
        let spec = Spec {
            kind: rule.kind,
            bind_address: if rule.bind_address.trim().is_empty() { "127.0.0.1".into() } else { rule.bind_address.clone() },
            port: rule.port.unwrap_or(0),
            destination: rule.destination.clone(),
            destination_port: rule.destination_port.unwrap_or(0),
        };
        let mut forwarding = Forwarding::start(target, spec);
        self.rules.generation += 1;
        let generation = self.rules.generation;
        if let Some(mut events) = forwarding.take_events() {
            cx.spawn_in(window, async move |this, cx| {
                while let Some(event) = events.next().await {
                    let alive = this.update_in(cx, |shell, window, cx| {
                        if shell.rules.running.get(&id).map(|(_, g)| *g) != Some(generation) {
                            return; // stopped or restarted
                        }
                        shell.on_forward_event(id, event, window);
                        cx.notify();
                    });
                    if alive.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        self.rules.running.insert(id, (forwarding, generation));
        self.rules.status.insert(id, RuleStatus::Connecting(LoginState::default()));
        cx.notify();
    }

    fn on_forward_event(&mut self, id: u64, event: ForwardEvent, window: &mut Window) {
        match event {
            ForwardEvent::Login(event) => {
                let asks = matches!(event, LoginEvent::HostKey(_) | LoginEvent::Prompt { .. } | LoginEvent::HostKeyChanged(_));
                if let Some(RuleStatus::Connecting(login)) = self.rules.status.get_mut(&id) {
                    login.apply(event);
                }
                // A question opens the connection screen, as for any login.
                if asks {
                    self.rules.login = Some(id);
                    window.focus(&self.forward_focus);
                }
            }
            ForwardEvent::Active { port } => {
                self.rules.status.insert(id, RuleStatus::Active { port });
                if self.rules.login == Some(id) {
                    self.rules.login = None;
                }
            }
            ForwardEvent::Closed(reason) => {
                let reason = reason.unwrap_or_else(|| "The connection was closed.".into());
                self.rules.running.remove(&id);
                match self.rules.status.get_mut(&id) {
                    // The open connection screen shows why, with Retry.
                    Some(RuleStatus::Connecting(login)) if self.rules.login == Some(id) => login.fail(reason),
                    _ => {
                        self.rules.status.insert(id, RuleStatus::Failed(reason));
                    }
                }
            }
        }
    }

    fn forward_answer(&mut self, answer: Answer) {
        let Some(id) = self.rules.login else { return };
        let cancels = matches!(answer, Answer::Text(None) | Answer::HostKey(HostKeyDecision::Reject));
        if let (Some((forwarding, _)), Some(RuleStatus::Connecting(login))) = (self.rules.running.get(&id), self.rules.status.get_mut(&id)) {
            if let Some(answer) = login.answer(answer) {
                forwarding.answer(answer);
            }
        }
        if cancels {
            self.stop_rule(id);
        }
    }

    fn on_rule_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(editor) = &mut self.rules.editor else { return };
        if editor.confirm_remove {
            if event.keystroke.key == "escape" {
                editor.confirm_remove = false;
                cx.notify();
            }
            return;
        }
        let port_text = |value: Option<u16>| value.map(|p| p.to_string()).unwrap_or_default();
        let (mut text, numeric) = match editor.field {
            Field::Label => (editor.rule.label.clone(), false),
            Field::Bind => (editor.rule.bind_address.clone(), false),
            Field::Destination => (editor.rule.destination.clone(), false),
            Field::Port => (port_text(editor.rule.port), true),
            Field::DestinationPort => (port_text(editor.rule.destination_port), true),
        };
        match dialog::edit(Some(&mut text), &event.keystroke, paste) {
            Key::Edited => {
                if numeric {
                    text.retain(|c| c.is_ascii_digit());
                }
                let port = text.parse::<u16>().ok();
                match editor.field {
                    Field::Label => editor.rule.label = text,
                    Field::Bind => editor.rule.bind_address = text,
                    Field::Destination => editor.rule.destination = text,
                    Field::Port => editor.rule.port = port,
                    Field::DestinationPort => editor.rule.destination_port = port,
                }
                self.save_rule_draft();
            }
            Key::Escape => self.rules.editor = None,
            Key::Enter | Key::Ignored => {
                if event.keystroke.key != "tab" {
                    return;
                }
                const ORDER: [Field; 5] = [Field::Label, Field::Port, Field::Bind, Field::Destination, Field::DestinationPort];
                let at = ORDER.iter().position(|f| *f == editor.field).unwrap_or(0);
                editor.field = ORDER[(at + 1) % ORDER.len()];
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn rule_input(&self, field: Field, label: &str, value: String, window: &Window, cx: &mut Context<Self>) -> Div {
        let focused = self.rules.editor.as_ref().is_some_and(|e| e.field == field) && self.editor_focus.is_focused(window);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(label.to_owned()))
            .child(
                field_box(focused)
                    .id(SharedString::from(format!("rule-{label}")))
                    .child(div().flex().items_center().text_color(rgb(theme::TEXT)).child(value).when(focused, |d| d.child(caret())))
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        if let Some(editor) = &mut shell.rules.editor {
                            editor.field = field;
                            editor.hosts_open = false;
                        }
                        window.focus(&shell.editor_focus);
                        cx.notify();
                    })),
            )
    }

    fn host_field(&self, label: &str, cx: &mut Context<Self>) -> Div {
        let Some(editor) = &self.rules.editor else { return div() };
        let chosen = editor.rule.host.and_then(|h| self.vault.host(h)).map(|h| h.display_label().to_owned());
        let picker = field_box(editor.hosts_open)
            .id("rule-host")
            .border_dashed()
            .child(div().flex_1().text_color(rgb(if chosen.is_some() { theme::TEXT } else { theme::TEXT_MUTED })).child(chosen.unwrap_or_else(|| "Hosts".into())))
            .child(svg().path(icon("chevron")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)))
            .on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.rules.editor {
                    editor.hosts_open = !editor.hosts_open;
                }
                cx.notify();
            }));
        let list = editor.hosts_open.then(|| {
            let mut list = div().id("rule-host-list").max_h(px(240.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::INPUT)).shadow_lg();
            for host in &self.vault.hosts {
                let id = host.id;
                list = list.child(menu_row(format!("rule-host-{id}"), host.display_label(), false, false).on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(editor) = &mut shell.rules.editor {
                        editor.rule.host = Some(id);
                        editor.hosts_open = false;
                    }
                    shell.save_rule_draft();
                    cx.notify();
                })));
            }
            // Opens upwards: the field sits low in a long form.
            gpui::deferred(div().absolute().bottom_full().left_0().right_0().mb_1().child(list))
        });
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(format!("{label} *")))
            .child(div().relative().child(picker).children(list))
    }

    fn render_rule_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.rules.editor.as_ref()?;
        let rule = &editor.rule;
        let port = |p: Option<u16>| p.map(|p| p.to_string()).unwrap_or_default();
        let header = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(if editor.created_here { "New Port Forwarding" } else { "Edit Port Forwarding" }))
            .child(dialog::close_button("rule-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.rules.editor = None;
                cx.notify();
            })));

        // Local / Remote / Dynamic, only while adding a rule, as in the legacy form.
        let kinds = editor.created_here.then(|| {
            div().flex().p_1().gap_1().rounded_md().bg(rgb(theme::INPUT)).children([RuleKind::Local, RuleKind::Remote, RuleKind::Dynamic].map(|kind| {
                let selected = rule.kind == kind;
                div()
                    .id(SharedString::from(format!("rule-kind-{}", kind_name(kind))))
                    .flex_1()
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .text_sm()
                    .text_color(rgb(theme::TEXT))
                    .when(selected, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                    .child(kind_name(kind))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(editor) = &mut shell.rules.editor {
                            let default_label = ForwardingRule::new(0, editor.rule.kind).label;
                            if editor.rule.label == default_label {
                                editor.rule.label = ForwardingRule::new(0, kind).label;
                            }
                            editor.rule.kind = kind;
                        }
                        shell.save_rule_draft();
                        cx.notify();
                    }))
            }))
        });
        let diagram = match rule.kind {
            RuleKind::Local => "pf_local",
            RuleKind::Remote => "pf_remote",
            RuleKind::Dynamic => "pf_dynamic",
        };
        let mut form = card(&format!("{} Port Forwarding", kind_name(rule.kind)))
            .children(kinds)
            .child(div().py(px(20.)).flex().justify_center().child(img(icon(diagram)).w(px(250.)).h(px(150.))))
            .child(self.rule_input(Field::Label, "Label", rule.label.clone(), window, cx));
        let destination = |form: Div, cx: &mut Context<Self>| {
            form.child(self.rule_input(Field::Destination, "Destination address *", rule.destination.clone(), window, cx))
                .child(self.rule_input(Field::DestinationPort, "Destination port number *", port(rule.destination_port), window, cx))
        };
        form = match rule.kind {
            RuleKind::Local => {
                let form = form
                    .child(self.rule_input(Field::Port, "Local port number *", port(rule.port), window, cx))
                    .child(self.rule_input(Field::Bind, "Bind address", rule.bind_address.clone(), window, cx))
                    .child(self.host_field("Intermediate host", cx));
                destination(form, cx)
            }
            RuleKind::Remote => {
                let form = form
                    .child(self.host_field("Remote host", cx))
                    .child(self.rule_input(Field::Port, "Remote port number *", port(rule.port), window, cx))
                    .child(self.rule_input(Field::Bind, "Bind address", rule.bind_address.clone(), window, cx));
                destination(form, cx)
            }
            RuleKind::Dynamic => form
                .child(self.rule_input(Field::Port, "Local port number *", port(rule.port), window, cx))
                .child(self.rule_input(Field::Bind, "Bind address", rule.bind_address.clone(), window, cx))
                .child(self.host_field("Intermediate host", cx)),
        };

        let id = rule.id;
        let footer = (!editor.created_here || self.vault.rule(id).is_some()).then(|| {
            div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).flex().justify_end().child(
                dialog::button("rule-remove", "Remove", Kind::Error, false).on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.rules.editor {
                        editor.confirm_remove = true;
                    }
                    cx.notify();
                })),
            )
        });
        let slider = div()
            .id("rule-editor")
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_rule_key))
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
            .child(div().id("rule-body").flex_1().overflow_y_scroll().p(px(15.)).child(form))
            .children(footer);
        let confirm = editor.confirm_remove.then(|| self.remove_rule_dialog(id, cx));
        Some(match confirm {
            Some(confirm) => div().absolute().inset_0().child(slider).child(confirm).into_any_element(),
            None => slider.into_any_element(),
        })
    }

    /// "Remove Port Forwarding rule", as in the legacy confirmation.
    fn remove_rule_dialog(&self, id: u64, cx: &mut Context<Self>) -> Stateful<Div> {
        let close = dialog::close_button("rule-remove-close").on_click(cx.listener(|shell, _, _, cx| {
            if let Some(editor) = &mut shell.rules.editor {
                editor.confirm_remove = false;
            }
            shell.rules.menu = None;
            cx.notify();
        }));
        dialog::backdrop(dialog::card("Remove Port Forwarding rule", false, close, dialog::text("Are you sure you want to delete this rule?"), vec![
            dialog::button("rule-remove-confirm", "Remove", Kind::Error, false)
                .on_click(cx.listener(move |shell, _, _, cx| shell.remove_rule(id, cx)))
                .into_any_element(),
        ]))
    }

    fn render_rule_card(&self, rule: &ForwardingRule, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = rule.id;
        let status = self.rules.status.get(&id);
        // The tile turns green while the rule runs; the subtitle says what is going on.
        let (tile_color, subtitle) = match status {
            Some(RuleStatus::Active { port }) => (theme::ACCENT, format!("Active on {}:{port}", rule.bind_address)),
            Some(RuleStatus::Connecting(_)) => (theme::TILE_BLUE, "Connecting…".to_owned()),
            Some(RuleStatus::Failed(reason)) => (theme::ERROR, reason.clone()),
            None => (theme::TILE_BLUE, if rule.is_valid() { rule.rule_text(&self.vault) } else { String::new() }),
        };
        let group = SharedString::from(format!("rule-card-{id}"));
        let edit = div()
            .id(SharedString::from(format!("rule-edit-{id}")))
            .size(px(28.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .invisible()
            .group_hover(group.clone(), |s| s.visible())
            .hover(|s| s.bg(rgb(theme::INPUT)))
            .child(svg().path(icon("edit")).size(px(12.)).text_color(rgb(theme::TEXT)))
            .on_click(cx.listener(move |shell, _, window, cx| {
                cx.stop_propagation();
                shell.edit_rule(id, window, cx);
            }));
        entity_card(format!("rule-{id}").into(), tile("port_forwarding", tile_color), rule.title(&self.vault), subtitle)
            .group(group)
            .child(edit)
            .on_click(cx.listener(move |shell, _, window, cx| shell.toggle_rule(id, window, cx)))
            .on_mouse_down(MouseButton::Right, cx.listener(move |shell, event: &gpui::MouseDownEvent, _, cx| {
                shell.rules.menu = Some((id, event.position, false));
                cx.notify();
            }))
    }

    pub fn render_port_forwarding(&self, cx: &mut Context<Self>) -> Div {
        let toolbar = vec![tool_button("new-forwarding", "port_forwarding", "NEW FORWARDING").on_click(cx.listener(|shell, _, window, cx| shell.new_rule(window, cx)))];
        if self.vault.rules.is_empty() {
            return page(
                "forwarding-scroll",
                toolbar,
                empty_state("port_forwarding", "Set up port forwarding", "Save port forwarding to access databases, web apps, and other services."),
            );
        }
        let body = div()
            .child(super::hosts::section_title("Port Forwarding"))
            .child(grid().children(self.vault.rules.iter().map(|rule| self.render_rule_card(rule, cx))));
        page("forwarding-scroll", toolbar, body)
    }

    /// The slider, the rule menu and a rule's connection screen.
    pub fn render_forwarding_overlay(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if let Some(id) = self.rules.login {
            if let (Some(RuleStatus::Connecting(login)), Some(rule)) = (self.rules.status.get(&id), self.vault.rule(id)) {
                let host = rule.host.and_then(|h| self.vault.host(h));
                let target = LoginTarget {
                    label: host.map(|h| h.display_label()).unwrap_or(""),
                    subtitle: format!("SSH {}", host.map(|h| h.destination()).unwrap_or_default()),
                    os: host.and_then(|h| h.os.as_deref()),
                };
                let actions = LoginActions {
                    answer: |shell: &mut Shell, answer, _, cx| {
                        shell.forward_answer(answer);
                        cx.notify();
                    },
                    close: |shell: &mut Shell, _, cx| {
                        if let Some(id) = shell.rules.login {
                            shell.stop_rule(id);
                        }
                        cx.notify();
                    },
                    retry: |shell: &mut Shell, window, cx| {
                        if let Some(id) = shell.rules.login.take() {
                            shell.stop_rule(id);
                            shell.toggle_rule(id, window, cx);
                        }
                    },
                    toggle_logs: |shell: &mut Shell, cx| {
                        if let Some(Some(RuleStatus::Connecting(login))) = shell.rules.login.map(|id| shell.rules.status.get_mut(&id)) {
                            login.show_logs = !login.show_logs;
                        }
                        cx.notify();
                    },
                    key: |shell: &mut Shell, event, _, cx| {
                        let paste = cx.read_from_clipboard().and_then(|item| item.text());
                        let Some(id) = shell.rules.login else { return };
                        let Some(RuleStatus::Connecting(login)) = shell.rules.status.get_mut(&id) else { return };
                        if let Some(answer) = login_view::edit(login, event, paste) {
                            if let Some(answer) = answer {
                                shell.forward_answer(answer);
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }
                    },
                    submit: |shell: &mut Shell, _, cx| {
                        let Some(id) = shell.rules.login else { return };
                        if let Some(RuleStatus::Connecting(login)) = shell.rules.status.get_mut(&id) {
                            if let Some(answer) = login.submit_text() {
                                shell.forward_answer(answer);
                            }
                        }
                        cx.notify();
                    },
                };
                let screen = login_view::render(target, login, &self.forward_focus, window, &actions, cx);
                return Some(div().id("forward-login").absolute().inset_0().occlude().child(screen).into_any_element());
            }
        }
        if let Some((id, position, confirm)) = self.rules.menu {
            if confirm {
                return Some(self.remove_rule_dialog(id, cx).into_any_element());
            }
            let running = self.rules.running.contains_key(&id);
            let items = menu_panel("rule-menu")
                .child(menu_row("rule-menu-toggle".into(), if running { "Stop" } else { "Start" }, false, false).on_click(cx.listener(move |shell, _, window, cx| {
                    shell.rules.menu = None;
                    shell.toggle_rule(id, window, cx);
                })))
                .child(menu_row("rule-menu-edit".into(), "Edit", false, false).on_click(cx.listener(move |shell, _, window, cx| shell.edit_rule(id, window, cx))))
                .child(menu_row("rule-menu-remove".into(), "Remove", true, false).on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(menu) = &mut shell.rules.menu {
                        menu.2 = true;
                    }
                    cx.notify();
                })));
            return Some(
                div()
                    .id("rule-menu-dismiss")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                        shell.rules.menu = None;
                        cx.notify();
                    }))
                    .child(div().absolute().left(position.x).top(position.y).child(items))
                    .into_any_element(),
            );
        }
        self.render_rule_editor(window, cx)
    }
}
