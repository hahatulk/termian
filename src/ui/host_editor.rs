//! New Host / Host Details: the legacy right-hand slider for a host, saved
//! as you type (Address, General, SSH), with Connect at the bottom and the
//! three-dots menu (Connect, Duplicate, Remove). Also the host card's
//! right-click menu and the "Remove a host" confirmation.

use gpui::{
    div, prelude::*, px, rgb, svg, AnyElement, Context, Div, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Pixels, Point,
    SharedString, Stateful, Window,
};
use termian_native::theme;
use termian_native::vault::{GroupId, Host, HostId};

use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::hosts::host_tile;
use crate::Shell;

const WIDTH: f32 = 420.;
const MENU_WIDTH: f32 = 220.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Address,
    Label,
    Tags,
    Port,
    Username,
    Password,
    /// A row of «Environment Variables».
    EnvName(usize),
    EnvValue(usize),
}

pub struct HostEditor {
    pub host: Host,
    /// Not in the vault yet: saved once an address is typed.
    pub is_new: bool,
    /// Opened with NEW HOST: the title stays "New Host" while it is open.
    created_here: bool,
    pub(super) field: Field,
    /// Tag being typed, added with Enter.
    tag: String,
    port: String,
    groups_open: bool,
    pub(super) keys_open: bool,
    /// The «+ SSH.id, Key, Certificate, FIDO2» menu is open.
    pub(super) key_menu: bool,
    menu_open: bool,
    /// "Remove a host" is showing.
    confirm_remove: bool,
    /// «Do you want to discard changes?» is showing.
    confirm_discard: bool,
    /// The open menu of a setting row (Backspace, Agent Forwarding, ...).
    pub(super) picker: Option<super::host_settings::Picker>,
    /// The «Environment Variables» or «Select Color Theme» panel over the form.
    pub(super) panel: Option<super::host_settings::Panel>,
}

/// Right-click menu of a host card.
pub struct HostMenu {
    host: HostId,
    position: Point<Pixels>,
    confirm_remove: bool,
}

pub(super) fn field_box(focused: bool) -> Div {
    div()
        .h(px(40.))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .bg(rgb(theme::INPUT))
        .border_1()
        .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
}

pub(super) fn caret() -> Div {
    div().w(px(1.5)).h(px(16.)).bg(rgb(theme::TEXT))
}

pub(super) fn card(title: &str) -> Div {
    div()
        .p(px(15.))
        .pb(px(20.))
        .flex()
        .flex_col()
        .gap(px(10.))
        .rounded_lg()
        .bg(rgb(theme::SURFACE))
        .child(div().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child(title.to_owned()))
}

pub(super) fn menu_row(id: String, label: &str, red: bool, disabled: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id))
        .h(px(32.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .text_sm()
        .text_color(rgb(match (disabled, red) {
            (true, _) => theme::TEXT_MUTED,
            (false, true) => theme::ERROR,
            (false, false) => theme::TEXT,
        }))
        .when(!disabled, |d| d.hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE))))
        .child(label.to_owned())
}

pub(super) fn menu_panel(id: &str) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .w(px(MENU_WIDTH))
        .p_1()
        .rounded_md()
        .bg(rgb(theme::SURFACE))
        .border_1()
        .border_color(rgb(theme::SURFACE_ACTIVE))
        .shadow_lg()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

/// "Remove a host" as in the legacy confirmation: the host and a red Remove.
fn remove_card(host: &Host, close: Stateful<Div>, remove: Stateful<Div>) -> Div {
    let row = div()
        .flex()
        .items_center()
        .gap_3()
        .py(px(14.))
        .child(host_tile(host))
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().text_color(rgb(theme::TEXT)).child(host.display_label().to_owned()))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(host.subtitle())),
        );
    dialog::card("Remove a host", false, close, div().flex().flex_col().child("You are going to remove this host:").child(row), vec![
        remove.into_any_element(),
    ])
}

