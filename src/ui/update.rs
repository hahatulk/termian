//! Automatic updates (`termian_native::updater`) and the legacy
//! `update-notice.js`: once an update is downloaded, verified and in place,
//! a notice in the corner offers a restart. The user chooses when, so open SSH
//! sessions are never cut off by an update.

use gpui::{div, prelude::*, px, rgb, Context, Div, FontWeight, Stateful};
use termian_native::updater::{self, Channel, State, Status};

use crate::Shell;

#[derive(Default)]
pub struct UpdateState {
    pub status: Status,
    pub(super) channel_error: Option<String>,
    /// «Позже»: hidden until the next launch.
    dismissed: bool,
    checking: bool,
    restart_error: Option<String>,
}

impl Shell {
    pub(super) fn set_update_channel(&mut self, channel: Channel, cx: &mut Context<Self>) {
        if self.update.checking || self.update.status.state == State::Installed || self.vault.update_channel == channel {
            return;
        }
        let previous = std::mem::replace(&mut self.vault.update_channel, channel);
        match self.store.save(&self.vault) {
            Ok(()) => {
                self.update.channel_error = None;
                if self.update.status.state != State::Disabled {
                    self.update.status = Status::default();
                }
            }
            Err(error) => {
                self.vault.update_channel = previous;
                self.update.channel_error = Some(format!("Не удалось сохранить канал обновлений: {error}"));
            }
        }
        cx.notify();
    }

    /// First check after 20 s, then every 6 hours, until an update is installed.
    pub fn start_updates(&mut self, cx: &mut Context<Self>) {
        if !updater::enabled() {
            self.update.status.state = State::Disabled;
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(updater::FIRST_CHECK).await;
            loop {
                let installed = this.update(cx, |shell, cx| {
                    shell.check_updates(cx);
                    shell.update.status.state == State::Installed
                });
                if !matches!(installed, Ok(false)) {
                    break;
                }
                cx.background_executor().timer(updater::CHECK_INTERVAL).await;
            }
        })
        .detach();
    }

    /// One check at a time; a found update is downloaded, verified and put in place.
    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.update.checking || matches!(self.update.status.state, State::Installed | State::Disabled) {
            return;
        }
        let channel = self.vault.update_channel;
        self.update.checking = true;
        self.update.status.set(State::Checking, None, None);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(async move { updater::check(&updater::current_version(), channel) }).await;
            let package = match found {
                Ok(Some(package)) => package,
                done => {
                    let _ = this.update(cx, |shell, cx| shell.finish_update(done.map(|_| None), cx));
                    return;
                }
            };
            let version = package.version.clone();
            let _ = this.update(cx, |shell, cx| {
                shell.update.status.set(State::Downloading, Some(version.clone()), None);
                cx.notify();
            });
            // Progress every 256 KB reaches Settings → Version through a channel.
            let (progress, mut reports) = futures::channel::mpsc::unbounded::<(u64, Option<u64>)>();
            let task = cx.background_spawn(async move {
                updater::download(&package, |downloaded, total| {
                    let _ = progress.unbounded_send((downloaded, total));
                })
                .and_then(|bytes| updater::install(&bytes))
            });
            while let Some((downloaded, total)) = futures::StreamExt::next(&mut reports).await {
                let _ = this.update(cx, |shell, cx| {
                    shell.update.status.downloaded = Some(downloaded);
                    shell.update.status.total = total;
                    cx.notify();
                });
            }
            let installed = task.await;
            let _ = this.update(cx, |shell, cx| shell.finish_update(installed.map(|()| Some(version)), cx));
        })
        .detach();
    }

    fn finish_update(&mut self, result: Result<Option<String>, String>, cx: &mut Context<Self>) {
        self.update.checking = false;
        match result {
            Ok(Some(version)) => self.update.status.set(State::Installed, Some(version), None),
            Ok(None) => self.update.status.set(State::Idle, None, None),
            Err(error) => {
                eprintln!("[Termian] Update failed: {error}");
                self.update.status.set(State::Error, None, Some(error));
            }
        }
        cx.notify();
    }

    pub fn restart_for_update(&mut self, cx: &mut Context<Self>) {
        match updater::relaunch() {
            Ok(()) => cx.quit(),
            Err(error) => {
                self.update.restart_error = Some(error);
                cx.notify();
            }
        }
    }

    pub fn render_update_notice(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let status = &self.update.status;
        if self.update.dismissed || status.state != State::Installed {
            return None;
        }
        let title = format!("Termian {} установлен", status.version.as_deref().unwrap_or(""));
        let text = self
            .update
            .restart_error
            .clone()
            .unwrap_or_else(|| "Новая версия запустится после перезапуска. Открытые SSH-сессии при перезапуске закроются.".into());
        Some(super::motion::toast(
            div()
                .absolute()
                .right(px(16.))
                .bottom(px(16.))
                .max_w(px(360.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .px(px(16.))
                .py(px(14.))
                .border_1()
                .border_color(rgb(0x454950))
                .rounded(px(10.))
                .bg(rgb(0x272b30))
                .shadow_lg()
                .text_color(rgb(0xf2f3f5))
                .text_size(px(13.))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                .child(div().text_size(px(12.)).text_color(rgb(0x9ea5b3)).child(text))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(notice_button("update-later", "Позже", false).on_click(cx.listener(|shell, _, _, cx| {
                            shell.update.dismissed = true;
                            cx.notify();
                        })))
                        .child(notice_button("update-restart", "Перезапустить", true).on_click(cx.listener(|shell, _, _, cx| shell.restart_for_update(cx)))),
                ),
        ))
    }
}

fn notice_button(id: &'static str, label: &'static str, primary: bool) -> Stateful<Div> {
    let (border, background, hover) = if primary { (0x3d7bfd, 0x3d7bfd, 0x5a8fff) } else { (0x454950, 0x35393f, 0x454950) };
    div()
        .id(id)
        .min_h(px(28.))
        .px(px(12.))
        .py(px(4.))
        .flex()
        .items_center()
        .border_1()
        .border_color(rgb(border))
        .rounded(px(6.))
        .bg(rgb(background))
        .hover(move |style| style.bg(rgb(hover)))
        .text_size(px(12.))
        .cursor_pointer()
        .child(label)
}
