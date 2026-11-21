use gpui::{
    anchored, div, px, rgb, svg, prelude::*, AnyElement, Context, Corner, Div, FocusHandle,
    FontWeight, KeyDownEvent, MouseButton, Pixels, Point, SharedString, Window,
};
use termian_native::{language::text as tr, theme, updater::State, vault_set::Destination};

use super::{
    assets::icon,
    dialog::{button, Key, Kind},
    host_editor::menu_panel,
    keyboard, motion,
    pointer::PointerFeedback,
    sftp::Side,
    text_input::{self, Mode},
};
use crate::Shell;

pub struct VaultPicker {
    at: Point<Pixels>,
    opener: Option<FocusHandle>,
    creating: bool,
    name: String,
    error: Option<String>,
    focus: FocusHandle,
    input: FocusHandle,
}

fn vault_icon(name: &str, size: f32, color: u32) -> impl IntoElement {
    svg().path(icon(name)).size(px(size)).flex_none().text_color(rgb(color))
}

impl Shell {
    pub(super) fn vault_picker_button(&self, cx: &mut Context<Self>) -> Div {
        let anchor = std::rc::Rc::new(std::cell::Cell::new(None));
        let painted = anchor.clone();
        let open = self.vault_picker.is_some();
        div()
            .flex_none()
            .mb(px(6.))
            .on_children_prepainted(move |children, _, _| {
                if let Some(bounds) = children.first() {
                    painted.set(Some(Point { x: bounds.left(), y: bounds.bottom() + px(6.) }));
                }
            })
            .child(keyboard::target(div().id("select-vault")
                .h(px(40.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .rounded_md()
                .bg(rgb(if open { theme::SURFACE_ACTIVE } else { theme::SURFACE }))
                .surface_feedback(theme::SURFACE_ACTIVE)
                .child(div().size(px(24.)).flex_none().flex().items_center().justify_center()
                    .rounded_sm().bg(rgb(theme::INPUT)).child(vault_icon("vaults", 13., theme::TEXT)))
                .child(div().flex_1().min_w_0().truncate().text_sm().font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(theme::TEXT)).child(self.vaults.selected_name().to_owned()))
                .child(vault_icon("caret_down", 12., theme::TEXT_MUTED))
                .on_click(cx.listener(move |shell, _, window, cx| {
                    let Some(at) = anchor.get() else { return };
                    let opener = window.focused(cx);
                    let focus = cx.focus_handle();
                    window.focus(&focus);
                    shell.vault_picker = Some(VaultPicker {
                        at, opener, creating: false, name: String::new(), error: None,
                        focus, input: cx.focus_handle().tab_stop(true),
                    });
                    cx.notify();
                }))))
    }

    fn close_vault_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = self.vault_picker.take() else { return };
        text_input::reset(cx);
        window.focus(picker.opener.as_ref().unwrap_or(&self.root_focus));
        cx.notify();
    }

    fn show_vault_create(&mut self, creating: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.vault_picker else { return };
        picker.creating = creating;
        picker.error = None;
        if !creating {
            picker.name.clear();
        }
        text_input::reset(cx);
        window.focus(if creating { &picker.input } else { &picker.focus });
        cx.notify();
    }

    fn create_vault(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(picker) = self.vault_picker.as_ref().filter(|p| p.creating && !p.name.trim().is_empty()) {
            let name = picker.name.clone();
            self.change_vault(Destination::New(&name), window, cx);
        }
    }