impl Shell {
    pub fn new_host(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let host = Host {
            id: self.vault.next_id(),
            label: String::new(),
            address: String::new(),
            os: None,
            group: self.group,
            username: None,
            tags: Vec::new(),
            port: None,
            password: None,
            key: None,
            identity: None,
            settings: Default::default(),
        };
        self.open_editor(host, true, window, cx);
    }

    pub fn edit_host(&mut self, id: HostId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(host) = self.vault.host(id).cloned() {
            self.open_editor(host, false, window, cx);
        }
    }

    fn open_editor(&mut self, host: Host, is_new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let port = host.port.map(|p| p.to_string()).unwrap_or_default();
        self.host_menu = None;
        self.group_editor = None;
        self.group_menu = None;
        self.host_editor = Some(HostEditor {
            host,
            is_new,
            created_here: is_new,
            field: Field::Address,
            tag: String::new(),
            port,
            groups_open: false,
            keys_open: false,
            key_menu: false,
            menu_open: false,
            confirm_remove: false,
            confirm_discard: false,
            picker: None,
            panel: None,
        });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    /// Writes the vault file; a failure is shown above the hosts.
    pub fn save_vault(&mut self) {
        if let Some(store) = &self.store {
            if let Err(error) = store.save(&self.vault) {
                self.vault_error = Some(format!("changes were not saved: {error}"));
            }
        }
    }

    /// Saves the draft as typed, like the legacy form: a new host appears
    /// once it has an address.
    pub(super) fn autosave(&mut self) {
        let Some(editor) = &mut self.host_editor else { return };
        editor.host.port = editor.port.parse().ok();
        if editor.host.address.trim().is_empty() && editor.is_new {
            return;
        }
        editor.is_new = false;
        self.vault.save_host(editor.host.clone());
        self.save_vault();
    }

    fn close_editor(&mut self, cx: &mut Context<Self>) {
        // A new host with something typed but no address is not saved:
        // legacy asks before throwing it away.
        if let Some(editor) = &mut self.host_editor {
            let host = &editor.host;
            let typed = !host.label.trim().is_empty() || !host.tags.is_empty() || host.username.is_some() || host.password.is_some() || host.group.is_some() || host.settings != Default::default() || !editor.port.is_empty();
            if editor.is_new && host.address.trim().is_empty() && typed && !editor.confirm_discard {
                editor.confirm_discard = true;
                cx.notify();
                return;
            }
        }
        self.host_editor = None;
        cx.notify();
    }

    fn connect_from_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &self.host_editor else { return };
        if editor.is_new || editor.host.address.trim().is_empty() {
            return;
        }
        let id = editor.host.id;
        self.host_editor = None;
        self.connect_host(id, window, cx);
    }

    pub fn remove_host(&mut self, id: HostId, cx: &mut Context<Self>) {
        self.vault.remove_host(id);
        self.save_vault();
        if self.host_editor.as_ref().is_some_and(|e| e.host.id == id) {
            self.host_editor = None;
        }
        self.host_menu = None;
        cx.notify();
    }

    fn duplicate_host(&mut self, id: HostId, window: &mut Window, cx: &mut Context<Self>) {
        self.host_menu = None;
        if let Some(copy) = self.vault.duplicate_host(id) {
            self.save_vault();
            if self.host_editor.is_some() {
                self.edit_host(copy, window, cx);
            }
        }
        cx.notify();
    }

