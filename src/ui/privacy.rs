use gpui::{
    div, prelude::*, px, rgb, svg, Animation, AnimationExt, AnyElement, Context, FocusHandle,
    SharedString, Subscription, Transformation, Window,
};
use std::time::{Duration, Instant};
use termian_native::privacy_password::{PrivacyPassword, MAX_BYTES};
use termian_native::theme;
use zeroize::{Zeroize, Zeroizing};

use super::pointer::PointerFeedback;
use super::text_input::{self, Mode};
use super::{
    assets::icon,
    cloud::Tooltip,
    dialog::{Key, Kind},
};
use crate::Shell;

const TRANSITION: Duration = Duration::from_millis(180);

#[derive(Default)]
pub struct Privacy {
    locked: bool,
    setting_up: bool,
    opening: bool,
    os_auth: bool,
    auth_pending: bool,
    password_pending: bool,
    focus: Option<FocusHandle>,
    repeat_focus: Option<FocusHandle>,
    observer: Option<Subscription>,
    keyboard: Option<Subscription>,
    input: Zeroizing<String>,
    repeat: Zeroizing<String>,
    hint: &'static str,
    failures: u8,
    retry_at: Option<Instant>,
    automatic_after: Option<Instant>,
    version: u64,
    recent_unlock: bool,
}

impl Privacy {
    fn busy(&self) -> bool {
        self.opening || self.auth_pending || self.password_pending
    }
}

#[cfg(target_os = "macos")]
extern "C" {
    fn termian_privacy_available(biometric: i32) -> i32;
    fn termian_privacy_verify(biometric: i32) -> i32;
}

