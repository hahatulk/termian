//! New Key / key details: the legacy Keychain slider with Label, Passphrase
//! (for encrypted keys), Private key, Public key and "Import from key file".
//! A key is saved once its private key reads correctly.

use gpui::{div, prelude::*, px, rgb, AnyElement, Context, Div, FontWeight, KeyDownEvent, PathPromptOptions, SharedString, Stateful, Window};
use termian_native::keys::{self, KeyCheck};
use termian_native::theme;
use termian_native::vault::SshKey;

use super::dialog::{self, Kind};
use crate::Shell;

const WIDTH: f32 = 420.;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Label,
    Passphrase,
    PrivateKey,
}

pub struct KeyEditor {
    key: SshKey,
    is_new: bool,
    created_here: bool,
    field: Field,
    confirm_remove: bool,
}

fn caret() -> Div {
    div().w(px(1.5)).h(px(14.)).bg(rgb(theme::TEXT))
}

impl Shell {
    pub fn new_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = SshKey { id: self.vault.next_id(), ..SshKey::default() };
        self.key_editor = Some(KeyEditor { key, is_new: true, created_here: true, field: Field::PrivateKey, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    pub fn edit_key(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.vault.key(id).cloned() else { return };
        self.key_editor = Some(KeyEditor { key, is_new: false, created_here: false, field: Field::Label, confirm_remove: false });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    /// Saves when the private key reads; fills in the type and public key from it.
    fn save_key_draft(&mut self) {
        let Some(editor) = &mut self.key_editor else { return };
        let key = &mut editor.key;
        if let KeyCheck::Valid { kind, public_key } = keys::check(key.private_key.as_deref().unwrap_or(""), key.passphrase.as_deref()) {
            key.kind = kind;
            key.public_key = Some(public_key);
        } else if editor.is_new {
            return;
        }
        if key.label.trim().is_empty() {
            key.label = key.kind.clone();
        }
        editor.is_new = false;
        self.vault.save_key(editor.key.clone());
        self.save_vault();
    }

    fn import_key_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Import".into()) });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = answer.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let read = std::fs::read_to_string(&path);
            let _ = this.update_in(cx, |shell, _, cx| {
                if let (Ok(text), Some(editor)) = (read, &mut shell.key_editor) {
                    editor.key.private_key = Some(text);
                    if editor.key.label.is_empty() {
                        editor.key.label = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    }
                    shell.save_key_draft();
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn on_key_editor_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let paste = cx.read_from_clipboard().and_then(|item| item.text());
        let Some(editor) = &mut self.key_editor else { return };
        let keystroke = &event.keystroke;
        if keystroke.key == "escape" {
            if editor.confirm_remove {
                editor.confirm_remove = false;
            } else {
                self.key_editor = None;
            }
            cx.notify();
            return;
        }
        if keystroke.key == "tab" {
            editor.field = match editor.field {
                Field::Label => Field::Passphrase,
                Field::Passphrase => Field::PrivateKey,
                Field::PrivateKey => Field::Label,
            };
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let value = match editor.field {
            Field::Label => &mut editor.key.label,
            Field::Passphrase => editor.key.passphrase.get_or_insert_with(String::new),
            Field::PrivateKey => editor.key.private_key.get_or_insert_with(String::new),
        };
        let multiline = editor.field == Field::PrivateKey;
        match keystroke.key.as_str() {
            _ if keystroke.modifiers.platform && keystroke.key == "v" => {
                let text = paste.unwrap_or_default();
                // The private key keeps its lines; other fields take the first one.
                value.push_str(if multiline { &text } else { text.lines().next().unwrap_or("") });
            }
            _ if keystroke.modifiers.platform || keystroke.modifiers.control => return,
            "backspace" => {
                value.pop();
            }
            "enter" if multiline => value.push('\n'),
            "enter" => {}
            _ => match keystroke.key_char.as_deref() {
                Some(text) if !text.chars().any(char::is_control) => value.push_str(text),
                _ => return,
            },
        }
        if editor.key.passphrase.as_deref() == Some("") {
            editor.key.passphrase = None;
        }
        self.save_key_draft();
        cx.stop_propagation();
        cx.notify();
    }

    fn key_field(&self, field: Field, label: &str, value: &str, masked: bool, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let focused = self.key_editor.as_ref().is_some_and(|e| e.field == field) && self.editor_focus.is_focused(window);
        let multiline = field == Field::PrivateKey;
        let shown = if masked { "•".repeat(value.chars().count()) } else { value.to_owned() };
        // Base64 lines have no spaces to wrap at, so the key is cut into rows that fit.
        let width = if multiline { 46 } else { usize::MAX };
        let lines: Vec<String> = if shown.is_empty() {
            vec![String::new()]
        } else {
            shown
                .lines()
                .flat_map(|line| {
                    let chars: Vec<char> = line.chars().collect();
                    if chars.is_empty() { vec![String::new()] } else { chars.chunks(width).map(|c| c.iter().collect()).collect() }
                })
                .collect()
        };
        let last = lines.len() - 1;
        div()
            .id(SharedString::from(format!("key-field-{label}")))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(label.to_owned()))
            .child(
                div()
                    .id(SharedString::from(format!("key-input-{label}")))
                    .min_h(px(40.))
                    .when(multiline, |d| d.min_h(px(120.)).max_h(px(260.)).overflow_y_scroll())
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(theme::INPUT))
                    .border_1()
                    .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
                    .text_color(rgb(theme::TEXT))
                    .when(multiline, |d| d.font_family("Menlo").text_xs())
                    .children(lines.into_iter().enumerate().map(move |(i, line)| {
                        div().flex().items_center().min_h(px(16.)).child(line).when(focused && i == last, |d| d.child(caret()))
                    }))
                    .on_click(cx.listener(move |shell, _, window, cx| {
                        if let Some(editor) = &mut shell.key_editor {
                            editor.field = field;
                        }
                        window.focus(&shell.editor_focus);
                        cx.notify();
                    })),
            )
    }

    pub fn render_key_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.key_editor.as_ref()?;
        let key = &editor.key;
        let private_key = key.private_key.as_deref().unwrap_or("");
        let check = keys::check(private_key, key.passphrase.as_deref());
        let show_passphrase = keys::is_encrypted(private_key) || key.passphrase.is_some();
        let title = if editor.created_here { "New Key" } else { "Key" };

        let header = div()
            .h(px(64.))
            .flex_none()
            .px(px(15.))
            .flex()
            .items_center()
            .gap_2()
            .border_b_1()
            .border_color(rgb(theme::TITLE_BAR))
            .child(div().flex_1().text_lg().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(title))
            .child(dialog::close_button("key-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.key_editor = None;
                cx.notify();
            })));

        let mut form = div()
            .p(px(15.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .rounded_lg()
            .bg(rgb(theme::SURFACE))
            .child(self.key_field(Field::Label, "Label", &key.label, false, window, cx));
        if show_passphrase {
            form = form.child(self.key_field(Field::Passphrase, "Passphrase", key.passphrase.as_deref().unwrap_or(""), true, window, cx));
        }
        form = form.child(self.key_field(Field::PrivateKey, "Private key", private_key, false, window, cx));
        let status = match &check {
            KeyCheck::Invalid => Some(("Incorrect format", theme::ERROR)),
            KeyCheck::NeedsPassphrase => Some(("Enter the passphrase of this key", theme::TEXT_MUTED)),
            _ => None,
        };
        form = form.children(status.map(|(text, color)| div().text_xs().text_color(rgb(color)).child(text)));
        if let Some(public_key) = &key.public_key {
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Public key"))
                    .child(div().px_3().py_2().rounded_md().bg(rgb(theme::INPUT)).font_family("Menlo").text_xs().text_color(rgb(theme::TEXT)).child(public_key.clone())),
            );
        }

        let import = editor.created_here.then(|| {
            div().p(px(15.)).rounded_lg().bg(rgb(theme::SURFACE)).child(
                dialog::button("key-import", "Import from key file", Kind::Accent, false)
                    .w_full()
                    .on_click(cx.listener(|shell, _, window, cx| shell.import_key_file(window, cx))),
            )
        });
        let remove = (!editor.is_new).then(|| {
            dialog::button("key-remove", "Remove", Kind::Error, false).on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.key_editor {
                    editor.confirm_remove = true;
                }
                cx.notify();
            }))
        });

        let confirm = editor.confirm_remove.then(|| {
            let id = key.id;
            let close = dialog::close_button("key-remove-close").on_click(cx.listener(|shell, _, _, cx| {
                if let Some(editor) = &mut shell.key_editor {
                    editor.confirm_remove = false;
                }
                cx.notify();
            }));
            let body = dialog::text("Are you sure you want to delete this key?");
            dialog::backdrop(dialog::card("Remove key", false, close, body, vec![
                dialog::button("key-remove-confirm", "Remove", Kind::Error, false)
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.vault.remove_key(id);
                        shell.save_vault();
                        shell.key_editor = None;
                        cx.notify();
                    }))
                    .into_any_element(),
            ]))
        });

        let slider = div()
            .id("key-editor")
            .track_focus(&self.editor_focus)
            .on_key_down(cx.listener(Self::on_key_editor_key))
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
            .child(div().id("key-body").flex_1().overflow_y_scroll().p(px(15.)).flex().flex_col().gap(px(15.)).child(form).children(import))
            .children(remove.map(|button| div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).flex().justify_end().child(button)));
        Some(match confirm {
            Some(confirm) => div().absolute().inset_0().child(slider).child(confirm).into_any_element(),
            None => slider.into_any_element(),
        })
    }
}
