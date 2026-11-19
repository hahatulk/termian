//! Settings → Shortcuts (legacy `index-347b6706`): the scheme name and
//! picker («Disabled», the presets, user schemes with «Remove»), then
//! «Shortcut / Action» rows. A row takes a new combination when clicked:
//! Enter keeps it, Backspace disables the action, Escape cancels; the tab and
//! pane rows take modifiers only. Editing a preset makes «<name> copy».

use gpui::{div, prelude::*, px, rgb, rgba, AnyElement, Context, Div, FontWeight, KeyDownEvent, ModifiersChangedEvent, SharedString, Window};
use termian_native::shortcuts::{self, ShortcutSettings, ACTIONS};
use termian_native::theme;

use super::text_input::{self, Mode};
use super::settings::{action_button, Field, Menu, SettingsWindow, MUTED};

const TAB_MOD: &str = "mod";
const PANE_MOD: &str = "paneMod";

/// A row being given a new combination.
#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    kind: String,
    old: Option<String>,
    new: Option<String>,
}

impl Recording {
    fn modifiers_only(&self) -> bool {
        self.kind == TAB_MOD || self.kind == PANE_MOD
    }
}

/// Key caps, as the legacy `KeyboardKey` draws them.
fn caps(shortcut: &str) -> Div {
    // Legacy key caps: dark, muted glyphs; word keys (Shift, Ctrl, …) small and upper-case.
    div().flex().gap(px(6.)).children(shortcuts::display(shortcut).into_iter().map(|cap| {
        let word = cap.chars().count() > 1 && cap.chars().all(char::is_alphabetic);
        let cap = if word { cap.to_uppercase() } else { cap };
        div()
            .min_w(px(22.))
            .h(px(22.))
            .px(px(5.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .bg(rgb(0x41455c))
            .text_size(px(if word { 8. } else { 13. }))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xb4b6c3))
            .child(cap)
    }))
}

impl SettingsWindow {
    fn shortcut_settings(&self, cx: &gpui::App) -> ShortcutSettings {
        self.shell.read(cx).vault.shortcuts.clone()
    }

    fn change_shortcuts(&self, cx: &mut Context<Self>, change: impl FnOnce(&mut ShortcutSettings) + 'static) {
        self.shell.update(cx, |shell, cx| shell.update_shortcuts(change, cx));
    }

    pub(super) fn rename_scheme(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some((current, _)) = self.shortcut_settings(cx).current_scheme() else { return };
        let name = name.to_owned();
        self.change_shortcuts(cx, move |s| s.rename(&current, &name));
    }

    fn start_recording(&mut self, kind: &str, old: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.recording.as_ref().is_some_and(|r| r.kind == kind) {
            return;
        }
        self.submit_recording(cx);
        self.commit_edit(cx);
        self.recording = Some(Recording { kind: kind.into(), old: old.clone(), new: old });
        window.focus(&self.focus);
        cx.notify();
    }

    /// Keeps the recorded combination (legacy `onSubmitBinding`).
    pub(super) fn submit_recording(&mut self, cx: &mut Context<Self>) {
        let Some(recording) = self.recording.take() else { return };
        if recording.new == recording.old {
            return;
        }
        match (recording.kind.as_str(), recording.new) {
            // The tab and pane modifiers cannot be disabled.
            (TAB_MOD | PANE_MOD, None) => {}
            (TAB_MOD, Some(modifier)) => self.change_shortcuts(cx, move |s| s.set_tab_mod(&modifier)),
            (PANE_MOD, Some(modifier)) => self.change_shortcuts(cx, move |s| s.set_pane_mod(&modifier)),
            (kind, new) => {
                let kind = kind.to_owned();
                self.change_shortcuts(cx, move |s| s.set_shortcut(&kind, new));
            }
        }
        cx.notify();
    }

