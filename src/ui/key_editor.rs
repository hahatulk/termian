//! New Key / key details: the legacy Keychain slider with Label, Passphrase
//! (for encrypted keys), Private key, Public key and "Import from key file".
//! A key is saved once its private key reads correctly.

use gpui::{div, prelude::*, px, rgb, AnyElement, Context, Div, FontWeight, KeyDownEvent, PathPromptOptions, SharedString, Stateful, Window};
use termian_native::keys::{self, KeyCheck};
use termian_native::theme;
use termian_native::vault::SshKey;

use super::pointer::PointerFeedback;
use super::dialog::{self, Kind};
use super::text_input::{self, Mode};
use crate::Shell;

const WIDTH: f32 = 420.;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_defaults_and_algorithms_match_legacy() {
        let mut options = Keygen::default();
        assert_eq!(options.algorithm(), keys::KeyAlgorithm::Ed25519);
        assert_eq!(options.curve, 521);
        assert_eq!(options.bits, 4096);
        assert!(options.save_passphrase);
        options.kind = KeyType::Ecdsa;
        assert_eq!(options.algorithm(), keys::KeyAlgorithm::Ecdsa521);
        options.curve = 384;
        assert_eq!(options.algorithm(), keys::KeyAlgorithm::Ecdsa384);
        options.kind = KeyType::Rsa;
        assert_eq!(options.algorithm(), keys::KeyAlgorithm::Rsa(4096));
    }

    #[test]
    fn generated_labels_do_not_duplicate_existing_keys() {
        assert_eq!(unique_label("ED25519", &["ED25519", "ED25519 (1)"]), "ED25519 (2)");
        assert_eq!(unique_label("RSA", &["ED25519"]), "RSA");
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyType { Ed25519, Ecdsa, Rsa }

struct Keygen {
    kind: KeyType,
    curve: usize,
    bits: usize,
    save_passphrase: bool,
    generating: bool,
    token: String,
    error: Option<String>,
}

impl Default for Keygen {
    fn default() -> Self {
        Self { kind: KeyType::Ed25519, curve: 521, bits: 4096, save_passphrase: true, generating: false, token: uuid::Uuid::new_v4().to_string(), error: None }
    }
}

impl Keygen {
    fn algorithm(&self) -> keys::KeyAlgorithm {
        match self.kind {
            KeyType::Ed25519 => keys::KeyAlgorithm::Ed25519,
            KeyType::Ecdsa => match self.curve {
                256 => keys::KeyAlgorithm::Ecdsa256,
                384 => keys::KeyAlgorithm::Ecdsa384,
                _ => keys::KeyAlgorithm::Ecdsa521,
            },
            KeyType::Rsa => keys::KeyAlgorithm::Rsa(self.bits),
        }
    }
}

fn unique_label(base: &str, existing: &[&str]) -> String {
    if !existing.contains(&base) { return base.to_owned(); }
    for n in 1.. {
        let candidate = format!("{base} ({n})");
        if !existing.contains(&candidate.as_str()) { return candidate; }
    }
    unreachable!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Label,
    Passphrase,
    PrivateKey,
    Certificate,
}

pub struct KeyEditor {
    key: SshKey,
    is_new: bool,
    created_here: bool,
    field: Field,
    confirm_remove: bool,
    generation: Option<Keygen>,
}

fn key_mode(field: Field) -> Mode {
    match field {
        Field::Label => Mode::Line,
        Field::Passphrase => Mode::Secret,
        Field::PrivateKey => Mode::PrivateKey,
        Field::Certificate => Mode::Multiline,
    }
}

impl Shell {
    pub fn new_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        let key = SshKey { id: self.vault.next_id(), ..SshKey::default() };
        self.identity_editor = None;
        self.keychain_menu = None;
        self.key_editor = Some(KeyEditor { key, is_new: true, created_here: true, field: Field::PrivateKey, confirm_remove: false, generation: None });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    pub fn new_generated_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        self.new_key(window, cx);
        if let Some(editor) = &mut self.key_editor {
            editor.generation = Some(Keygen::default());
            editor.field = Field::Label;
        }
    }

    fn generate_key(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.key_editor else { return };
        let Some(options) = &mut editor.generation else { return };
        if options.generating { return; }
        options.generating = true;
        options.error = None;
        let token = options.token.clone();
        let algorithm = options.algorithm();
        let passphrase = editor.key.passphrase.clone().unwrap_or_default();
        let task = cx.background_spawn(async move { keys::generate(algorithm, &passphrase) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, cx| {
                let Some(editor) = &mut shell.key_editor else { return };
                let Some(options) = &mut editor.generation else { return };
                if options.token != token { return; }
                options.generating = false;
                match result {
                    Err(error) => options.error = Some(error),
                    Ok(pair) => {
                        editor.key.kind = termian_native::vault::key_kind(&pair.public_key);
                        if editor.key.label.trim().is_empty() {
                            editor.key.label = unique_label(&editor.key.kind, &shell.vault.keys.iter().map(|k| k.label.as_str()).collect::<Vec<_>>());
                        }
                        editor.key.private_key = Some(pair.private_key);
                        editor.key.public_key = Some(pair.public_key);
                        if !options.save_passphrase { editor.key.passphrase = None; }
                        editor.is_new = false;
                        editor.generation = None;
                        shell.vault.save_key(editor.key.clone());
                        shell.save_vault();
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    pub fn edit_key(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        let Some(key) = self.vault.key(id).cloned() else { return };
        self.identity_editor = None;
        self.key_editor = Some(KeyEditor { key, is_new: false, created_here: false, field: Field::Label, confirm_remove: false, generation: None });
        window.focus(&self.editor_focus);
        cx.notify();
    }

    /// Saves when the private key reads; fills in the type and public key from it.
    fn save_key_draft(&mut self) {
        let Some(editor) = &mut self.key_editor else { return };
        if editor.generation.is_some() { return; }
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
        if editor.generation.as_ref().is_some_and(|g| g.generating) { return; }
        if keystroke.key == "tab" && editor.generation.is_some() {
            editor.field = if editor.field == Field::Label { Field::Passphrase } else { Field::Label };
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if keystroke.key == "enter" && editor.generation.is_some() {
            self.generate_key(cx);
            cx.stop_propagation();
            return;
        }
        if keystroke.key == "tab" {
            editor.field = match editor.field {
                Field::Label => Field::Passphrase,
                Field::Passphrase => Field::PrivateKey,
                Field::PrivateKey => Field::Certificate,
                Field::Certificate => Field::Label,
            };
            cx.stop_propagation();
            cx.notify();
            return;
        }
        let value = match editor.field {
            Field::Label => &mut editor.key.label,
            Field::Passphrase => editor.key.passphrase.get_or_insert_with(String::new),
            Field::PrivateKey => editor.key.private_key.get_or_insert_with(String::new),
            Field::Certificate => editor.key.certificate.get_or_insert_with(String::new),
        };
        let mode = key_mode(editor.field);
        if !matches!(text_input::edit(("key-editor", editor.field), Some(value), keystroke, mode, cx), super::dialog::Key::Edited) {
            return;
        }
        if editor.key.passphrase.as_deref() == Some("") {
            editor.key.passphrase = None;
        }
        if editor.key.certificate.as_deref().is_some_and(|c| c.trim().is_empty()) {
            editor.key.certificate = None;
        }
        self.save_key_draft();
        cx.stop_propagation();
        cx.notify();
    }

    fn key_field(&self, field: Field, label: &str, value: &str, masked: bool, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let focused = self.key_editor.as_ref().is_some_and(|e| e.field == field) && self.editor_focus.is_focused(window);
        let multiline = matches!(field, Field::PrivateKey | Field::Certificate);
        let key = ("key-editor", field);
        let mode = if masked { Mode::Secret } else { key_mode(field) };
        let input = div().id(SharedString::from(format!("key-input-{label}")))
            .min_h(px(40.))
            .when(multiline, |d| d.min_h(px(120.)).max_h(px(260.)).overflow_y_scroll())
            .px_3().py_2().rounded_md().bg(rgb(theme::INPUT)).border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
            .when(multiline, |d| d.font_family("Menlo").text_xs())
            .child(text_input::text(key, value, "", focused, mode, cx));
        let input = text_input::field(input, key, mode, move |shell, window, _| {
            if let Some(editor) = &mut shell.key_editor {
                editor.field = field;
            }
            window.focus(&shell.editor_focus);
        }, cx);
        div().id(SharedString::from(format!("key-field-{label}"))).flex().flex_col().gap_1()
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(label.to_owned()))
            .child(input)
    }

    fn render_key_generation(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let editor = self.key_editor.as_ref().unwrap();
        let options = editor.generation.as_ref().unwrap();
        let mut form = div().flex().flex_col().gap(px(15.))
            .child(self.key_field(Field::Label, "Label", &editor.key.label, false, window, cx))
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Key type"));
        let mut types = div().flex().gap_2();
        for (kind, label) in [(KeyType::Ed25519, "ED25519"), (KeyType::Ecdsa, "ECDSA"), (KeyType::Rsa, "RSA")] {
            types = types.child(dialog::button(&format!("keygen-{label}"), label, if kind == options.kind { Kind::Accent } else { Kind::Regular }, false).on_click(cx.listener(move |shell, _, _, cx| {
                if let Some(g) = shell.key_editor.as_mut().and_then(|e| e.generation.as_mut()) { if !g.generating { g.kind = kind; } }
                cx.notify();
            })));
        }
        form = form.child(types);
        if options.kind != KeyType::Ed25519 {
            let values: &[usize] = if options.kind == KeyType::Ecdsa { &[521, 384, 256] } else { &[4096, 2048, 1024] };
            let mut choices = div().flex().gap_2();
            for &value in values {
                let selected = if options.kind == KeyType::Ecdsa { options.curve == value } else { options.bits == value };
                choices = choices.child(dialog::button(&format!("keygen-size-{value}"), &value.to_string(), if selected { Kind::Accent } else { Kind::Regular }, false).on_click(cx.listener(move |shell, _, _, cx| {
                    if let Some(g) = shell.key_editor.as_mut().and_then(|e| e.generation.as_mut()) {
                        if !g.generating { if g.kind == KeyType::Ecdsa { g.curve = value; } else { g.bits = value; } }
                    }
                    cx.notify();
                })));
            }
            form = form.child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(if options.kind == KeyType::Ecdsa { "Elliptic curve size (bits)" } else { "Key size (bits)" })).child(choices);
        }
        let content = if options.generating {
            div().text_color(rgb(theme::TEXT_MUTED)).child("Generating a key...")
        } else {
            form.child(self.key_field(Field::Passphrase, "Passphrase", editor.key.passphrase.as_deref().unwrap_or(""), true, window, cx))
                .child(div().id("keygen-save-passphrase").flex().items_center().justify_between().rounded_sm().icon_feedback().text_color(rgb(theme::TEXT))
                    .child("Save passphrase").child(super::settings_terminal::toggle(options.save_passphrase))
                    .on_click(cx.listener(|shell, _, _, cx| {
                        if let Some(g) = shell.key_editor.as_mut().and_then(|e| e.generation.as_mut()) { g.save_passphrase = !g.save_passphrase; }
                        cx.notify();
                    })))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("Encryption uses OpenSSH defaults. Cipher and rounds controls are not available."))
                .children(options.error.as_ref().map(|error| div().text_sm().text_color(rgb(theme::ERROR)).child(error.clone())))
                .child(dialog::button("keygen-generate", "Generate & save", Kind::Accent, false).w_full().on_click(cx.listener(|shell, _, _, cx| shell.generate_key(cx))))
        };
        div().id("key-generator").track_focus(&self.editor_focus).on_key_down(cx.listener(Self::on_key_editor_key))
            .absolute().top_0().right(self.editor_inset()).bottom_0().w(px(WIDTH)).occlude().flex().flex_col().bg(rgb(theme::BACKGROUND)).border_l_1().border_color(rgb(theme::TITLE_BAR)).shadow_lg()
            .child(div().h(px(64.)).px(px(15.)).flex().items_center().gap_2().border_b_1().border_color(rgb(theme::TITLE_BAR))
                .child(div().flex_1().text_lg().text_color(rgb(theme::TEXT)).child("Generate Key"))
                .child(dialog::collapse_button("keygen-close").on_click(cx.listener(|shell, _, _, cx| { shell.key_editor = None; cx.notify(); }))))
            .child(div().id("keygen-body").flex_1().overflow_y_scroll().p(px(15.)).child(content))
            .into_any_element()
    }

    pub fn render_key_editor(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.key_editor.as_ref()?;
        if editor.generation.is_some() { return Some(self.render_key_generation(window, cx)); }
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
            .child(dialog::collapse_button("key-close").on_click(cx.listener(|shell, _, _, cx| {
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
        // As in legacy: one OpenSSH certificate per key, signed for its public key.
        let certificate = key.certificate.as_deref().unwrap_or("");
        form = form.child(self.key_field(Field::Certificate, "Certificate", certificate, false, window, cx));
        if !certificate.is_empty() && keys::parse_certificate(certificate).is_none() {
            form = form.child(div().text_xs().text_color(rgb(theme::ERROR)).child("Unknown certificate format"));
        }
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
            div().p(px(15.)).rounded_lg().bg(rgb(theme::SURFACE)).flex().flex_col().gap_2().child(
                dialog::button("key-import", "Import from key file", Kind::Accent, false)
                    .w_full()
                    .on_click(cx.listener(|shell, _, window, cx| shell.import_key_file(window, cx))),
            ).when(editor.is_new, |d| d.child(
                dialog::button("key-generate-open", "Generate Key", Kind::Regular, false).w_full()
                    .on_click(cx.listener(|shell, _, window, cx| shell.new_generated_key(window, cx)))
            ))
        });
        // "Export to host", as in the legacy key form; a saved key only.
        let export = (!editor.is_new && self.vault.key(key.id).is_some()).then(|| {
            let id = key.id;
            div().p(px(15.)).rounded_lg().bg(rgb(theme::SURFACE)).child(
                dialog::button("key-export", "Export to host", Kind::Accent, false).w_full().on_click(cx.listener(move |shell, _, window, cx| shell.export_key(id, window, cx))),
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
            .right(self.editor_inset())
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
            .child(div().id("key-body").flex_1().overflow_y_scroll().p(px(15.)).flex().flex_col().gap(px(15.)).child(form).children(import).children(export))
            .children(remove.map(|button| div().flex_none().p(px(15.)).border_t_1().border_color(rgb(theme::TITLE_BAR)).flex().justify_end().child(button)));
        Some(match confirm {
            Some(confirm) => div().absolute().inset_0().child(slider).child(confirm).into_any_element(),
            None => slider.into_any_element(),
        })
    }
}
