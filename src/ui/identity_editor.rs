//! New Identity / identity details: the legacy Keychain form with Label,
//! Username*, Password and Key. Saved once it has a username.

use gpui::{div, prelude::*, px, rgb, svg, AnyElement, Context, Div, FontWeight, KeyDownEvent, SharedString, Window};
use termian_native::theme;
use termian_native::vault::Identity;

use super::assets::icon;
use super::dialog::{self, Key, Kind};
use super::host_editor::{caret, field_box, menu_row};
use super::hosts::tile;
use crate::Shell;

const WIDTH: f32 = 420.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Label,
    Username,
    Password,
}

pub struct IdentityEditor {
    identity: Identity,
    created_here: bool,
    field: Field,
    keys_open: bool,
    confirm_remove: bool,
}

impl Shell {
    /// The KEY split button's menu: New Key, New Identity.
    pub fn render_keychain_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let at = self.keychain_menu?;
        let items = super::host_editor::menu_panel("keychain-menu")
            .child(menu_row("keychain-menu-key".into(), "New Key", false, false).on_click(cx.listener(|shell, _, window, cx| {
                shell.keychain_menu = None;
                shell.new_key(window, cx);
            })))
            .child(menu_row("keychain-menu-generate".into(), "Generate Key", false, false).on_click(cx.listener(|shell, _, window, cx| shell.new_generated_key(window, cx))))
            .child(menu_row("keychain-menu-identity".into(), "New Identity", false, false).on_click(cx.listener(|shell, _, window, cx| shell.new_identity(window, cx))));
        // Window coordinates; the Vaults content starts below the tab strip, beside the sidebar.
        Some(
            gpui::deferred(
                div()
                    .id("keychain-menu-dismiss")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|shell, _, _, cx| {
                        shell.keychain_menu = None;
                        cx.notify();
                    }))
                    .child(gpui::anchored().position(at).child(items)),
            )
            .into_any_element(),
        )
    }

    pub fn new_identity(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let identity = Identity { id: self.vault.next_id(), ..Identity::default() };
        self.key_editor = None;
        self.keychain_menu = None;
        self.identity_editor = Some(IdentityEditor { identity, created_here: true, field: Field::Label, keys_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    pub fn edit_identity(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(identity) = self.vault.identity(id).cloned() else { return };
        self.key_editor = None;
        self.identity_editor = Some(IdentityEditor { identity, created_here: false, field: Field::Label, keys_open: false, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    fn save_identity_draft(&mut self) {
        let Some(editor) = &self.identity_editor else { return };
        // Username is required, as in the legacy form.
        if editor.identity.username.trim().is_empty() {
            return;
        }
        let mut identity = editor.identity.clone();
        if identity.label.trim().is_empty() {
            identity.label = identity.username.clone();
        }
        self.vault.save_identity(identity);
        self.save_vault();
    }

    fn on_identity_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(editor) = &mut self.identity_editor else { return };
        if editor.confirm_remove {
            if event.keystroke.key == "escape" {
                editor.confirm_remove = false;
                cx.notify();
            }
            return;
        }
        if event.keystroke.key == "tab" {
            editor.field = match editor.field {
                Field::Label => Field::Username,
                Field::Username => Field::Password,
                Field::Password => Field::Label,
            };
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let value = match editor.field {
            Field::Label => &mut editor.identity.label,
            Field::Username => &mut editor.identity.username,
            Field::Password => editor.identity.password.get_or_insert_with(String::new),
        };
        match dialog::edit(Some(value), &event.keystroke, paste) {
            Key::Edited => {
                if editor.identity.password.as_deref() == Some("") {
                    editor.identity.password = None;
                }
                self.save_identity_draft();
            }
            Key::Escape => self.identity_editor = None,
            Key::Enter | Key::Ignored => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn identity_input(&self, field: Field, placeholder: &str, value: &str, masked: bool, window: &Window, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        let focused = self.identity_editor.as_ref().is_some_and(|e| e.field == field) && self.editor_focus.is_focused(window);
        let shown = if masked { "•".repeat(value.chars().count()) } else { value.to_owned() };
        let empty = shown.is_empty();
        field_box(focused)
            .id(SharedString::from(format!("identity-{placeholder}")))
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
                if let Some(editor) = &mut shell.identity_editor {
                    editor.field = field;
                    editor.keys_open = false;
                }
                window.focus(&shell.editor_focus);
                cx.notify();
            }))
    }

    pub fn render_identity_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.identity_editor.as_ref()?;
        let identity = &editor.identity;
        let header = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(if editor.created_here { "New Identity" } else { "Identity" }))
            .child(dialog::collapse_button("identity-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.identity_editor = None;
                cx.notify();
            })));

        let chosen = identity.key.and_then(|k| self.vault.key(k)).map(|k| format!("{} ({})", k.label, k.kind));
        let picker = field_box(editor.keys_open)
            .id("identity-key")
            .child(svg().path(icon("keychain")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
            .child(div().flex_1().text_color(rgb(if chosen.is_some() { theme::TEXT } else { theme::TEXT_MUTED })).child(chosen.unwrap_or_else(|| "Key".into())))
            .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)))
            .on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.identity_editor {
                    editor.keys_open = !editor.keys_open;
                }
                cx.notify();
            }));
        let list = editor.keys_open.then(|| {
            let choice = |id: Option<u64>, label: String, usable: bool, cx: &mut Context<Self>| {
                menu_row(format!("identity-key-{id:?}"), &label, false, !usable).when(usable, |d| {
                    d.on_click(cx.listener(move |shell, _, _, cx| {
                        if let Some(editor) = &mut shell.identity_editor {
                            editor.identity.key = id;
                            editor.keys_open = false;
                        }
                        shell.save_identity_draft();
                        cx.notify();
                    }))
                })
            };
            let mut list = div().id("identity-key-list").max_h(px(220.)).overflow_y_scroll().p_1().rounded_md().bg(rgb(theme::INPUT)).shadow_lg().child(choice(None, "No key".into(), true, cx));
            for key in &self.vault.keys {
                let usable = key.private_key.is_some();
                let note = if usable { "" } else { " — not available in this client" };
                list = list.child(choice(Some(key.id), format!("{} ({}){note}", key.label, key.kind), usable, cx));
            }
            gpui::deferred(div().absolute().top_full().left_0().right_0().mt_1().child(list))
        });

        let form = div()
            .p(px(15.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .child(div().flex().items_center().gap_3().child(tile("identity", theme::TILE_BLUE).size(px(36.)).rounded_lg()).child(self.identity_input(Field::Label, "Label", &identity.label, false, window, cx).flex_1()))
            .child(self.identity_input(Field::Username, "Username*", &identity.username, false, window, cx))
            .child(self.identity_input(Field::Password, "Password", identity.password.as_deref().unwrap_or(""), true, window, cx))
            .child(div().relative().child(picker).children(list));

        let id = identity.id;
        let saved = self.vault.identity(id).is_some();
        let footer = saved.then(|| {
            div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).flex().justify_end().child(
                dialog::button("identity-remove", "Remove", Kind::Error, false).on_click(cx.listener(|shell, _, _, cx| {
                    if let Some(editor) = &mut shell.identity_editor {
                        editor.confirm_remove = true;
                    }
                    cx.notify();
                })),
            )
        });
        let confirm = editor.confirm_remove.then(|| {
            let close = dialog::close_button("identity-remove-close").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.identity_editor {
                    editor.confirm_remove = false;
                }
                cx.notify();
            }));
            dialog::backdrop(dialog::card("Remove identity", false, close, dialog::text("Are you sure you want to delete this identity?"), vec![
                dialog::button("identity-remove-confirm", "Remove", Kind::Error, false)
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.vault.remove_identity(id);
                        shell.save_vault();
                        shell.identity_editor = None;
                        cx.notify();
                    }))
                    .into_any_element(),
            ]))
        });
        let slider = div()
            .id("identity-editor")
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_identity_key))
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
            .child(div().flex_1().p(px(15.)).child(form))
            .children(footer);
        Some(match confirm {
            Some(confirm) => div().absolute().inset_0().child(slider).child(confirm).into_any_element(),
            None => slider.into_any_element(),
        })
    }
}