    pub(super) fn record_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(recording) = self.recording.as_mut() else { return };
        let keystroke = &event.keystroke;
        let m = keystroke.modifiers;
        let plain = !(m.platform || m.control || m.alt || m.shift);
        match keystroke.key.as_str() {
            "enter" if plain => self.submit_recording(cx),
            "escape" if plain => self.recording = None,
            "backspace" if plain && !recording.modifiers_only() => recording.new = None,
            _ if recording.modifiers_only() => {}
            key => recording.new = Some(shortcuts::record(m.control, m.alt, m.shift, m.platform, key)),
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// The tab and pane rows take the modifiers held down.
    pub(super) fn record_modifiers(&mut self, event: &ModifiersChangedEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(recording) = self.recording.as_mut().filter(|r| r.modifiers_only()) else { return };
        let m = event.modifiers;
        let held = shortcuts::record(m.control, m.alt, m.shift, m.platform, "");
        if !held.is_empty() {
            recording.new = Some(held);
            cx.notify();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn shortcut_row(&self, kind: &str, keys: Option<String>, suffix: Option<&str>, description: &str, conflict: Option<&str>, cx: &mut Context<Self>) -> AnyElement {
        let recording = self.recording.as_ref().filter(|r| r.kind == kind);
        let shown = recording.map_or(keys.clone(), |r| r.new.clone());
        let editing = recording.is_some();
        let kind_owned = kind.to_owned();
        let keys_cell = match (&shown, editing) {
            (Some(keys), _) => caps(keys).into_any_element(),
            (None, true) => div().text_size(px(13.)).text_color(rgb(MUTED)).child("Press keys…").into_any_element(),
            (None, false) => div().text_size(px(13.)).text_color(rgb(MUTED)).child("Disabled").into_any_element(),
        };
        div()
            .id(SharedString::from(format!("shortcut-{kind}")))
            .h(px(40.))
            .px(px(20.))
            .flex()
            .items_center()
            .when(editing, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
            .when(!editing, |d| d.hover(|s| s.bg(rgb(theme::SURFACE))))
            .cursor_pointer()
            .child(
                div()
                    .w(px(210.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(keys_cell)
                    .when_some(suffix.filter(|_| shown.is_some()), |d, suffix| {
                        d.child(div().text_size(px(13.)).text_color(rgb(MUTED)).child(format!("+  {suffix}")))
                    })
                    .when_some(conflict.filter(|_| editing), |d, other| {
                        d.child(div().text_size(px(12.)).text_color(rgb(0xf0bc6b)).child(format!("⚠ This will disable \"{other}\" shortcut")))
                    }),
            )
            .child(div().text_size(px(13.)).text_color(rgb(MUTED)).child(description.to_owned()))
            .on_click(cx.listener(move |settings, _, window, cx| settings.start_recording(&kind_owned, keys.clone(), window, cx)))
            .into_any_element()
    }

    pub(super) fn render_shortcuts(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let settings = self.shortcut_settings(cx);
        let current = settings.current_scheme();
        let name = current.as_ref().map_or_else(|| "Disabled".to_owned(), |(name, _)| name.clone());
        let editable = current.as_ref().is_some_and(|(_, s)| s.editable);
        let menu_open = self.menu == Some(Menu::Scheme);

        // The scheme name (editable for user schemes) and the picker.
        let typing = self.editing.as_ref().filter(|(f, _)| *f == Field::SchemeName).map(|(_, t)| t.clone());
        let name_field = div()
            .id("shortcut-scheme-name")
            .min_w(px(160.))
            .pb(px(4.))
            .border_b_1()
            .border_color(rgb(if typing.is_some() { theme::ACCENT } else { 0x4a4e66 }))
            .flex()
            .items_center()
            .text_size(px(16.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(theme::TEXT))
            .child(text_input::text(("settings", Field::SchemeName), typing.as_deref().unwrap_or(&name), "", typing.is_some(), Mode::Line, cx));
        let name_field = if editable {
            let name = name.clone();
            text_input::field(name_field.cursor_text(), ("settings", Field::SchemeName), Mode::Line, move |settings, window, cx| {
                if settings.editing.as_ref().is_none_or(|(field, _)| *field != Field::SchemeName) {
                    settings.submit_recording(cx);
                    settings.commit_edit(cx);
                    settings.editing = Some((Field::SchemeName, name.clone()));
                }
                window.focus(&settings.focus);
            }, cx)
        } else {
            name_field
        };
        let arrow = div()
            .id("shortcut-scheme-menu")
            .ml(px(10.))
            .size(px(28.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .hover(|s| s.bg(rgb(theme::SURFACE)))
            .child(super::settings_terminal::arrow(menu_open))
            .on_click(cx.listener(|settings, _, _, cx| {
                settings.commit_edit(cx);
                settings.submit_recording(cx);
                settings.menu = if settings.menu == Some(Menu::Scheme) { None } else { Some(Menu::Scheme) };
                cx.notify();
            }));
        let mut header = vec![div().flex().items_center().child(name_field).child(arrow).into_any_element()];

        if menu_open {
            let choices = std::iter::once("Disabled".to_owned()).chain(settings.names());
            let items = choices.map(|choice| {
                let selected = choice == name;
                let removable = settings.is_editable(&choice);
                let pick = choice.clone();
                let remove = choice.clone();
                div()
                    .id(SharedString::from(format!("shortcut-scheme-{choice}")))
                    .px(px(10.))
                    .py(px(6.))
                    .rounded(px(10.))
                    .flex()
                    .justify_between()
                    .items_center()
                    .text_size(px(14.))
                    .text_color(rgb(theme::TEXT))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgba(0xffffff14)))
                    .child(choice.clone())
                    .child(
                        div()
                            .flex()
                            .gap(px(10.))
                            .when(selected, |d| d.child(div().text_color(rgb(MUTED)).child("✓")))
                            .when(removable, |d| {
                                d.child(
                                    div()
                                        .id(SharedString::from(format!("shortcut-scheme-remove-{choice}")))
                                        .text_color(rgb(0xec706f))
                                        .child("✕")
                                        .on_click(cx.listener(move |settings, _, _, cx| {
                                            cx.stop_propagation();
                                            settings.menu = None;
                                            settings.remove_scheme = Some(remove.clone());
                                            cx.notify();
                                        })),
                                )
                            }),
                    )
                    .on_click(cx.listener(move |settings, _, _, cx| {
                        settings.menu = None;
                        let pick = pick.clone();
                        settings.change_shortcuts(cx, move |s| s.select((pick != "Disabled").then_some(pick.as_str())));
                    }))
            });
            header.push(div().mt(px(6.)).w(px(320.)).p(px(5.)).rounded(px(15.)).border_1().border_color(rgba(0xffffff1a)).bg(rgb(0x2f3348)).children(items).into_any_element());
        }

        if let Some(scheme) = self.remove_scheme.clone() {
            header.push(
                div()
                    .mt(px(10.))
                    .p(px(15.))
                    .max_w(px(520.))
                    .rounded(px(10.))
                    .bg(rgb(0x2f3348))
                    .text_size(px(14.))
                    .text_color(rgb(theme::TEXT))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Remove shortcuts scheme"))
                    .child(div().mt(px(8.)).text_color(rgb(MUTED)).child("Are you sure you want to delete shortcuts scheme?"))
                    .child(
                        div()
                            .mt(px(14.))
                            .flex()
                            .justify_end()
                            .gap(px(10.))
                            .child(action_button("shortcut-remove-cancel", "Cancel", false, false).on_click(cx.listener(|settings, _, _, cx| {
                                settings.remove_scheme = None;
                                cx.notify();
                            })))
                            .child(action_button("shortcut-remove-confirm", "Remove", false, false).bg(rgb(0xec706f)).border_color(rgb(0xec706f)).on_click(cx.listener(
                                move |settings, _, _, cx| {
                                    settings.remove_scheme = None;
                                    let scheme = scheme.clone();
                                    settings.change_shortcuts(cx, move |s| s.delete(&scheme));
                                },
                            ))),
                    )
                    .into_any_element(),
            );
        }

        let mut out = vec![div().px(px(20.)).pt(px(30.)).children(header).into_any_element()];
        let Some((_, scheme)) = current else { return out };

        out.push(
            div()
                .mt(px(15.))
                .px(px(20.))
                .h(px(36.))
                .flex()
                .items_center()
                .text_size(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(theme::TEXT))
                .child(div().w(px(210.)).child("Shortcut"))
                .child("Action")
                .into_any_element(),
        );
        out.push(self.shortcut_row(TAB_MOD, Some(scheme.tab_mod.clone()), Some("[1…9]"), "Switch to specific Tab [1…9]", None, cx));
        out.push(self.shortcut_row(PANE_MOD, Some(scheme.pane_mod.clone()), Some("arrows"), "Move focus between Split View panes", None, cx));
        let recording_new = self.recording.as_ref().and_then(|r| r.new.clone());
        for (kind, action) in ACTIONS {
            let keys = scheme.shortcut(kind).map(str::to_owned);
            let conflict = recording_new.as_deref().and_then(|new| settings.conflict(kind, new));
            out.push(self.shortcut_row(kind, keys, None, action, conflict, cx));
        }
        out.push(div().h(px(30.)).into_any_element());
        out
    }
}
