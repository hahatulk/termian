use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui::{
    div, prelude::*, px, rgb, rgba, Animation, AnimationExt, AnyElement, ElementId, Window,
};

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn system_animations() -> bool {
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let workspace: *mut objc::runtime::Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        let reduce: objc::runtime::BOOL =
            msg_send![workspace, accessibilityDisplayShouldReduceMotion];
        reduce == objc::runtime::NO
    }
}

#[cfg(target_os = "windows")]
fn system_animations() -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, value: *mut i32, flags: u32) -> i32;
    }
    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    let mut enabled = 1;
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut enabled, 0) };
    ok == 0 || enabled != 0
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn system_animations() -> bool {
    true
}

pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(system_animations)
}

fn ease(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}

fn enter<E: IntoElement + 'static>(
    element: E,
    id: impl Into<ElementId>,
    ms: u64,
    style: impl Fn(E, f32) -> E + 'static,
) -> AnyElement {
    if enabled() {
        element
            .with_animation(
                id,
                Animation::new(Duration::from_millis(ms)).with_easing(ease),
                style,
            )
            .into_any_element()
    } else {
        style(element, 1.).into_any_element()
    }
}

pub fn page(element: impl Styled + IntoElement + 'static, id: impl Into<ElementId>) -> AnyElement {
    enter(element, id, 220, |d, t| d.opacity(0.2 + 0.8 * t))
}

pub fn modal(card: impl Styled + IntoElement + 'static) -> AnyElement {
    enter(card, "modal-enter", 245, |d, t| d.opacity(t))
}

pub fn scrim(color: u32) -> AnyElement {
    enter(
        div().absolute().inset_0().bg(rgba(color)),
        "scrim-enter",
        195,
        |d, t| d.opacity(t),
    )
}

pub fn popover(panel: impl Styled + IntoElement + 'static, id: impl Into<ElementId>) -> AnyElement {
    enter(panel, id, 155, |d, t| d.opacity(0.25 + 0.75 * t))
}

pub fn slide_in(
    panel: impl Styled + IntoElement + 'static,
    id: impl Into<ElementId>,
    width: f32,
) -> AnyElement {
    enter(panel, id, 260, move |d, t| d.right(px(-width * (1. - t))))
}

pub fn toast(panel: impl Styled + IntoElement + 'static) -> AnyElement {
    enter(panel, "toast-enter", 285, |d, t| d.opacity(t))
}

pub fn sidebar_marker(color: u32) -> AnyElement {
    div()
        .absolute()
        .left_0()
        .top_1()
        .bottom_1()
        .w(px(2.))
        .flex()
        .items_center()
        .child(enter(
            div().rounded_full().bg(rgb(color)).w_full(),
            "selection-enter",
            240,
            |d, t| d.h(gpui::relative(0.2 + 0.8 * t)),
        ))
        .into_any_element()
}

pub struct Tween {
    from: f32,
    target: f32,
    started: Instant,
}

impl Tween {
    pub fn at(value: f32) -> Self {
        Self {
            from: value,
            target: value,
            started: Instant::now(),
        }
    }

    pub fn toward(&mut self, target: f32, ms: u64, window: &Window) -> f32 {
        if !enabled() {
            *self = Self::at(target);
            return target;
        }
        let elapsed = self.started.elapsed().as_secs_f32() / (ms as f32 / 1000.);
        let value = if elapsed >= 1. {
            self.target
        } else {
            self.from + (self.target - self.from) * ease(elapsed)
        };
        if self.target != target {
            self.from = value;
            self.target = target;
            self.started = Instant::now();
        }
        if value != target {
            window.request_animation_frame();
        }
        value
    }
}
