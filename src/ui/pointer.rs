use gpui::{prelude::*, rgb, rgba, Div, Fill, Stateful};
use termian_native::{terminal::blend, theme};

pub(crate) trait PointerFeedback {
    fn pointer_feedback(self, hover: impl Into<Fill>, pressed: impl Into<Fill>) -> Self;
    fn surface_feedback(self, background: u32) -> Self;
    fn icon_feedback(self) -> Self;
    fn menu_feedback(self, destructive: bool) -> Self;
}

impl PointerFeedback for Stateful<Div> {
    fn pointer_feedback(self, hover: impl Into<Fill>, pressed: impl Into<Fill>) -> Self {
        super::keyboard::target(self).cursor_pointer()
            .hover(|s| s.bg(hover))
            .active(|s| s.bg(pressed))
    }

    fn surface_feedback(self, background: u32) -> Self {
        self.pointer_feedback(
            rgb(blend(0xffffff, background, 24)),
            rgb(blend(0xffffff, background, 43)),
        )
    }

    fn icon_feedback(self) -> Self {
        self.pointer_feedback(rgba(0xffffff1e), rgba(0xffffff36))
    }

    fn menu_feedback(self, destructive: bool) -> Self {
        if destructive {
            self.pointer_feedback(rgba(0xf25e6129), rgba(0xf25e6147))
        } else {
            self.pointer_feedback(rgb(theme::SURFACE_ACTIVE), rgb(theme::INPUT))
        }
    }
}