    fn vault_picker_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.vault_picker else { return };
        if keyboard::tab(event, window, cx, Some(&picker.focus)) {
            return;
        }
        if event.keystroke.key == "escape" {
            if picker.creating {
                self.show_vault_create(false, window, cx);
            } else {
                self.close_vault_picker(window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if !picker.creating {
            match event.keystroke.key.as_str() {
                "up" => keyboard::step(window, cx, Some(&picker.focus), true),
                "down" => keyboard::step(window, cx, Some(&picker.focus), false),
                _ => {}
            }
        } else if picker.input.is_focused(window) {
            match text_input::edit("vault-name", Some(&mut picker.name), &event.keystroke, Mode::Line, cx) {
                Key::Enter => self.create_vault(window, cx),
                Key::Edited => {
                    picker.error = None;
                    cx.notify();
                }
                _ => return,
            }
            cx.stop_propagation();
        }
    }

    fn change_vault(&mut self, dest: Destination<'_>, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(dest, Destination::Existing(id) if id == self.vaults.selected()) {
            self.close_vault_picker(window, cx);
            return;
        }
        let cloud_busy = self.log_sync_busy();
        let Some(picker) = &mut self.vault_picker else { return };
        if self.host_editor.is_some() || self.key_editor.is_some() || self.identity_editor.is_some()
            || self.group_editor.is_some() || self.rules.editor.is_some() || self.snippet_editor.is_some()
            || self.package_editor.is_some() || self.key_export.is_some()
        {
            picker.error = Some(tr("Finish editing or close the editor before switching Vaults.").into());
            cx.notify();
            return;
        }
        if !self.workspaces.is_empty() || self.rules.is_running()
            || [Side::Left, Side::Right].into_iter().any(|side| self.sftp.remote(side).is_some())
            || !self.sftp.actions.edits.is_empty() || !self.sftp.actions.resolving.is_empty()
        {
            picker.error = Some(tr("Close sessions and SFTP, finish file edits, and stop port forwarding before switching Vaults.").into());
            cx.notify();
            return;
        }
        if cloud_busy || self.log_read_pending || matches!(self.update.status.state, State::Checking | State::Downloading) {
            picker.error = Some(tr("Wait for the current operation to finish before switching Vaults.").into());
            cx.notify();
            return;
        }
        let mut vaults = self.vaults.clone();
        let (store, vault) = match vaults.select(&self.vault, dest) {
            Ok(loaded) => loaded,
            Err(error) => {
                picker.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.close_vault_picker(window, cx);
        let settings = self.settings_window.take();
        let update = std::mem::take(&mut self.update);
        window.defer(cx, move |window, cx| {
            if let Some(settings) = settings {
                let _ = settings.update(cx, |_, window, _| window.remove_window());
            }
            let next = window.replace_root(cx, |_, cx| {
                let mut shell = Shell::new(vaults, store, vault, cx);
                shell.update = update;
                shell
            });
            crate::init_main(&next, window, cx);
        });
    }

    pub(crate) fn render_vault_picker(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let picker = self.vault_picker.as_ref()?;
        let viewport = window.viewport_size();
        let width = (viewport.width - px(16.)).clamp(px(0.), px(304.));
        let height = (viewport.height - px(16.)).max(px(0.));
        let mut panel = menu_panel("vault-picker-panel")
            .w(width)
            .max_h(height)
            .min_h_0()
            .overflow_y_scroll()
            .p_2()
            .rounded(px(12.))
            .flex()
            .flex_col()
            .gap_1()
            .text_color(rgb(theme::TEXT))
            .tab_group()
            .track_focus(&picker.focus)
            .on_key_down(cx.listener(Self::vault_picker_key));
        if picker.creating {
            let input = picker.input.clone();
            let focused = input.is_focused(window);
            let name = div().id("vault-name").track_focus(&input).h(px(38.)).flex_none().px_3()
                .flex().items_center().rounded_md().bg(rgb(theme::TITLE_BAR))
                .border_1().border_color(rgb(if focused { theme::ACCENT } else { theme::SURFACE_ACTIVE }))
                .child(text_input::text("vault-name", &picker.name, tr("Vault name"), focused, Mode::Line, cx));
            let name = text_input::field(name, "vault-name", Mode::Line, move |_, window, _| window.focus(&input), cx);
            let disabled = picker.name.trim().is_empty();
            panel = panel
                .child(div().px_2().py_1().text_sm().font_weight(FontWeight::SEMIBOLD).child(tr("New Vault")))
                .child(name)
                .children(picker.error.as_ref().map(|error| vault_error(error)))
                .child(div().pt_2().flex().flex_wrap().gap_2().justify_end().flex_none()
                    .child(keyboard::target(button("cancel-vault", tr("Cancel"), Kind::Text, false))
                        .on_click(cx.listener(|shell, _, window, cx| shell.show_vault_create(false, window, cx))))
                    .child(button("save-vault", tr("Create Vault"), Kind::Accent, disabled)
                        .when(!disabled, |d| keyboard::target(d).on_click(cx.listener(|shell, _, window, cx| shell.create_vault(window, cx))))));
        } else {
            let entries = self.vaults.entries().iter().map(|entry| {
                let id = entry.id;
                let active = id == self.vaults.selected();
                keyboard::target(div().id(SharedString::from(format!("vault-{id}"))).h(px(36.)).flex_none()
                    .px_2().flex().items_center().gap_2().rounded_md()
                    .when(active, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                    .menu_feedback(false)
                    .child(vault_icon("vaults", 14., theme::TEXT_MUTED))
                    .child(div().flex_1().min_w_0().truncate().text_sm().child(entry.name().to_owned()))
                    .when(active, |d| d.child(vault_icon("check", 14., theme::ACCENT)))
                    .on_click(cx.listener(move |shell, _, window, cx| shell.change_vault(Destination::Existing(id), window, cx))))
            });
            let list_height = (height - px(94.)).clamp(px(36.), px(252.));
            panel = panel
                .child(div().px_2().py_1().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(tr("Vaults")))
                .child(div().id("vault-entries").max_h(list_height).min_h_0()
                    .overflow_y_scroll().flex().flex_col().gap_1().children(entries))
                .children(picker.error.as_ref().map(|error| vault_error(error)))
                .child(div().h(px(1.)).flex_none().my_1().bg(rgb(theme::SURFACE_ACTIVE)))
                .child(keyboard::target(div().id("new-vault").h(px(36.)).flex_none().px_2()
                    .flex().items_center().gap_2().rounded_md().menu_feedback(false)
                    .child(vault_icon("plus", 14., theme::ACCENT))
                    .child(div().min_w_0().truncate().text_sm().child(tr("New Vault")))
                    .on_click(cx.listener(|shell, _, window, cx| shell.show_vault_create(true, window, cx)))));
        }
        Some(div().id("vault-picker").absolute().inset_0().occlude()
            .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, window, cx| shell.close_vault_picker(window, cx)))
            .child(anchored().position(picker.at).anchor(Corner::TopLeft).snap_to_window_with_margin(px(8.))
                .child(motion::popover(panel, "vault-picker-enter")))
            .into_any_element())
    }
}

fn vault_error(message: &str) -> Div {
    div().px_2().py_1().flex_none().text_xs().text_color(rgb(theme::ERROR)).child(message.to_owned())
}