    fn on_editor_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(editor) = &mut self.host_editor else { return };
        if editor.confirm_discard {
            if event.keystroke.key == "escape" {
                editor.confirm_discard = false;
                cx.notify();
            }
            return;
        }
        if editor.confirm_remove {
            if event.keystroke.key == "escape" {
                editor.confirm_remove = false;
                cx.notify();
            }
            return;
        }
        if event.keystroke.key == "tab" {
            const ORDER: [Field; 6] = [Field::Address, Field::Label, Field::Tags, Field::Port, Field::Username, Field::Password];
            let at = ORDER.iter().position(|f| *f == editor.field).unwrap_or(0);
            let step = if event.keystroke.modifiers.shift { ORDER.len() - 1 } else { 1 };
            editor.field = ORDER[(at + step) % ORDER.len()];
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let value = match editor.field {
            Field::Address => &mut editor.host.address,
            Field::Label => &mut editor.host.label,
            Field::Tags => &mut editor.tag,
            Field::Port => &mut editor.port,
            Field::Username => editor.host.username.get_or_insert_with(String::new),
            Field::Password => editor.host.password.get_or_insert_with(String::new),
            Field::EnvName(i) | Field::EnvValue(i) => {
                let env = &mut editor.host.settings.env;
                if i >= env.len() {
                    return;
                }
                if matches!(editor.field, Field::EnvName(_)) { &mut env[i].name } else { &mut env[i].value }
            }
        };
        let before = value.clone();
        let key = dialog::edit(Some(value), &event.keystroke, paste);
        if editor.field == Field::Port {
            editor.port.retain(|c| c.is_ascii_digit());
            editor.port.truncate(5);
        }
        // Empty optional fields are stored as absent.
        if editor.host.username.as_deref() == Some("") {
            editor.host.username = None;
        }
        if editor.host.password.as_deref() == Some("") {
            editor.host.password = None;
        }
        match key {
            Key::Edited => {
                let changed = match editor.field {
                    Field::Tags => false,
                    Field::Port => editor.port != before,
                    _ => true,
                };
                if changed {
                    self.autosave();
                }
            }
            Key::Enter => match editor.field {
                Field::Tags => {
                    let tag = std::mem::take(&mut editor.tag).trim().to_owned();
                    if !tag.is_empty() && !editor.host.tags.contains(&tag) {
                        editor.host.tags.push(tag);
                        self.autosave();
                    }
                }
                // Enter in the connection fields connects, as in the legacy form.
                Field::Address | Field::Port | Field::Username | Field::Password => self.connect_from_editor(window, cx),
                Field::Label | Field::EnvName(_) | Field::EnvValue(_) => {}
            },
            // Escape leaves a sub-panel first, then the form.
            Key::Escape if editor.panel.is_some() => editor.panel = None,
            Key::Escape => self.close_editor(cx),
            Key::Ignored => {
                // Backspace on an empty tag field removes the last tag.
                if event.keystroke.key == "backspace" && editor.field == Field::Tags && editor.tag.is_empty() && editor.host.tags.pop().is_some() {
                    self.autosave();
                } else {
                    return;
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub(super) fn input(&self, field: Field, placeholder: &str, value: &str, masked: bool, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let Some(editor) = &self.host_editor else { return div().id("none") };
        let focused = editor.field == field && self.editor_focus.is_focused(window);
        let shown = if masked { "•".repeat(value.chars().count()) } else { value.to_owned() };
        let empty = shown.is_empty();
        field_box(focused)
            .id(SharedString::from(format!("editor-{placeholder}")))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .text_color(rgb(if empty { theme::TEXT_MUTED } else { theme::TEXT }))
                    .when(focused && empty, |d| d.child(caret()))
                    .child(if empty { placeholder.to_owned() } else { shown })
                    .when(focused && !empty, |d| d.child(caret())),
            )
            .on_click(cx.listener(move |shell, _, window, cx| {
                if let Some(editor) = &mut shell.host_editor {
                    editor.field = field;
                    editor.groups_open = false;
                }
                window.focus(&shell.editor_focus);
                cx.notify();
            }))
    }

    /// The slider over the right side of the Hosts screen.
    pub fn render_host_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.host_editor.as_ref()?;
        let host = &editor.host;
        let title = if editor.created_here { "New Host" } else { "Host Details" };
        let header = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            // Legacy shows the vault under the title; this client has only the personal one.
            .child(div().flex_1().flex().flex_col().child(div().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title)).child(div().text_sm().text_color(rgb(theme::TEXT_MUTED)).child("Personal vault")))
            .when(!editor.is_new, |d| {
                d.child(svg().path(icon("check")).size(px(12.)).text_color(rgb(theme::TEXT)))
            })
            .child(
                div()
                    .id("editor-dots")
                    .size(px(28.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                    .child(svg().path(icon("dots")).w(px(16.)).h(px(4.)).text_color(rgb(theme::TEXT)))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            editor.menu_open = !editor.menu_open;
                        }
                        cx.notify();
                    })),
            )
            .child(dialog::collapse_button("editor-close").on_click(cx.listener(|shell, _, _, cx| shell.close_editor(cx))));

