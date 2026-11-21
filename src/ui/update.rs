//! Automatic updates (`termian_native::updater`) and the legacy
//! `update-notice.js`: once an update is downloaded, verified and in place,
//! a notice in the corner offers a restart. The user chooses when, so open SSH
//! sessions are never cut off by an update.

use gpui::{div, prelude::*, px, rgb, Context, Div, FontWeight, Stateful, Task};
use std::time::Duration;
use termian_native::updater::{self, Channel, Mode, Package, Schedule, State, Status};

use crate::Shell;

#[derive(Default)]
pub struct UpdateState {
    pub status: Status,
    pub(super) settings_error: Option<String>,
    dismissed: bool,
    dismissed_available: Option<String>,
    checking: bool,
    timer: Option<Task<()>>,
    available: Option<Package>,
    restart_error: Option<String>,
}

impl Shell {
    pub(super) fn set_update_settings(&mut self, channel: Channel, schedule: Schedule, cx: &mut Context<Self>) {
        let previous = (self.vault.update_channel, self.vault.update_schedule);
        if self.update.checking || previous == (channel, schedule)
            || (self.update.status.state == State::Installed && channel != previous.0) {
            return;
        }
        self.vault.update_channel = channel;
        self.vault.update_schedule = schedule;
        match self.store.save(&self.vault) {
            Ok(()) => {
                self.update.settings_error = None;
                if channel != previous.0 && self.update.status.state != State::Disabled {
                    self.update.available = None;
                    self.update.status = Status::default();
                }
                let delay = if previous.1.mode == Mode::Manual { updater::FIRST_CHECK } else { schedule.interval.duration() };
                self.schedule_update_check(delay, cx);
            }
            Err(error) => {
                (self.vault.update_channel, self.vault.update_schedule) = previous;
                self.update.settings_error = Some(format!("Не удалось сохранить настройки обновлений: {error}"));
            }
        }
        cx.notify();
    }

    pub fn start_updates(&mut self, cx: &mut Context<Self>) {
        if !updater::enabled() {
            self.update.status.state = State::Disabled;
            return;
        }
        self.schedule_update_check(updater::FIRST_CHECK, cx);
    }

    fn schedule_update_check(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.update.timer = None;
        if self.update.checking || matches!(self.update.status.state, State::Installed | State::Disabled) {
            return;
        }
        let Some(download) = self.vault.update_schedule.mode.automatic_download() else { return };
        self.update.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |shell, cx| {
                if let Some(task) = shell.update.timer.take() {
                    task.detach();
                }
                shell.begin_update_check(download, cx);
            });
        }));
    }

    pub fn check_updates(&mut self, cx: &mut Context<Self>) {
        self.begin_update_check(false, cx);
    }

    fn begin_update_check(&mut self, download: bool, cx: &mut Context<Self>) {
        if self.update.checking || matches!(self.update.status.state, State::Installed | State::Disabled) {
            return;
        }
        let channel = self.vault.update_channel;
        self.update.timer = None;
        self.update.available = None;
        self.update.checking = true;
        self.update.status.set(State::Checking, None, None);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(async move { updater::check(&updater::current_version(), channel) }).await;
            let _ = this.update(cx, |shell, cx| match found {
                Ok(Some(package)) if download => shell.download_package(package, cx),
                Ok(Some(package)) => {
                    shell.update.status.set(State::Available, Some(package.version.clone()), None);
                    shell.update.available = Some(package);
                    shell.end_update(cx);
                }
                Ok(None) => shell.finish_update(Ok(None), cx),
                Err(error) => shell.finish_update(Err(error), cx),
            });
        })
        .detach();
    }

    pub(super) fn download_update(&mut self, cx: &mut Context<Self>) {
        if self.update.checking || self.update.status.state != State::Available {
            return;
        }
        if let Some(package) = self.update.available.take() {
            self.download_package(package, cx);
        }
    }

    fn download_package(&mut self, package: Package, cx: &mut Context<Self>) {
        self.update.timer = None;
        self.update.checking = true;
        let version = package.version.clone();
        self.update.status.set(State::Downloading, Some(version.clone()), None);
        cx.notify();
        cx.spawn(async move |this, cx| {
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
        match result {
            Ok(Some(version)) => self.update.status.set(State::Installed, Some(version), None),
            Ok(None) => self.update.status.set(State::Idle, None, None),
            Err(error) => {
                eprintln!("[Termian] Update failed: {error}");
                self.update.status.set(State::Error, None, Some(error));
            }
        }
        self.end_update(cx);
    }

    fn end_update(&mut self, cx: &mut Context<Self>) {
        self.update.checking = false;
        self.schedule_update_check(self.vault.update_schedule.interval.duration(), cx);
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
        let installed = status.state == State::Installed;
        let version = status.version.as_ref()?;
        if installed && self.update.dismissed
            || !installed && (status.state != State::Available || self.update.dismissed_available.as_ref() == Some(version)) {
            return None;
        }
        let (title, text, action) = if installed {
            (format!("Termian {version} установлен"), self.update.restart_error.clone()
                .unwrap_or_else(|| "Новая версия запустится после перезапуска. Открытые SSH-сессии при перезапуске закроются.".into()), "Перезапустить")
        } else {
            (format!("Доступен Termian {version}"), "Скачать обновление можно в Settings → Version.".into(), "Скачать")
        };
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
                        .child(notice_button("update-later", "Позже", false).on_click(cx.listener(move |shell, _, _, cx| {
                            if installed {
                                shell.update.dismissed = true;
                            } else {
                                shell.update.dismissed_available = shell.update.status.version.clone();
                            }
                            cx.notify();
                        })))
                        .child(notice_button("update-action", action, true).on_click(cx.listener(move |shell, _, _, cx| {
                            if installed { shell.restart_for_update(cx); }
                            else { shell.download_update(cx); }
                        }))),
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
