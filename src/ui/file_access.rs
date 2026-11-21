//! The macOS «Доступ к файлам» window (legacy `file-access-guide.js`): the
//! folders the SFTP Local panel may need and their access, refreshed every
//! two seconds while open, with a link to System Settings for a denied one.

use termian_native::language::text as tr;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use chrono::Local;
use gpui::{
    div, prelude::*, px, rgb, size, App, Bounds, Context, FocusHandle, FontWeight, KeyDownEvent, Window, WindowBounds,
    WindowHandle, WindowOptions, TitlebarOptions,
};
use termian_native::file_access::{self, Folder, Status};
use termian_native::theme;

use super::settings::{action_button, MUTED};

const REFRESH: Duration = Duration::from_secs(2);
const OK: u32 = 0x3dcc94;
const DENIED: u32 = 0xec706f;

pub struct FileAccessWindow {
    folders: Vec<Folder>,
    updated: String,
    focus: FocusHandle,
}

impl FileAccessWindow {
    fn refresh(&mut self) {
        self.folders = file_access::folders(&dirs::home_dir().unwrap_or_default());
        self.updated = Local::now().format("%H:%M:%S").to_string();
    }
}

/// Brings the window forward, or opens it; only one is open at a time.
pub fn open(slot: &Rc<RefCell<Option<WindowHandle<FileAccessWindow>>>>, cx: &mut App) {
    if let Some(handle) = *slot.borrow() {
        if handle.update(cx, |_, window, _| window.activate_window()).is_ok() {
            return;
        }
    }
    let bounds = Bounds::centered(None, size(px(560.), px(520.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions { title: Some(tr("Доступ к файлам").into()), appears_transparent: cfg!(target_os = "macos"), ..Default::default() }),
        ..Default::default()
    };
    let opened = cx.open_window(options, |window, cx| {
        #[cfg(windows)]
        super::assets::set_window_icon(window);
        cx.new(|cx| {
            let focus = cx.focus_handle();
            window.focus(&focus);
            // The status is probed again while the window lives, so a change
            // in System Settings shows up without reopening it.
            cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(REFRESH).await;
                if this.update(cx, |this: &mut FileAccessWindow, cx| { this.refresh(); cx.notify(); }).is_err() {
                    break;
                }
            })
            .detach();
            let mut this = FileAccessWindow { folders: Vec::new(), updated: String::new(), focus };
            this.refresh();
            this
        })
    });
    *slot.borrow_mut() = opened.ok();
}

impl Render for FileAccessWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.folders.iter().enumerate().map(|(index, folder)| {
            let tone = match folder.status {
                Status::Granted => OK,
                Status::Denied => DENIED,
                _ => MUTED,
            };
            let url = folder.settings_url;
            let side = if folder.status == Status::Denied {
                action_button(format!("file-access-open-{index}"), tr("Открыть настройки"), false, false)
                    .on_click(move |_, _, cx| cx.open_url(url))
                    .into_any_element()
            } else {
                div().text_size(px(12.)).text_color(rgb(tone)).child(folder.status.label()).into_any_element()
            };
            let name = if folder.required { folder.title.to_owned() } else { termian_native::ui_format!("{} (optional)", "{} (необязательно)", folder.title) };
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(8.))
                .border_1()
                .border_color(rgb(0x454950))
                .bg(rgb(0x202328))
                .child(div().size(px(8.)).flex_none().rounded_full().bg(rgb(tone)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(name))
                        .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(folder.path.clone())),
                )
                .child(side)
        });
        div()
            .id("file-access")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, _| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                }
            }))
            .size_full()
            .overflow_y_scroll()
            .bg(rgb(0x272b30))
            .text_color(rgb(theme::TEXT))
            .px(px(24.))
            .pt(px(if cfg!(target_os = "macos") { 40. } else { 22. }))
            .pb(px(22.))
            .child(div().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(tr("Доступ к файлам")))
            .child(div().mt(px(6.)).mb(px(14.)).text_size(px(12.)).line_height(px(18.)).text_color(rgb(MUTED)).child(
                "Панель Local в SFTP работает с файлами этого Mac. macOS один раз спрашивает разрешение на Рабочий стол, Документы и Загрузки. Если доступ был отклонён, включите termian в Системные настройки → Конфиденциальность и безопасность → Файлы и папки. Статус ниже обновляется автоматически.",
            ))
            .child(div().flex().flex_col().gap(px(8.)).children(rows))
            .child(
                div()
                    .mt(px(14.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(11.)).text_color(rgb(MUTED)).child(termian_native::ui_format!("Updated: {}", "Обновлено: {}", self.updated)))
                    .child(action_button("file-access-close", tr("Закрыть"), false, false).on_click(|_, window, _| window.remove_window())),
            )
    }
}
