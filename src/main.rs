use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Context, SharedString, Stateful,
    Div, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use termian_native::tabs::{Tab, TabKind, Tabs};
use termian_native::theme;

/// Room left of the tab strip for the macOS traffic-light buttons.
#[cfg(target_os = "macos")]
const WINDOW_CONTROLS_INSET: f32 = 80.;
#[cfg(not(target_os = "macos"))]
const WINDOW_CONTROLS_INSET: f32 = 12.;

struct Shell {
    tabs: Tabs,
}

impl Shell {
    fn render_tab(&self, tab: &Tab, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = tab.id;
        let (text, background) = if id == self.tabs.active().id {
            (theme::TEXT, theme::SURFACE_ACTIVE)
        } else {
            (theme::TEXT_MUTED, theme::SURFACE)
        };
        let mut element = div()
            .id(SharedString::from(format!("tab-{id:?}")))
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .rounded_md()
            .text_sm()
            .text_color(rgb(text))
            .bg(rgb(background))
            .on_click(cx.listener(move |shell, _, _, cx| {
                shell.tabs.activate(id);
                cx.notify();
            }));
        if tab.closable() {
            element = element.child(
                div()
                    .id(SharedString::from(format!("close-{id:?}")))
                    .child("×")
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        cx.stop_propagation();
                        shell.tabs.close(id);
                        cx.notify();
                    })),
            );
        }
        element.child(SharedString::from(tab.title().to_owned()))
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs: Vec<_> = self.tabs.all().iter().map(|tab| self.render_tab(tab, cx)).collect();
        // Sync / push / pull indicators; glyphs stand in until the icon set is ported.
        let toolbar = ["⟳", "⇪", "⇩"].map(|glyph| div().px_2().text_color(rgb(theme::TEXT_MUTED)).child(glyph));

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::BACKGROUND))
            .child(
                div()
                    .h(px(48.))
                    .pl(px(WINDOW_CONTROLS_INSET))
                    .pr_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .bg(rgb(theme::TITLE_BAR))
                    .children(tabs)
                    .child(
                        div()
                            .id("new-tab")
                            .px_2()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child("+")
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.tabs.open(TabKind::Launcher);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .children(toolbar),
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
                    appears_transparent: cfg!(target_os = "macos"),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Shell { tabs: Tabs::default() }),
        )
        .expect("failed to open the main window");
        cx.activate(true);
    });
}
