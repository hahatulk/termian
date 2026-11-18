//! The rest of the legacy host form: Backspace, Agent Forwarding, Startup
//! Command, Environment Variable, charset and the host's colour theme, with
//! their menus and the «Environment Variables» and «Select Color Theme»
//! panels.

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, FontWeight, SharedString, Stateful, Window};
use termian_native::terminal_settings::themes;
use termian_native::theme;
use termian_native::vault::EnvVar;

use super::assets::icon;
use super::host_editor::{card, field_box, menu_row, Field};
use crate::Shell;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    Backspace,
    Agent,
    Startup,
    Charset,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Env,
    Theme,
}

/// A setting as a legacy field: icon, title, and the value on the right.
fn row(id: &str, icon_name: &str, title: String, value: Option<&str>, active: bool) -> Stateful<Div> {
    field_box(false)
        .id(SharedString::from(id.to_owned()))
        .child(svg().path(icon(icon_name)).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
        .child(div().flex_1().min_w_0().truncate().text_color(rgb(if active { theme::TEXT } else { theme::TEXT_MUTED })).child(title))
        .children(value.map(|v| div().flex_none().text_color(rgb(theme::TEXT_MUTED)).child(v.to_owned())))
}

/// A menu choice with the legacy check mark on the chosen one.
fn choice(id: String, label: &str, value: Option<&str>, checked: bool) -> Stateful<Div> {
    menu_row(id, "", false, false)
        .gap_2()
        .child(div().w(px(14.)).children(checked.then(|| svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::TEXT)))))
        .child(div().flex_1().child(label.to_owned()))
        .children(value.map(|v| div().text_color(rgb(theme::TEXT_MUTED)).child(v.to_owned())))
}

fn list(id: &str) -> Stateful<Div> {
    // Menus open over other rows: clicks must not reach them.
    div().id(SharedString::from(id.to_owned())).occlude().max_h(px(240.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::INPUT)).shadow_lg()
}

impl Shell {
    /// Legacy «Credentials»: Username (saved identities are suggested while it
    /// is focused), Password, and «+ SSH.id, Key, Certificate, FIDO2».
    pub(super) fn render_credentials(&self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(editor) = &self.host_editor else { return Vec::new() };
        let host = &editor.host;
        let mut out = Vec::new();
        let field_icon = |name: &str| svg().path(icon(name)).size(px(14.)).text_color(rgb(theme::TEXT_MUTED));
        let clear = |id: &str| {
            div()
                .id(SharedString::from(id.to_owned()))
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(svg().path(icon("close")).size(px(9.)).text_color(rgb(theme::TEXT_MUTED)))
        };

        match host.identity.and_then(|id| self.vault.identity(id)) {
            // A chosen identity stands in for the username, password and key.
            Some(identity) => {
                out.push(
                    field_box(false)
                        .child(field_icon("identity"))
                        .child(div().flex_1().truncate().text_color(rgb(theme::TEXT)).child(identity.label.clone()))
                        .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(identity.subtitle()))
                        .child(clear("credentials-identity-clear").on_click(cx.listener(|shell, _, _, cx| {
                            if let Some(editor) = &mut shell.host_editor {
                                editor.host.identity = None;
                            }
                            shell.autosave();
                            cx.notify();
                        })))
                        .into_any_element(),
                );
                return out;
            }
            None => {
                let username = host.username.clone().unwrap_or_default();
                let focused = editor.field == Field::Username && self.editor_focus.is_focused(window);
                let typed = username.to_lowercase();
                let matches: Vec<_> = self
                    .vault
                    .identities
                    .iter()
                    .filter(|i| typed.is_empty() || i.label.to_lowercase().contains(&typed) || i.username.to_lowercase().contains(&typed))
                    .collect();
                let suggestions = (focused && !matches.is_empty()).then(|| {
                    let mut menu = list("credentials-identities");
                    for identity in matches {
                        let id = identity.id;
                        let detail = std::iter::once(identity.username.clone())
                            .chain(identity.password.as_ref().map(|_| "••••••".to_owned()))
                            .chain(identity.key.and_then(|k| self.vault.key(k)).map(|k| k.label.clone()))
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(", ");
                        menu = menu.child(
                            menu_row(format!("credentials-identity-{id}"), "", false, false)
                                .h(px(44.))
                                .gap_2()
                                .child(svg().path(icon("identity")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                                .child(div().flex().flex_col().child(identity.label.clone()).child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(detail)))
                                .on_click(cx.listener(move |shell, _, _, cx| {
                                    if let Some(editor) = &mut shell.host_editor {
                                        editor.host.identity = Some(id);
                                        editor.host.username = None;
                                        editor.field = Field::Label;
                                    }
                                    shell.autosave();
                                    cx.notify();
                                })),
                        );
                    }
                    menu
                });
                out.push(
                    div()
                        .relative()
                        .child(self.input(Field::Username, "Username", &username, false, window, cx))
                        .children(suggestions.map(|menu| gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(menu)).with_priority(1)))
                        .into_any_element(),
                );
                out.push(self.input(Field::Password, "Password", host.password.as_deref().unwrap_or(""), true, window, cx).into_any_element());
            }
        }

