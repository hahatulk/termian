use gpui::{
    div, prelude::*, px, rgb, svg, Animation, AnimationExt, AnyElement, Context, FocusHandle,
    SharedString, Subscription, Transformation, Window,
};
use std::time::{Duration, Instant};
use termian_native::theme;
use zeroize::{Zeroize, Zeroizing};

use super::{
    assets::icon,
    cloud::Tooltip,
    dialog::Key,
};
use super::text_input::{self, Mode};
use crate::Shell;

const TRANSITION: Duration = Duration::from_millis(180);

#[derive(Default)]
pub struct Privacy {
    locked: bool,
    opening: bool,
    auth_pending: bool,
    focus: Option<FocusHandle>,
    observer: Option<Subscription>,
    keyboard: Option<Subscription>,
    input: Zeroizing<String>,
    hint: &'static str,
    failures: u8,
    retry_at: Option<Instant>,
    automatic_after: Option<Instant>,
    version: u64,
    recent_unlock: bool,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn termian_privacy_available(biometric: i32) -> i32;
    fn termian_privacy_verify(biometric: i32) -> i32;
}

fn available(biometric: bool) -> bool {
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
        self.privacy.locked
    }

    pub(crate) fn install_privacy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.privacy.focus = Some(cx.focus_handle());
        let main = window.window_handle();
        let shell = cx.weak_entity();
        self.privacy.keyboard = Some(cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle() != main {
                return;
            }
            let _ = shell.update(cx, |shell, cx| {
                if !shell.privacy.locked {
                    return;
                }
                cx.stop_propagation();
                if shell.privacy.auth_pending
                    || shell.privacy.opening
                    || shell.privacy_password().is_none()
                {
                    return;
                }
                let key = &event.keystroke;
                match text_input::edit("privacy-password", Some(&mut shell.privacy.input), key, Mode::Secret, cx) {
                    Key::Enter => shell.check_privacy_password(window, cx),
                    Key::Edited => cx.notify(),
                    _ => {}
                }
            });
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

    fn privacy_password(&self) -> Option<&str> {
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

    fn lock_privacy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.privacy.locked || (self.privacy_password().is_none() && !available(false)) {
            return;
        }
        self.privacy.locked = true;
        self.privacy.recent_unlock = false;
        self.privacy.version += 1;
        self.privacy.hint = "";
        self.privacy.input.zeroize();
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

    fn check_privacy_password(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.privacy.auth_pending || self.privacy.opening || self.privacy_password().is_none() {
            return;
        }
        if self
            .privacy
            .retry_at
            .is_some_and(|after| Instant::now() < after)
        {
            self.privacy.hint = "Too many attempts. Try again in a few seconds.";
        } else if self.privacy_password() == Some(self.privacy.input.as_str()) {
            self.open_privacy(window, cx);
            return;
        } else {
            self.privacy.failures += 1;
            self.privacy.hint = "Incorrect encryption password.";
            if self.privacy.failures >= 5 {
                self.privacy.retry_at = Some(Instant::now() + Duration::from_secs(10));
                self.privacy.failures = 0;
                self.privacy.hint = "Too many attempts. Try again in 10 seconds.";
            }
        }
        self.privacy.input.zeroize();
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
                    .cursor_pointer()
                    .hover(|d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                    .tooltip(|_, cx| cx.new(|_| Tooltip("Lock Termian".into())).into())
                    .child(glyph)
                    .on_click(cx.listener(|shell, _, window, cx| shell.lock_privacy(window, cx))),
            )
            .into_any_element()
    }

    pub(crate) fn render_privacy(&self, cx: &mut Context<Self>) -> AnyElement {
        let version = self.privacy.version;
        let opening = self.privacy.opening;
        let busy = opening || self.privacy.auth_pending;
        let has_password = self.privacy_password().is_some();
        let tile = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .rounded_md()
                .px_4()
                .py_2()
                .bg(rgb(theme::SURFACE_ACTIVE))
                .text_color(rgb(theme::TEXT))
                .cursor_pointer()
                .child(label)
        };
        let label = if self.privacy.auth_pending {
            "Waiting for macOS…"
        } else {
            "Use Touch ID or Mac password"
        };
        let glyph = svg()
            .path(icon("privacy_lock"))
            .size(px(38.))
            .text_color(rgb(theme::TEXT))
            .with_animation(
                SharedString::from(format!("privacy-icon-{version}-{opening}")),
                Animation::new(TRANSITION),
                move |glyph, t| {
                    glyph.with_transformation(Transformation::rotate(gpui::percentage(
                        if opening {
                            1. - 0.10 * t
                        } else {
                            0.10 * (1. - t)
                        },
                    )))
                },
            );
        let overlay = div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgb(theme::BACKGROUND))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(glyph)
            .child(div().text_xl().child("Termian is locked"))
            .when(has_password, |d| {
                let field = div().id("privacy-password").w(px(290.)).h(px(38.)).px_3().flex().items_center()
                    .border_1().rounded_md().border_color(rgb(theme::ACCENT)).bg(rgb(theme::SURFACE_ACTIVE)).overflow_hidden()
                    .child(text_input::text("privacy-password", &self.privacy.input, "Encryption password", !busy, Mode::Secret, cx));
                d.child(text_input::field(field, "privacy-password", Mode::Secret, |shell, window, _| shell.privacy_focus(window), cx))
                .child(tile("privacy-unlock", "Unlock").on_click(cx.listener(
                    |shell, _, window, cx| {
                        shell.check_privacy_password(window, cx);
                    },
                )))
            })
            .when(cfg!(target_os = "macos"), |d| {
                d.child(tile("privacy-touch-id", label).when(!busy, |button| {
                    button.on_click(cx.listener(|shell, _, window, cx| {
                        shell.privacy_authenticate(available(true), window, cx);
                    }))
                }))
            })
            .child(div().h(px(24.)).text_sm().child(self.privacy.hint))
            .with_animation(
                SharedString::from(format!("privacy-overlay-{version}-{opening}")),
                Animation::new(TRANSITION),
                move |d, t| d.opacity(if opening { 1. - t } else { t }),
            );
        div()
            .relative()
            .size_full()
            .bg(rgb(theme::BACKGROUND))
            .text_color(rgb(theme::TEXT))
            .when_some(self.privacy.focus.as_ref(), |d, focus| d.track_focus(focus))
            .child(overlay)
            .into_any_element()
    }
}