fn available(biometric: bool) -> bool {
    if std::env::var_os("TERMIAN_DEMO").is_some() {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        unsafe { termian_privacy_available(i32::from(biometric)) != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = biometric;
        false
    }
}

fn authenticate(biometric: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { termian_privacy_verify(i32::from(biometric)) != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = biometric;
        false
    }
}

impl Shell {
    pub(crate) fn privacy_locked(&self) -> bool {
        self.privacy.locked || self.privacy.setting_up
    }

    pub(crate) fn install_privacy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.privacy.focus = Some(cx.focus_handle().tab_stop(true));
        self.privacy.repeat_focus = Some(cx.focus_handle().tab_stop(true));
        let main = window.window_handle();
        let shell = cx.weak_entity();
        self.privacy.keyboard = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle() == main {
                let _ = shell.update(cx, |shell, cx| {
                    shell.privacy_key(&event.keystroke, window, cx)
                });
            }
        }));
        self.privacy.observer = Some(cx.observe_window_activation(window, |shell, window, cx| {
            if window.is_window_active() && shell.privacy.locked {
                shell.privacy_focus(window);
                if shell
                    .privacy
                    .automatic_after
                    .is_none_or(|after| Instant::now() >= after)
                {
                    shell.privacy_authenticate(true, window, cx);
                }
            }
        }));
    }

    fn cloud_password(&self) -> Option<&str> {
        self.vault
            .cloud
            .as_ref()
            .map(|config| config.passphrase.as_str())
            .filter(|key| !key.is_empty())
    }

    fn privacy_focus(&self, window: &mut Window) {
        if let Some(focus) = &self.privacy.focus {
            window.focus(focus);
        }
    }

    fn privacy_key(
        &mut self,
        stroke: &gpui::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.privacy_locked() {
            return;
        }
        cx.stop_propagation();
        if self.privacy.busy() {
            return;
        }
        let mods = stroke.modifiers;
        if stroke.key == "tab" && !mods.alt && !mods.control && !mods.platform {
            super::keyboard::step(window, cx, None, mods.shift);
            return;
        }
        if stroke.key == "escape" && self.privacy.setting_up {
            self.cancel_privacy_setup(window, cx);
            return;
        }
        let first = self
            .privacy
            .focus
            .as_ref()
            .is_some_and(|focus| focus.is_focused(window));
        let repeat = self.privacy.setting_up
            && self
                .privacy
                .repeat_focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window));
        if !(first || repeat)
            || !(self.privacy.setting_up
                || self.vault.privacy_password.is_some()
                || self.cloud_password().is_some())
        {
            return;
        }
        let local_password = self.privacy.setting_up || self.vault.privacy_password.is_some();
        let (field, value) = if repeat {
            ("privacy-repeat", &mut self.privacy.repeat)
        } else {
            ("privacy-password", &mut self.privacy.input)
        };
        match text_input::edit(field, Some(value), stroke, Mode::Secret, cx) {
            Key::Enter if self.privacy.setting_up && first => {
                if let Some(focus) = &self.privacy.repeat_focus {
                    window.focus(focus);
                }
            }
            Key::Enter => self.submit_privacy_password(window, cx),
            Key::Edited => {
                if local_password && value.len() > MAX_BYTES {
                    value.zeroize();
                    self.privacy.hint = "The password is too long.";
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn cancel_privacy_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.privacy.setting_up || self.privacy.busy() {
            return;
        }
        self.privacy.setting_up = false;
        self.privacy.input.zeroize();
        self.privacy.repeat.zeroize();
        self.privacy.hint = "";
        text_input::reset(cx);
        window.focus(&self.root_focus);
        cx.notify();
    }

    fn lock_privacy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.privacy_locked() {
            return;
        }
        self.privacy.os_auth = available(false);
        if !self.privacy.os_auth
            && self.cloud_password().is_none()
            && self.vault.privacy_password.is_none()
        {
            self.privacy.setting_up = true;
            self.privacy.version += 1;
            self.privacy.hint = "";
            text_input::reset(cx);
            self.privacy_focus(window);
            cx.notify();
            return;
        }
        self.activate_privacy_lock(window, cx);
    }

    fn activate_privacy_lock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.privacy.setting_up = false;
        self.privacy.locked = true;
        self.privacy.recent_unlock = false;
        self.privacy.version += 1;
        self.privacy.hint = "";
        self.privacy.input.zeroize();
        self.privacy.repeat.zeroize();
        self.app_menu = None;
        text_input::reset(cx);
        self.privacy_focus(window);
        let main = window.window_handle();
        cx.defer(move |cx| {
            for handle in cx.windows().into_iter().filter(|handle| *handle != main) {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
        });
        cx.notify();
    }

    fn open_privacy(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.privacy.locked || self.privacy.opening {
            return;
        }
        self.privacy.opening = true;
        self.privacy.hint = "";
        self.privacy.input.zeroize();
        self.privacy.failures = 0;
        self.privacy.retry_at = None;
        let timer = cx.background_executor().timer(TRANSITION);
        cx.spawn_in(window, async move |this, cx| {
            timer.await;
            let _ = cx.update(|window, cx| {
                let _ = this.update(cx, |shell, cx| {
                    shell.privacy.opening = false;
                    shell.privacy.locked = false;
                    shell.privacy.recent_unlock = true;
                    window.focus(&shell.root_focus);
                    cx.notify();
                });
            });
        })
        .detach();
        cx.notify();
    }

    fn privacy_authenticate(&mut self, biometric: bool, window: &Window, cx: &mut Context<Self>) {
        if !self.privacy.locked
            || self.privacy.opening
            || self.privacy.auth_pending
            || self.privacy.password_pending
            || !available(biometric)
        {
            return;
        }
        self.privacy.auth_pending = true;
        self.privacy.input.zeroize();
        self.privacy.hint = "";
        let task = cx.background_spawn(async move { authenticate(biometric) });
        cx.spawn_in(window, async move |this, cx| {
            let verified = task.await;
            let _ = cx.update(|window, cx| {
                let _ = this.update(cx, |shell, cx| {
                    shell.privacy.auth_pending = false;
                    shell.privacy.automatic_after = Some(Instant::now() + Duration::from_secs(1));
                    if verified {
                        shell.open_privacy(window, cx);
                    } else {
                        shell.privacy_focus(window);
                        shell.privacy.hint = "Authentication did not complete.";
                        cx.notify();
                    }
                });
            });
        })
        .detach();
        cx.notify();
    }

    fn submit_privacy_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.privacy.busy() {
            return;
        }
        if self.privacy.setting_up {
            self.save_privacy_password(window, cx);
            return;
        }
        if !self.privacy.locked {
            return;
        }
        if self
            .privacy
            .retry_at
            .is_some_and(|after| Instant::now() < after)
        {
            self.privacy.input.zeroize();
            self.privacy.hint = "Too many attempts. Try again in a few seconds.";
            cx.notify();
            return;
        }
        let password = std::mem::take(&mut self.privacy.input);
        let Some(verifier) = self.vault.privacy_password.clone() else {
            let verified = self
                .cloud_password()
                .is_some_and(|cloud| cloud == password.as_str());
            self.finish_privacy_password(verified, window, cx);
            return;
        };
        self.privacy.password_pending = true;
        self.privacy.hint = "";
        let task = cx.background_spawn(async move { verifier.verify(&password) });
        cx.spawn_in(window, async move |this, cx| {
            let verified = task.await;
            let _ = this.update_in(cx, |shell, window, cx| {
                shell.privacy.password_pending = false;
                shell.finish_privacy_password(verified, window, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn finish_privacy_password(&mut self, verified: bool, window: &Window, cx: &mut Context<Self>) {
        if verified {
            self.open_privacy(window, cx);
            return;
        }
        self.privacy.failures += 1;
        self.privacy.hint = "Incorrect password.";
        if self.privacy.failures >= 5 {
            self.privacy.retry_at = Some(Instant::now() + Duration::from_secs(10));
            self.privacy.failures = 0;
            self.privacy.hint = "Too many attempts. Try again in 10 seconds.";
        }
        cx.notify();
    }

    fn save_privacy_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.privacy.input.is_empty() || self.privacy.input != self.privacy.repeat {
            self.privacy.hint = if self.privacy.input.is_empty() {
                "Enter a password."
            } else {
                "The passwords do not match."
            };
            self.privacy.repeat.zeroize();
            if self.privacy.input.is_empty() {
                self.privacy_focus(window);
            } else if let Some(focus) = &self.privacy.repeat_focus {
                window.focus(focus);
            }
            cx.notify();
            return;
        }
        let password = std::mem::take(&mut self.privacy.input);
        self.privacy.repeat.zeroize();
        self.privacy.password_pending = true;
        self.privacy.hint = "";
        let task = cx.background_spawn(async move { PrivacyPassword::new(&password) });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |shell, window, cx| {
                shell.privacy.password_pending = false;
                if let Some(verifier) = result {
                    let previous = shell.vault.privacy_password.replace(verifier);
                    if shell.store.save(&shell.vault).is_ok() {
                        shell.activate_privacy_lock(window, cx);
                        return;
                    }
                    shell.vault.privacy_password = previous;
                }
                shell.privacy.hint = "Could not save the lock password.";
                shell.privacy_focus(window);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn render_privacy_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let glyph = svg()
            .path(icon("privacy_open"))
            .size(px(18.))
            .text_color(rgb(theme::TEXT));
        let glyph = if self.privacy.recent_unlock {
            glyph
                .with_animation(
                    SharedString::from(format!("privacy-open-{}", self.privacy.version)),
                    Animation::new(TRANSITION),
                    |glyph, t| {
                        glyph.with_transformation(Transformation::rotate(gpui::percentage(
                            1. - 0.1 * (1. - t),
                        )))
                    },
                )
                .into_any_element()
        } else {
            glyph.into_any_element()
        };
        div()
            .mr_3()
            .px_2()
            .child(
                div()
                    .id("privacy-lock")
                    .size(px(30.))
                    .rounded_md()
                    .flex()
                    .items_center()
                    .justify_center()
                    .icon_feedback()
                    .tooltip(|_, cx| cx.new(|_| Tooltip("Lock Termian".into())).into())
                    .child(glyph)
                    .on_click(cx.listener(|shell, _, window, cx| shell.lock_privacy(window, cx))),
            )
            .into_any_element()
    }

    pub(crate) fn render_privacy(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let setup = self.privacy.setting_up;
        let busy = self.privacy.busy();
        let has_local = self.vault.privacy_password.is_some();
        let has_password = has_local || self.cloud_password().is_some();
        let version = self.privacy.version;
        let opening = self.privacy.opening;
        let field = |repeat: bool, placeholder: &'static str, cx: &mut Context<Self>| {
            let (id, value, handle) = if repeat {
                (
                    "privacy-repeat",
                    &self.privacy.repeat,
                    self.privacy.repeat_focus.as_ref(),
                )
            } else {
                (
                    "privacy-password",
                    &self.privacy.input,
                    self.privacy.focus.as_ref(),
                )
            };
            let focused = !busy && handle.is_some_and(|focus| focus.is_focused(window));
            let input = div()
                .id(id)
                .w(px(290.))
                .h(px(38.))
                .px_3()
                .flex()
                .items_center()
                .border_1()
                .rounded_md()
                .border_color(rgb(if focused { theme::ACCENT } else { theme::INPUT }))
                .bg(rgb(theme::SURFACE_ACTIVE))
                .overflow_hidden()
                .when_some(handle, |d, focus| d.track_focus(focus))
                .child(text_input::text(
                    id,
                    value,
                    placeholder,
                    focused,
                    Mode::Secret,
                    cx,
                ));
            if busy {
                input
            } else {
                text_input::field(
                    input,
                    id,
                    Mode::Secret,
                    move |shell, window, _| {
                        if repeat {
                            if let Some(focus) = &shell.privacy.repeat_focus {
                                window.focus(focus);
                            }
                        } else {
                            shell.privacy_focus(window);
                        }
                    },
                    cx,
                )
            }
        };
        let mut overlay = div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgb(theme::BACKGROUND))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                svg()
                    .path(icon("privacy_lock"))
                    .size(px(38.))
                    .text_color(rgb(theme::TEXT)),
            )
            .child(div().text_xl().child(if setup {
                "Set a lock password"
            } else {
                "Termian is locked"
            }));
        if setup {
            overlay = overlay
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("This password is for this device."),
                )
                .child(field(false, "Lock password", cx))
                .child(field(true, "Repeat password", cx));
        } else if has_password {
            overlay = overlay.child(field(
                false,
                if has_local {
                    "Lock password"
                } else {
                    "Cloud encryption password"
                },
                cx,
            ));
        } else if let Some(focus) = &self.privacy.focus {
            overlay = overlay.track_focus(focus);
        }
        let mut actions = div().flex().items_center().gap_2();
        if setup || has_password {
            actions =
                actions.child(
                    super::dialog::button(
                        "privacy-submit",
                        if busy {
                            "Please wait…"
                        } else if setup {
                            "Lock Termian"
                        } else {
                            "Unlock"
                        },
                        Kind::Accent,
                        busy,
                    )
                    .when(!busy, |d| {
                        d.on_click(cx.listener(|shell, _, window, cx| {
                            shell.submit_privacy_password(window, cx)
                        }))
                    }),
                );
        }
        if setup {
            actions = actions.child(
                super::dialog::button("privacy-cancel", "Cancel", Kind::Regular, busy).when(
                    !busy,
                    |d| {
                        d.on_click(cx.listener(|shell, _, window, cx| {
                            shell.cancel_privacy_setup(window, cx)
                        }))
                    },
                ),
            );
        } else if self.privacy.os_auth {
            actions = actions.child(
                super::dialog::button(
                    "privacy-touch-id",
                    if self.privacy.auth_pending {
                        "Waiting for macOS…"
                    } else {
                        "Use Touch ID or Mac password"
                    },
                    Kind::Regular,
                    busy,
                )
                .when(!busy, |d| {
                    d.on_click(cx.listener(|shell, _, window, cx| {
                        shell.privacy_authenticate(false, window, cx)
                    }))
                }),
            );
        }
        overlay = overlay
            .child(actions)
            .child(div().h(px(24.)).text_sm().child(self.privacy.hint));
        div()
            .relative()
            .size_full()
            .tab_group()
            .bg(rgb(theme::BACKGROUND))
            .text_color(rgb(theme::TEXT))
            .child(overlay.with_animation(
                SharedString::from(format!("privacy-overlay-{version}-{opening}")),
                Animation::new(TRANSITION),
                move |d, t| d.opacity(if opening { 1. - t } else { t }),
            ))
            .into_any_element()
    }
}