        let address = card("Address").child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(host_tile(host).size(px(36.)).rounded_lg())
                .child(self.input(Field::Address, "IP or Hostname", &host.address, false, window, cx).flex_1()),
        );

        let group_label = host.group.and_then(|g| self.vault.group(g)).map(|g| g.label.clone());
        let group_picker = div()
            .id("editor-group")
            .child(
                field_box(editor.groups_open)
                    .child(svg().path(icon("group")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                    .child(div().flex_1().text_color(rgb(if group_label.is_some() { theme::TEXT } else { theme::TEXT_MUTED })).child(group_label.clone().unwrap_or_else(|| "Parent Group".into())))
                    .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED))),
            )
            .on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.host_editor {
                    editor.groups_open = !editor.groups_open;
                }
                cx.notify();
            }));
        let groups = editor.groups_open.then(|| {
            let choice = |id: Option<GroupId>, label: String, cx: &mut Context<Self>| {
                menu_row(format!("editor-group-{id:?}"), &label, false, false).on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(editor) = &mut shell.host_editor {
                        editor.host.group = id;
                        editor.groups_open = false;
                    }
                    shell.autosave();
                    cx.notify();
                }))
            };
            let mut list = div().id("editor-group-list").occlude().max_h(px(220.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::INPUT)).shadow_lg().child(choice(None, "No group".into(), cx));
            for group in &self.vault.groups {
                // Nested groups read as a path: work / Cenix.
                let path = self.vault.breadcrumbs(Some(group.id)).iter().map(|g| g.label.as_str()).collect::<Vec<_>>().join(" / ");
                list = list.child(choice(Some(group.id), path, cx));
            }
            list
        });

        let tags_focused = editor.field == Field::Tags && self.editor_focus.is_focused(window);
        let tags = field_box(tags_focused)
            .id("editor-tags")
            .h_auto()
            .min_h(px(40.))
            .py_1()
            .flex_wrap()
            .child(svg().path(icon("tag")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
            .children(host.tags.iter().enumerate().map(|(i, tag)| {
                div()
                    .id(SharedString::from(format!("editor-tag-{i}")))
                    .px_2()
                    .py(px(2.))
                    .flex()
                    .items_center()
                    .gap_1()
                    .rounded_sm()
                    .bg(rgb(theme::SURFACE_ACTIVE))
                    .text_sm()
                    .text_color(rgb(theme::TEXT))
                    .child(tag.clone())
                    .child(svg().path(icon("close")).size(px(8.)).text_color(rgb(theme::TEXT_MUTED)))
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            if i < editor.host.tags.len() {
                                editor.host.tags.remove(i);
                            }
                        }
                        shell.autosave();
                        cx.notify();
                    }))
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_color(rgb(if editor.tag.is_empty() { theme::TEXT_MUTED } else { theme::TEXT }))
                    .child(if editor.tag.is_empty() && host.tags.is_empty() { "Tags".to_owned() } else { editor.tag.clone() })
                    .when(tags_focused, |d| d.child(caret())),
            )
            .on_click(cx.listener(|shell, _, window, cx| {
                if let Some(editor) = &mut shell.host_editor {
                    editor.field = Field::Tags;
                }
                window.focus(&shell.editor_focus);
                cx.notify();
            }));
        // Existing tags that match what is typed, as the legacy select suggests.
        let suggestions: Vec<String> = if tags_focused {
            let typed = editor.tag.to_lowercase();
            self.vault.all_tags().into_iter().filter(|t| !host.tags.contains(t) && t.to_lowercase().contains(&typed)).take(6).collect()
        } else {
            Vec::new()
        };
        // Drawn over the fields below, like the legacy select's menu, so nothing jumps.
        let suggestions = (!suggestions.is_empty()).then(|| {
            div().absolute().top_full().left_0().right_0().mt_1().p_2().rounded_md().bg(rgb(theme::INPUT)).shadow_lg().flex().flex_wrap().gap_1().children(suggestions.into_iter().map(|tag| {
                div()
                    .id(SharedString::from(format!("editor-suggest-{tag}")))
                    .px_2()
                    .py(px(2.))
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(theme::SURFACE_ACTIVE))
                    .text_sm()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .hover(|s| s.text_color(rgb(theme::TEXT)))
                    .child(tag.clone())
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(editor) = &mut shell.host_editor {
                            editor.host.tags.push(tag.clone());
                            editor.tag.clear();
                        }
                        shell.autosave();
                        cx.notify();
                    }))
            }))
        });

        let general = card("General")
            .child(self.input(Field::Label, "Label", &host.label, false, window, cx))
            // Lists open over the form, like the legacy autocomplete menus.
            .child(div().relative().child(group_picker).children(groups.map(|list| gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(list)))))
            .child(div().relative().child(tags).children(suggestions.map(gpui::deferred)))
            .child(self.render_backspace_row(cx));

        // Legacy: «SSH on [22] port», then «Credentials», then the other settings.
        let divider = || div().h(px(1.)).bg(rgb(theme::SURFACE_ACTIVE));
        let ssh = div()
            .p(px(15.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(theme::TEXT))
                    .child("SSH on")
                    .child(self.input(Field::Port, "22", &editor.port, false, window, cx).w(px(90.)))
                    .child("port"),
            )
            .child(divider())
            .child(div().font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child("Credentials"))
            .children(self.render_credentials(window, cx))
            .child(divider())
            .children(self.render_ssh_settings(cx));

        let can_connect = !editor.is_new && !host.address.trim().is_empty();
        let footer = div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).child(
            dialog::button("editor-connect", "Connect", Kind::Accent, !can_connect)
                .w_full()
                .when(can_connect, |d| d.on_click(cx.listener(|shell, _, window, cx| shell.connect_from_editor(window, cx)))),
        );

        let menu = editor.menu_open.then(|| {
            let id = host.id;
            let disabled = editor.is_new;
            div().absolute().top(px(56.)).right(px(50.)).child(
                menu_panel("editor-menu")
                    .child(menu_row("editor-menu-connect".into(), "Connect", false, disabled).when(!disabled, |d| {
                        d.on_click(cx.listener(|shell, _, window, cx| shell.connect_from_editor(window, cx)))
                    }))
                    .child(menu_row("editor-menu-duplicate".into(), "Duplicate", false, disabled).when(!disabled, |d| {
                        d.on_click(cx.listener(move |shell, _, window, cx| shell.duplicate_host(id, window, cx)))
                    }))
                    .child(menu_row("editor-menu-remove".into(), "Remove", true, disabled).when(!disabled, |d| {
                        d.on_click(cx.listener(|shell, _, _, cx| {
                            if let Some(editor) = &mut shell.host_editor {
                                editor.menu_open = false;
                                editor.confirm_remove = true;
                            }
                            cx.notify();
                        }))
                    })),
            )
        });

        let confirm = editor.confirm_remove.then(|| {
            let id = host.id;
            let close = dialog::close_button("remove-close").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.host_editor {
                    editor.confirm_remove = false;
                }
                cx.notify();
            }));
            let remove = dialog::button("remove-confirm", "Remove", Kind::Error, false)
                .on_click(cx.listener(move |shell, _, _, cx| shell.remove_host(id, cx)));
            dialog::backdrop(remove_card(host, close, remove))
        });
        // Legacy «Do you want to discard changes?» for an unfinished new host.
        let confirm = confirm.or_else(|| {
            editor.confirm_discard.then(|| {
                let close = dialog::close_button("discard-close").on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.host_editor {
                        editor.confirm_discard = false;
                    }
                    cx.notify();
                }));
                let discard = dialog::button("discard-confirm", "Discard changes", Kind::Error, false).on_click(cx.listener(|shell, _, _, cx| {
                    shell.host_editor = None;
                    cx.notify();
                }));
                dialog::backdrop(dialog::card(
                    "Do you want to discard changes?",
                    false,
                    close,
                    dialog::text("Not all required fields are filled. Do you want to finish editing and discard all changes?"),
                    vec![discard.into_any_element()],
                ))
            })
        });

        Some(
            div()
                .id("host-editor")
                .track_focus(&self.editor_focus)
                .on_key_down(cx.listener(Self::on_editor_key))
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
                .map(|d| match editor.panel {
                    // A sub-panel takes the form's place, as the legacy nested sliders do.
                    Some(panel) => d.child(div().flex_1().min_h_0().child(self.render_host_panel(panel, window, cx))),
                    None => d.child(div().id("editor-body").flex_1().overflow_y_scroll().p(px(15.)).flex().flex_col().gap(px(15.)).child(address).child(general).child(ssh)),
                })
                .child(footer)
                .children(menu),
        )
        // The confirmation covers the whole window, above the slider; one
        // wrapper either way so opening it does not replay the slide.
        .map(|slider| div().absolute().inset_0().child(dialog::slide_in(slider, WIDTH)).children(confirm).into_any_element())
    }

    pub fn open_host_menu(&mut self, host: HostId, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.host_menu = Some(HostMenu { host, position: event.position, confirm_remove: false });
        cx.notify();
    }

    /// Right-click menu of a host card, and its Remove confirmation.
    pub fn render_host_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.host_menu.as_ref()?;
        let host = self.vault.host(menu.host)?;
        let id = host.id;
        if menu.confirm_remove {
            let close = dialog::close_button("remove-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.host_menu = None;
                cx.notify();
            }));
            let remove =
                dialog::button("remove-confirm", "Remove", Kind::Error, false).on_click(cx.listener(move |shell, _, _, cx| shell.remove_host(id, cx)));
            return Some(dialog::backdrop(remove_card(host, close, remove)).into_any_element());
        }
        let viewport = window.viewport_size();
        let left = menu.position.x.min(viewport.width - px(MENU_WIDTH + 8.));
        let rows = 5 + self.workspace_tabs().len();
        let top = menu.position.y.min(viewport.height - px(rows as f32 * 32. + 16.));
        let items = menu_panel("host-menu")
            .child(menu_row("host-menu-quick".into(), "Quick Connect", false, false).on_click(cx.listener(move |shell, _, window, cx| {
                shell.host_menu = None;
                shell.connect_host(id, window, cx);
            })))
            .child(menu_row("host-menu-sftp".into(), "Connect with SFTP", false, false).on_click(cx.listener(move |shell, _, window, cx| {
                shell.host_menu = None;
                shell.open_sftp_for(id, window, cx);
            })))
            // Legacy «Add to Workspace»: one row per open workspace tab.
            .children(self.workspace_tabs().into_iter().map(|(tab, title)| {
                menu_row(format!("host-menu-workspace-{tab:?}").into(), &format!("Add to Workspace: {title}"), false, false).on_click(cx.listener(move |shell, _, window, cx| {
                    shell.host_menu = None;
                    shell.add_to_workspace(tab, id, window, cx);
                }))
            }))
            .child(menu_row("host-menu-edit".into(), "Edit", false, false).on_click(cx.listener(move |shell, _, window, cx| {
                shell.edit_host(id, window, cx);
            })))
            .child(menu_row("host-menu-duplicate".into(), "Duplicate", false, false).on_click(cx.listener(move |shell, _, window, cx| {
                shell.duplicate_host(id, window, cx);
            })))
            .child(menu_row("host-menu-remove".into(), "Remove", true, false).on_click(cx.listener(|shell, _, _, cx| {
                if let Some(menu) = &mut shell.host_menu {
                    menu.confirm_remove = true;
                }
                cx.notify();
            })));
        Some(
            div()
                .id("host-menu-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                    shell.host_menu = None;
                    cx.notify();
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(|shell, _, _, cx| {
                    shell.host_menu = None;
                    cx.notify();
                }))
                .child(div().absolute().left(left).top(top).child(items))
                .into_any_element(),
        )
    }
}
