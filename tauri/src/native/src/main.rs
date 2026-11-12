use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, MouseButton, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use termian_native::tabs::{TabKind, Tabs};
use termian_native::theme;

struct Shell {
    tabs: Tabs,
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.tabs.active().id;
        let strip = self.tabs.all().iter().map(|tab| {
            let id = tab.id;
            let is_active = id == active;
            div()
                .id(SharedString::from(format!("tab-{id}")))
                .px_3()
                .py_1()
                .rounded_md()
                .text_sm()
                .text_color(rgb(if is_active { theme::TEXT } else { theme::TEXT_MUTED }))
                .bg(rgb(if is_active { theme::SURFACE_ACTIVE } else { theme::SURFACE }))
                .child(SharedString::from(tab.title().to_owned()))
                .on_mouse_down(MouseButton::Left, cx.listener(move |shell, _, _, cx| {
                    shell.tabs.activate(id);
                    cx.notify();
                }))
        });

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::BACKGROUND))
            .child(
                div()
                    .h(px(48.))
                    .pl(px(80.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .bg(rgb(theme::TITLE_BAR))
                    .children(strip)
                    .child(
                        div()
                            .id("new-tab")
                            .px_2()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child("+")
                            .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                                shell.tabs.open(TabKind::NewTab);
                                cx.notify();
                            })),
                    ),
            )
            .child(div().flex_1())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1440.), px(860.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Termian".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Shell { tabs: Tabs::default() }),
        )
        .unwrap();
        cx.activate(true);
    });
}