        // The key: chosen, being picked, or the legacy «+» button with its menu.
        if let Some(key) = host.key.and_then(|k| self.vault.key(k)) {
            out.push(
                field_box(false)
                    .child(field_icon("keychain"))
                    .child(div().flex_1().truncate().text_color(rgb(theme::TEXT)).child(key.label.clone()))
                    .child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(if key.certificate.is_some() { "Certificate".to_owned() } else { key.kind.clone() }))
                    .child(clear("credentials-key-clear").on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            editor.host.key = None;
                        }
                        shell.autosave();
                        cx.notify();
                    })))
                    .into_any_element(),
            );
        } else if editor.keys_open {
            let mut menu = list("credentials-keys");
            let certificates_only = editor.certificates_only;
            for key in self.vault.keys.iter().filter(|k| !certificates_only || k.certificate.is_some()) {
                let id = key.id;
                // A key whose private part could not be read from the legacy profile cannot be used.
                let usable = key.private_key.is_some();
                let label = if usable { key.label.clone() } else { format!("{} — not available in this client", key.label) };
                menu = menu.child(menu_row(format!("credentials-key-{id}"), &label, false, !usable).when(usable, |d| {
                    d.on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            editor.host.key = Some(id);
                            editor.keys_open = false;
                        }
                        shell.autosave();
                        cx.notify();
                    }))
                }));
            }
            out.push(
                div()
                    .relative()
                    .child(
                        field_box(true)
                            .id("credentials-key-field")
                            .child(field_icon("keychain"))
                            .child(div().flex_1().text_color(rgb(theme::TEXT_MUTED)).child(if certificates_only { "Certificate" } else { "Key" }))
                            .child(clear("credentials-key-cancel").on_click(cx.listener(|shell, _, _, cx| {
                                if let Some(editor) = &mut shell.host_editor {
                                    editor.keys_open = false;
                                }
                                cx.notify();
                            }))),
                    )
                    .child(gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(menu)).with_priority(1))
                    .into_any_element(),
            );
        } else {
            let open = editor.key_menu;
            let button = div()
                .id("credentials-add")
                .h(px(36.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded_md()
                .text_color(rgb(theme::TEXT))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(svg().path(icon("plus")).size(px(12.)).text_color(rgb(theme::TEXT)))
                .child("SSH.id, Key, Certificate, FIDO2")
                .on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.host_editor {
                        editor.key_menu = !editor.key_menu;
                    }
                    cx.notify();
                }));
            // SSH.id is a cloud service and FIDO2 keys are not supported here.
            // A certificate belongs to its key, so Certificate picks among keys that have one.
            let menu = open.then(|| {
                list("credentials-add-menu")
                    .child(menu_row("credentials-add-sshid".into(), "SSH.id — Passkeys for SSH", false, true))
                    .child(menu_row("credentials-add-key".into(), "Key", false, false).on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            editor.key_menu = false;
                            editor.keys_open = true;
                            editor.certificates_only = false;
                        }
                        cx.notify();
                    })))
                    .child(menu_row("credentials-add-certificate".into(), "Certificate", false, false).on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            editor.key_menu = false;
                            editor.keys_open = true;
                            editor.certificates_only = true;
                        }
                        cx.notify();
                    })))
                    .child(menu_row("credentials-add-fido2".into(), "FIDO2", false, true))
            });
            out.push(div().relative().child(button).children(menu.map(|menu| gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(menu)).with_priority(1))).into_any_element());
        }
        out
    }

    fn change_host_settings(&mut self, change: impl FnOnce(&mut termian_native::vault::HostSettings), cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.host_editor {
            change(&mut editor.host.settings);
            editor.picker = None;
        }
        self.autosave();
        cx.notify();
    }

    fn toggle_picker(&mut self, picker: Picker, cx: &mut Context<Self>) {
        if let Some(editor) = &mut self.host_editor {
            editor.picker = if editor.picker == Some(picker) { None } else { Some(picker) };
        }
        cx.notify();
    }

    /// A row whose menu opens under it, over the form.
    fn with_picker(&self, picker: Picker, row: Stateful<Div>, menu: impl FnOnce(&mut Context<Self>) -> Stateful<Div>, cx: &mut Context<Self>) -> Div {
        let open = self.host_editor.as_ref().is_some_and(|e| e.picker == Some(picker));
        let row = row.on_click(cx.listener(move |shell, _, _, cx| shell.toggle_picker(picker, cx)));
        let menu = open.then(|| menu(cx));
        div().relative().child(row).children(menu.map(|menu| gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(menu)).with_priority(1)))
    }

    /// «Backspace»: Default or Ctrl+H; it sits in the General card.
    pub(super) fn render_backspace_row(&self, cx: &mut Context<Self>) -> Div {
        let Some(editor) = &self.host_editor else { return div() };
        let ctrl_h = editor.host.settings.ctrl_h_backspace;
        let row = row("editor-backspace", "backspace", "Backspace".into(), Some(if ctrl_h { "Ctrl+H" } else { "Default" }), ctrl_h);
        let menu = |cx: &mut Context<Self>| {
            list("editor-backspace-list")
                .child(choice("backspace-ctrl-h".into(), "Backspace", Some("Ctrl+H"), ctrl_h).on_click(cx.listener(|shell, _, _, cx| shell.change_host_settings(|s| s.ctrl_h_backspace = true, cx))))
                .child(choice("backspace-default".into(), "Backspace", Some("Default"), !ctrl_h).on_click(cx.listener(|shell, _, _, cx| shell.change_host_settings(|s| s.ctrl_h_backspace = false, cx))))
        };
        self.with_picker(Picker::Backspace, row, menu, cx)
    }

    /// The SSH card's rows after the credentials, in the legacy order.
    pub(super) fn render_ssh_settings(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(editor) = &self.host_editor else { return Vec::new() };
        let settings = &editor.host.settings;
        let mut rows = Vec::new();

        let agent = settings.agent_forwarding;
        let agent_row = row("editor-agent", "agent", "Agent Forwarding".into(), Some(if agent { "Enabled" } else { "Disabled" }), agent);
        let agent_menu = |cx: &mut Context<Self>| {
            list("editor-agent-list")
                .child(choice("agent-on".into(), "Enabled", None, agent).on_click(cx.listener(|shell, _, _, cx| shell.change_host_settings(|s| s.agent_forwarding = true, cx))))
                .child(choice("agent-off".into(), "Disabled", None, !agent).on_click(cx.listener(|shell, _, _, cx| shell.change_host_settings(|s| s.agent_forwarding = false, cx))))
        };
        rows.push(self.with_picker(Picker::Agent, agent_row, agent_menu, cx).into_any_element());

        let snippet = settings.startup_snippet.and_then(|id| self.vault.snippets.iter().find(|s| s.id == id));
        let title = snippet.map_or_else(|| "Startup Command".to_owned(), |s| if s.label.is_empty() { s.script.clone() } else { s.label.clone() });
        let startup_row = row("editor-startup", "snippets", title, None, snippet.is_some());
        let chosen = settings.startup_snippet;
        let startup_menu = |cx: &mut Context<Self>| {
            let mut menu = list("editor-startup-list").child(choice("startup-none".into(), "No startup command", None, chosen.is_none()).on_click(cx.listener(|shell, _, _, cx| shell.change_host_settings(|s| s.startup_snippet = None, cx))));
            for snippet in &self.vault.snippets {
                let id = snippet.id;
                let label = if snippet.label.is_empty() { snippet.script.as_str() } else { snippet.label.as_str() };
                menu = menu.child(choice(format!("startup-{id}"), label, None, chosen == Some(id)).on_click(cx.listener(move |shell, _, _, cx| shell.change_host_settings(|s| s.startup_snippet = Some(id), cx))));
            }
            menu
        };
        rows.push(self.with_picker(Picker::Startup, startup_row, startup_menu, cx).into_any_element());

        // Legacy shows the variables as `NAME=value, …`.
        let env_text = settings.env.iter().map(|v| format!("{}={}", v.name, v.value)).collect::<Vec<_>>().join(", ");
        let env_row = row("editor-env", "env", if env_text.is_empty() { "Environment Variable".into() } else { env_text }, None, !settings.env.is_empty()).on_click(cx.listener(|shell, _, window, cx| {
            if let Some(editor) = &mut shell.host_editor {
                if editor.host.settings.env.is_empty() {
                    editor.host.settings.env.push(EnvVar::default());
                }
                editor.field = Field::EnvName(0);
                editor.panel = Some(Panel::Env);
            }
            window.focus(&shell.editor_focus);
            cx.notify();
        }));
        rows.push(env_row.into_any_element());

        let charset = settings.charset.clone().unwrap_or_else(|| "UTF-8".into());
        let charset_row = row("editor-charset", "charset", charset.clone(), None, settings.charset.is_some());
        let charset_menu = |cx: &mut Context<Self>| {
            let mut menu = list("editor-charset-list");
            for name in termian_native::serial::charsets() {
                let value = (name != "UTF-8").then(|| name.to_owned());
                menu = menu.child(choice(format!("charset-{name}"), name, None, charset == name).on_click(cx.listener(move |shell, _, _, cx| {
                    let value = value.clone();
                    shell.change_host_settings(|s| s.charset = value, cx)
                })));
            }
            menu
        };
        rows.push(self.with_picker(Picker::Charset, charset_row, charset_menu, cx).into_any_element());

        rows.push(self.theme_row(cx).into_any_element());
        rows
    }

    /// The host's terminal theme with a small preview; it opens the theme list.
    fn theme_row(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let name = self.host_editor.as_ref().and_then(|e| e.host.settings.theme_name().map(str::to_owned));
        let shown = name.clone().unwrap_or_else(|| self.vault.terminal.theme().name.clone());
        div()
            .id("editor-theme")
            .flex()
            .items_center()
            .gap_3()
            .p_1()
            .rounded_md()
            .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
            .child(theme_preview(&shown))
            .child(div().flex_1().text_color(rgb(theme::TEXT)).child(shown))
            .on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.host_editor {
                    editor.panel = Some(Panel::Theme);
                }
                cx.notify();
            }))
    }

    /// A panel that takes the form's place: its own header with a back arrow.
    pub(super) fn render_host_panel(&self, panel: Panel, window: &Window, cx: &mut Context<Self>) -> Div {
        let back = div()
            .id("editor-panel-back")
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
            .child(svg().path(icon("arrow_left")).size(px(14.)).text_color(rgb(theme::TEXT)))
            .on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.host_editor {
                    editor.panel = None;
                    // Rows left empty are dropped, as legacy does not save them.
                    editor.host.settings.env.retain(|v| !v.name.trim().is_empty());
                    editor.field = Field::Label;
                }
                shell.autosave();
                cx.notify();
            }));
        let title = match panel {
            Panel::Env => "Environment Variables",
            Panel::Theme => "Select Color Theme",
        };
        let header = div().h(px(56.)).flex_none().px(px(15.)).flex().items_center().gap_2().border_b_1().border_color(rgb(theme::TITLE_BAR)).child(back).child(div().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title));
        let body = match panel {
            Panel::Env => self.env_panel(window, cx),
            Panel::Theme => self.theme_panel(cx),
        };
        div().size_full().flex().flex_col().child(header).child(div().id("editor-panel-body").flex_1().overflow_y_scroll().p(px(15.)).child(body))
    }

    fn env_panel(&self, window: &Window, cx: &mut Context<Self>) -> Div {
        let Some(editor) = &self.host_editor else { return div() };
        let host = [&editor.host.label, &editor.host.address].into_iter().find(|s| !s.trim().is_empty()).cloned().unwrap_or_else(|| "this host".into());
        let mut panel = card("Variables").child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child(format!("Set an environment variable for {host}")));
        for (i, var) in editor.host.settings.env.iter().enumerate() {
            let remove = div()
                .id(SharedString::from(format!("env-remove-{i}")))
                .size(px(28.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(svg().path(icon("close")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)))
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(editor) = &mut shell.host_editor {
                        if i < editor.host.settings.env.len() {
                            editor.host.settings.env.remove(i);
                            editor.field = Field::Label;
                        }
                    }
                    shell.autosave();
                    cx.notify();
                }));
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(self.input(Field::EnvName(i), "Variable", &var.name, false, window, cx).flex_1())
                    .child(self.input(Field::EnvValue(i), "Value", &var.value, false, window, cx).flex_1())
                    .child(remove),
            );
        }
        panel.child(
            div()
                .id("env-add")
                .h(px(36.))
                .flex()
                .items_center()
                .gap_2()
                .text_color(rgb(theme::TEXT))
                .child(svg().path(icon("plus")).size(px(12.)).text_color(rgb(theme::TEXT)))
                .child("Add variable")
                .on_click(cx.listener(|shell, _, window, cx| {
                    if let Some(editor) = &mut shell.host_editor {
                        editor.host.settings.env.push(EnvVar::default());
                        editor.field = Field::EnvName(editor.host.settings.env.len() - 1);
                    }
                    window.focus(&shell.editor_focus);
                    cx.notify();
                })),
        )
    }

    fn theme_panel(&self, cx: &mut Context<Self>) -> Div {
        let chosen = self.host_editor.as_ref().and_then(|e| e.host.settings.theme_name().map(str::to_owned));
        let global = self.vault.terminal.theme().name.clone();
        let option = |id: String, name: &str, label: String, checked: bool, value: Option<String>, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(id))
                .flex()
                .items_center()
                .gap_3()
                .p_1()
                .rounded_md()
                .when(checked, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                .child(theme_preview(name))
                .child(div().flex_1().text_color(rgb(theme::TEXT)).child(label))
                .children(checked.then(|| svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::ACCENT))))
                .on_click(cx.listener(move |shell, _, _, cx| {
                    let value = value.clone();
                    shell.change_host_settings(|s| s.color_scheme = value.map(serde_json::Value::String), cx);
                    if let Some(editor) = &mut shell.host_editor {
                        editor.panel = None;
                    }
                }))
        };
        let mut list = div().flex().flex_col().gap_1().child(option("theme-global".into(), &global, format!("Default ({global})"), chosen.is_none(), None, cx));
        for theme in themes() {
            list = list.child(option(format!("theme-{}", theme.name), &theme.name, theme.name.clone(), chosen.as_deref() == Some(theme.name.as_str()), Some(theme.name.clone()), cx));
        }
        list
    }
}

/// The legacy theme thumbnail: lines in the theme's colours on its background.
fn theme_preview(name: &str) -> Div {
    let theme = termian_native::terminal_settings::theme(name);
    let bar = |w: f32, color: u32| div().w(px(w)).h(px(3.)).rounded_full().bg(rgb(color));
    div()
        .w(px(56.))
        .h(px(36.))
        .flex_none()
        .p(px(6.))
        .flex()
        .flex_col()
        .gap(px(3.))
        .rounded_md()
        .border_1()
        .border_color(rgb(theme::SURFACE_ACTIVE))
        .bg(rgb(theme.background))
        .child(bar(40., theme.foreground))
        .child(div().flex().gap(px(3.)).child(bar(18., theme.foreground)).child(bar(18., theme.foreground)))
        .child(div().flex().gap(px(3.)).child(bar(10., theme.palette[2])).child(bar(10., theme.foreground)).child(bar(6., theme.palette[4])))
}
