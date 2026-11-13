//! The Settings window (legacy `settings.html` / `setting-process`): a header,
//! the tab list on the left and the tab on the right. Account carries the
//! local account, «Своё облако» and the session log cleanup; SFTP the
//! «Open with…» associations; Logs the log switch; Version the updater.
//! It reads the main window's [`Shell`] and acts through it.

use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, rgb, rgba, size, App, Bounds, Context, Div, Entity, FocusHandle, FontWeight, KeyDownEvent, PathPromptOptions, SharedString, Stateful, Subscription,
    TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions,
};
use termian_native::self_cloud::{self, Status};
use termian_native::session_log::{self, PERIODS};
use termian_native::theme;
use termian_native::updater::State;

use super::cloud::Operation;
use super::dialog::{checkbox, edit, Key};
use crate::Shell;

gpui::actions!(termian, [OpenSettings, OpenVaults, OpenSftp, Quit]);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Account,
    Terminal,
    Sftp,
    Logs,
    Shortcuts,
    Version,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Account, Tab::Terminal, Tab::Sftp, Tab::Logs, Tab::Shortcuts, Tab::Version];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Account => "Account",
            Tab::Terminal => "Terminal",
            Tab::Sftp => "SFTP",
            Tab::Logs => "Logs",
            Tab::Shortcuts => "Shortcuts",
            Tab::Version => "Version",
        }
    }
}

const FOREGROUND: u32 = 0x1e2133;
const BORDER: u32 = 0x151826;
pub(super) const CARD: u32 = 0x292c3f;
const ERROR_TONE: u32 = 0xec706f;
pub(super) const MUTED: u32 = 0x9ea5b3;

pub struct SettingsWindow {
    pub(super) shell: Entity<Shell>,
    main: WindowHandle<Shell>,
    pub tab: Tab,
    pub(super) focus: FocusHandle,
    /// «Логи сессий»: the period waiting for confirmation (index in [`PERIODS`]).
    logs_period: Option<usize>,
    logs_busy: bool,
    /// Result of the last cleanup: success and text.
    logs_result: Option<(bool, String)>,
    /// Logs → «Disabled» waits for «Disable Logs».
    disable_logs: bool,
    /// The text field being typed into, and its text.
    pub(super) editing: Option<(Field, String)>,
    sftp_error: Option<String>,
    /// Terminal: the open drop-down list.
    pub(super) menu: Option<Menu>,
    /// Terminal: «Detect OS Script» is expanded.
    pub(super) script_open: bool,
    _observe: Subscription,
}

/// A text field of the Settings window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Field {
    /// SFTP: the app path of an extension.
    App(String),
    FontSize,
    KeepAlive,
    Scrollback,
    /// «Local Terminal Path»: a typed path to add.
    Shell,
    /// A font name typed into the font picker.
    Font,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Menu {
    Emulation,
    Font,
    Shell,
}

impl Shell {
    /// Settings… (⌘, / Ctrl+,): brings the window forward, or opens it.
    pub fn open_settings(&mut self, tab: Option<Tab>, cx: &mut Context<Self>) {
        if let Some(handle) = self.settings_window {
            let shown = handle.update(cx, |settings, window, cx| {
                if let Some(tab) = tab {
                    settings.tab = tab;
                }
                window.activate_window();
                cx.notify();
            });
            if shown.is_ok() {
                return;
            }
        }
        let Some(main) = self.main_window else { return };
        let shell = cx.entity();
        let bounds = Bounds::centered(None, size(px(960.), px(720.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some("Settings".into()), appears_transparent: cfg!(target_os = "macos"), ..Default::default() }),
            ..Default::default()
        };
        let opened = cx.open_window(options, |window, cx| {
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus);
                let observe = cx.observe(&shell, |_, _, cx| cx.notify());
                SettingsWindow {
                    shell,
                    main,
                    tab: tab.unwrap_or(Tab::Account),
                    focus,
                    logs_period: None,
                    logs_busy: false,
                    logs_result: None,
                    disable_logs: false,
                    editing: None,
                    sftp_error: None,
                    menu: None,
                    script_open: false,
                    _observe: observe,
                }
            })
        });
        match opened {
            Ok(handle) => {
                self.settings_window = Some(handle);
                // A fresh status, as the legacy window asks for one when it opens.
                self.check_cloud(false, cx);
            }
            Err(error) => self.vault_error = Some(format!("Could not open Settings: {error}")),
        }
    }

    /// Records sessions again, or stops and deletes this vault's logs.
    pub fn set_logs_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if !enabled {
            session_log::delete_all(&mut self.vault.history, &self.logs_dir);
        }
        self.vault.logs_disabled = !enabled;
        self.save_vault();
        cx.notify();
    }
}

/// The application menu (legacy `buildMacApplicationMenu`) and its keys.
pub fn install_menus(main: WindowHandle<Shell>, cx: &mut App) {
    use gpui::{KeyBinding, Menu, MenuItem};
    #[cfg(target_os = "macos")]
    cx.bind_keys([KeyBinding::new("cmd-,", OpenSettings, None), KeyBinding::new("cmd-q", Quit, None)]);
    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([KeyBinding::new("ctrl-,", OpenSettings, None), KeyBinding::new("ctrl-1", OpenVaults, None), KeyBinding::new("ctrl-2", OpenSftp, None)]);
    cx.on_action(move |_: &OpenSettings, cx| {
        let _ = main.update(cx, |shell, _, cx| shell.open_settings(None, cx));
    });
    cx.on_action(move |_: &OpenVaults, cx| {
        let _ = main.update(cx, |shell, window, cx| shell.show_pinned(0, window, cx));
    });
    cx.on_action(move |_: &OpenSftp, cx| {
        let _ = main.update(cx, |shell, window, cx| shell.show_pinned(1, window, cx));
    });
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.set_menus(vec![Menu {
        name: "termian".into(),
        items: vec![
            MenuItem::action("Vaults", OpenVaults),
            MenuItem::action("SFTP", OpenSftp),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("Quit termian", Quit),
        ],
    }]);
}

impl Shell {
    fn show_pinned(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.tabs.all()[index].id;
        self.tabs.activate(id);
        window.activate_window();
        window.focus(&self.root_focus);
        cx.notify();
    }
}

pub(super) fn section_title(text: &str) -> Div {
    div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child(text.to_owned())
}

pub(super) fn body_text(text: impl Into<SharedString>) -> Div {
    div().mt(px(8.)).text_size(px(14.)).line_height(px(21.)).text_color(rgb(MUTED)).child(text.into())
}

/// `SettingsSection`: a rounded card, 700 px at most, 30 px apart.
pub(super) fn section(children: Vec<gpui::AnyElement>) -> Div {
    div().w_full().max_w(px(700.)).mx_auto().mt(px(30.)).p(px(20.)).rounded(px(10.)).bg(rgb(CARD)).children(children)
}

pub(super) fn action_button(id: impl Into<SharedString>, label: &str, primary: bool, disabled: bool) -> Stateful<Div> {
    div()
        .id(id.into())
        .min_h(px(32.))
        .px(px(14.))
        .flex()
        .items_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if primary { theme::ACCENT } else { 0x454950 }))
        .when(primary, |d| d.bg(rgb(theme::ACCENT)))
        .text_size(px(13.))
        .text_color(rgb(if primary { 0xffffff } else { theme::TEXT }))
        .when(disabled, |d| d.opacity(0.5))
        .when(!disabled, |d| d.cursor_pointer().hover(|s| s.border_color(rgb(theme::ACCENT))))
        .child(label.to_owned())
}

fn notice(ok: bool, text: String) -> Div {
    div()
        .mt(px(8.))
        .px(px(10.))
        .py(px(8.))
        .rounded(px(6.))
        .text_size(px(12.))
        .line_height(px(18.))
        .text_color(rgb(if ok { theme::TEXT } else { ERROR_TONE }))
        .bg(if ok { rgba(0x00cc741a) } else { rgba(0xec706f1a) })
        .child(text)
}

pub(super) fn actions_row() -> Div {
    div().mt(px(16.)).flex().flex_wrap().items_center().gap(px(10.))
}

impl SettingsWindow {
    /// Runs `f` on the main window's shell, then brings that window forward.
    fn in_main(&self, cx: &mut Context<Self>, f: impl FnOnce(&mut Shell, &mut Window, &mut Context<Shell>) + 'static) {
        let _ = self.main.update(cx, |shell, window, cx| {
            f(shell, window, cx);
            window.activate_window();
        });
    }

    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.key == "w" && (keystroke.modifiers.platform || keystroke.modifiers.control) && self.editing.is_none() {
            window.remove_window();
            return;
        }
        if self.editing.is_none() {
            return;
        }
        let paste = cx.read_from_clipboard().and_then(|c| c.text());
        match edit(self.editing.as_mut().map(|(_, text)| text), keystroke, paste) {
            Key::Enter => self.commit_edit(cx),
            Key::Escape => self.editing = None,
            Key::Edited => self.live_edit(cx),
            Key::Ignored => return,
        }
        cx.notify();
    }

    /// Ends typing into the field: saves what it holds.
    pub(super) fn commit_edit(&mut self, cx: &mut Context<Self>) {
        let Some((field, text)) = self.editing.take() else { return };
        match field {
            Field::App(ext) => {
                let result = self.shell.update(cx, |shell, _| shell.sftp.actions.associations.set(&ext, PathBuf::from(text.trim())));
                self.sftp_error = result.err();
            }
            other => self.commit_terminal_field(other, &text, cx),
        }
    }

    fn clear_logs(&mut self, index: usize, cx: &mut Context<Self>) {
        let since = session_log::period_start(PERIODS[index].3, std::time::SystemTime::now());
        self.logs_busy = true;
        self.logs_result = None;
        let (cleared, cloud) = self.shell.update(cx, |shell, cx| {
            let cleared = session_log::clear(&mut shell.vault.history, &shell.logs_dir, since);
            shell.save_vault();
            cx.notify();
            (cleared, shell.vault.cloud.clone().and_then(|c| c.validate().ok().map(|client| (c, client))))
        });
        let remote = cx.background_spawn(async move { cloud.map(|(config, client)| client.delete_logs(since).map_err(|e| config.redact(&e))) });
        cx.spawn(async move |this, cx| {
            let remote = remote.await;
            let _ = this.update(cx, |settings, cx| {
                settings.logs_busy = false;
                settings.logs_period = None;
                settings.logs_result = Some((true, session_log::cleared_text(cleared, remote)));
                cx.notify();
            });
        })
        .detach();
    }

    fn render_account(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let shell = self.shell.read(cx);
        let status = shell.cloud.header.status.clone();
        let config = shell.vault.cloud.clone().filter(|c| c.validate().is_ok());
        let configured = config.is_some();
        let auto_sync = config.as_ref().is_some_and(|c| c.auto_sync);
        let (line, tone) = self_cloud::settings_status(&status, config.as_ref().map(|c| c.url.as_str()));
        let connected = configured && self_cloud::connected(&status);
        let checking = matches!(status, Status::Checking);

        let account = section(vec![
            section_title("Локальная учётная запись").into_any_element(),
            div().mt(px(8.)).text_size(px(14.)).text_color(rgb(theme::ACCENT)).child("Локальный пользователь").into_any_element(),
            body_text("Профиль постоянно активен. Вход в аккаунт Termius не требуется.").into_any_element(),
            body_text("Выход, смена аккаунта и удаление учётной записи отключены. Синхронизация работает через ваше облако.").into_any_element(),
        ]);

        let mut cloud = vec![
            section_title("Своё облако").into_any_element(),
            body_text("Личная синхронизация с вашим сервером. Отправить и получить изменения также можно кнопками в верхней панели главного окна.").into_any_element(),
            div()
                .mt(px(12.))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT))
                .child(div().size(px(8.)).flex_none().rounded_full().bg(rgb(tone.color())))
                .child(line)
                .into_any_element(),
        ];
        if let (true, Status::Error { message, credentials }) = (configured, &status) {
            cloud.push(
                notice(false, message.clone())
                    .when(*credentials, |d| {
                        d.child(div().mt(px(4.)).text_color(rgb(MUTED)).child("Автоматическая проверка приостановлена до изменения настроек или ручной проверки."))
                    })
                    .into_any_element(),
            );
        }
        cloud.push(
            checkbox("settings-auto-sync", "Синхронизировать автоматически каждый час (изменения устройства и облака объединяются)", configured && auto_sync)
                .mt(px(12.))
                .text_size(px(13.))
                .text_color(rgb(theme::TEXT))
                .when(!configured, |d| d.opacity(0.5))
                .when(configured, |d| {
                    d.on_click(cx.listener(move |settings, _, _, cx| {
                        settings.shell.update(cx, |shell, cx| {
                            if let Some(config) = &mut shell.vault.cloud {
                                config.auto_sync = !auto_sync;
                            }
                            shell.save_vault();
                            cx.notify();
                        });
                    }))
                })
                .into_any_element(),
        );
        let mut buttons = vec![action_button("settings-cloud-config", if configured { "Настройки подключения" } else { "Подключить сервер" }, false, false)
            .on_click(cx.listener(|settings, _, _, cx| settings.in_main(cx, |shell, window, cx| shell.open_cloud(None, window, cx))))
            .into_any_element()];
        if configured {
            buttons.push(
                action_button("settings-cloud-sync", "Синхронизировать", false, false)
                    .on_click(cx.listener(|settings, _, _, cx| settings.in_main(cx, |shell, window, cx| shell.open_cloud(Some(Operation::Sync), window, cx))))
                    .into_any_element(),
            );
        }
        if connected {
            buttons.push(
                action_button("settings-cloud-download", "Получить данные с сервера", false, false)
                    .on_click(cx.listener(|settings, _, _, cx| settings.in_main(cx, |shell, window, cx| shell.open_cloud(Some(Operation::Download), window, cx))))
                    .into_any_element(),
            );
        }
        if configured {
            buttons.push(
                action_button("settings-cloud-check", "Проверить", false, checking)
                    .when(!checking, |d| d.on_click(cx.listener(|settings, _, _, cx| settings.shell.update(cx, |shell, cx| shell.check_cloud(false, cx)))))
                    .into_any_element(),
            );
        }
        cloud.push(actions_row().children(buttons).into_any_element());

        let mut logs = vec![
            section_title("Логи сессий").into_any_element(),
            body_text("Логи всего вывода терминала хранятся 60 дней. Очистка удаляет записи на экране Logs, файлы логов на этом устройстве и их копии в своём облаке за выбранный период.")
                .into_any_element(),
        ];
        logs.push(match self.logs_period {
            Some(index) => actions_row()
                .child(div().text_size(px(13.)).text_color(rgb(theme::TEXT)).child(format!("Удалить логи {}? Это нельзя отменить.", PERIODS[index].2)))
                .child(
                    action_button("settings-logs-delete", if self.logs_busy { "Удаление…" } else { "Удалить" }, false, self.logs_busy)
                        .when(!self.logs_busy, |d| d.on_click(cx.listener(move |settings, _, _, cx| settings.clear_logs(index, cx)))),
                )
                .child(
                    action_button("settings-logs-cancel", "Отмена", false, self.logs_busy).when(!self.logs_busy, |d| {
                        d.on_click(cx.listener(|settings, _, _, cx| {
                            settings.logs_period = None;
                            cx.notify();
                        }))
                    }),
                )
                .into_any_element(),
            None => actions_row()
                .children(PERIODS.iter().enumerate().map(|(index, (id, label, ..))| {
                    action_button(SharedString::from(format!("settings-logs-{id}")), label, false, false).on_click(cx.listener(move |settings, _, _, cx| {
                        settings.logs_result = None;
                        settings.logs_period = Some(index);
                        cx.notify();
                    }))
                }))
                .into_any_element(),
        });
        if let Some((ok, text)) = &self.logs_result {
            logs.push(notice(*ok, text.clone()).into_any_element());
        }

        vec![account.into_any_element(), section(cloud).into_any_element(), section(logs).mb(px(60.)).into_any_element()]
    }

    fn render_sftp(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let apps = self.shell.read(cx).sftp.actions.associations.list();
        if apps.is_empty() {
            return vec![section(vec![div()
                .py(px(30.))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(8.))
                .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("No file type associations yet"))
                .child(div().max_w(px(310.)).text_center().text_size(px(14.)).text_color(rgb(MUTED)).child("Apps you choose using the “Open with…” option to open files via SFTP will appear here."))
                .into_any_element()])
            .into_any_element()];
        }
        let rows = apps.into_iter().map(|(ext, app)| {
            let editing = self.editing.as_ref().filter(|(e, _)| *e == Field::App(ext.clone())).map(|(_, text)| text.clone());
            let path = app.to_string_lossy().into_owned();
            let field_ext = ext.clone();
            let pick_ext = ext.clone();
            let forget_ext = ext.clone();
            div()
                .mt(px(10.))
                .h(px(40.))
                .px(px(10.))
                .flex()
                .items_center()
                .gap(px(10.))
                .rounded(px(10.))
                .bg(rgb(0x353950))
                .child(div().w(px(150.)).flex_none().truncate().child(format!(".{ext}")))
                .child(
                    div()
                        .id(SharedString::from(format!("settings-app-{ext}")))
                        .flex_1()
                        .min_w_0()
                        .h(px(30.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(rgb(if editing.is_some() { theme::ACCENT } else { 0x353950 }))
                        .child(div().truncate().child(editing.clone().unwrap_or(path.clone())))
                        .when(editing.is_some(), |d| d.child(div().mx(px(1.)).w(px(1.5)).h(px(16.)).bg(rgb(theme::TEXT))))
                        .on_click(cx.listener(move |settings, _, window, cx| {
                            if settings.editing.as_ref().is_some_and(|(e, _)| *e == Field::App(field_ext.clone())) {
                                return;
                            }
                            settings.commit_edit(cx);
                            settings.editing = Some((Field::App(field_ext.clone()), path.clone()));
                            window.focus(&settings.focus);
                            cx.notify();
                        })),
                )
                .child(action_button(SharedString::from(format!("settings-app-edit-{ext}")), "Изменить…", false, false).on_click(cx.listener(move |_, _, window, cx| {
                    let answer = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose application".into()) });
                    let ext = pick_ext.clone();
                    cx.spawn_in(window, async move |this, cx| {
                        let Ok(Ok(Some(paths))) = answer.await else { return };
                        let Some(app) = paths.into_iter().next() else { return };
                        let _ = this.update(cx, |settings, cx| {
                            let result = settings.shell.update(cx, |shell, _| shell.sftp.actions.associations.set(&ext, app));
                            settings.sftp_error = result.err();
                            cx.notify();
                        });
                    })
                    .detach();
                })))
                .child(
                    div()
                        .id(SharedString::from(format!("settings-app-forget-{ext}")))
                        .size(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .text_color(rgb(ERROR_TONE))
                        .hover(|s| s.bg(rgba(0xec706f22)))
                        .child("✕")
                        .on_click(cx.listener(move |settings, _, _, cx| {
                            if settings.editing.as_ref().is_some_and(|(e, _)| *e == Field::App(forget_ext.clone())) {
                                settings.editing = None;
                            }
                            let result = settings.shell.update(cx, |shell, _| shell.sftp.actions.associations.forget(&forget_ext));
                            settings.sftp_error = result.err();
                            cx.notify();
                        })),
                )
        });
        vec![section(vec![
            section_title("File type associations").into_any_element(),
            div()
                .mt(px(15.))
                .p(px(10.))
                .rounded(px(10.))
                .bg(rgb(0x23263a))
                .text_size(px(14.))
                .text_color(rgb(theme::TEXT))
                .child(div().px(px(10.)).h(px(40.)).flex().items_center().gap(px(10.)).child(div().w(px(150.)).child("File Type")).child("Open With"))
                .children(rows)
                .into_any_element(),
        ])
        .children(self.sftp_error.clone().map(|error| notice(false, error)))
        .into_any_element()]
    }

    fn render_logs(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let enabled = !self.shell.read(cx).vault.logs_disabled;
        let choice = |id: &'static str, label: &'static str, value: bool| {
            div()
                .id(id)
                .px(px(10.))
                .py(px(4.))
                .rounded(px(6.))
                .text_size(px(14.))
                .when(value == enabled, |d| d.bg(rgb(0x353950)).text_color(rgb(theme::TEXT)))
                .when(value != enabled, |d| d.text_color(rgb(MUTED)).cursor_pointer().hover(|s| s.bg(rgba(0xffffff14))))
                .child(label)
        };
        let mut card = vec![
            div().text_size(px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("Logs retention policies").into_any_element(),
            div().mt(px(10.)).text_size(px(14.)).text_color(rgb(MUTED)).child("Control how logs are stored in your vaults.").into_any_element(),
            div().my(px(10.)).h(px(1.)).w_full().bg(rgb(0x353950)).into_any_element(),
            div()
                .p(px(10.))
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(px(14.)).text_color(rgb(theme::TEXT)).child("Personal"))
                .child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .child(choice("settings-logs-enabled", "Enabled", true).on_click(cx.listener(|settings, _, _, cx| {
                            settings.disable_logs = false;
                            settings.shell.update(cx, |shell, cx| shell.set_logs_enabled(true, cx));
                        })))
                        .child(choice("settings-logs-disabled", "Disabled", false).on_click(cx.listener(move |settings, _, _, cx| {
                            if enabled {
                                settings.disable_logs = true;
                                cx.notify();
                            }
                        }))),
                )
                .into_any_element(),
        ];
        if self.disable_logs {
            card.push(
                div()
                    .mt(px(10.))
                    .p(px(15.))
                    .rounded(px(10.))
                    .bg(rgb(0x23263a))
                    .text_size(px(14.))
                    .text_color(rgb(theme::TEXT))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Confirm disabling logs"))
                    .child(
                        div()
                            .mt(px(10.))
                            .flex()
                            .child("Disabling logs will ")
                            .child(div().font_weight(FontWeight::BOLD).child("delete the logs "))
                            .child("from this vault: Personal"),
                    )
                    .child(
                        actions_row()
                            .justify_end()
                            .child(action_button("settings-logs-keep", "Cancel", false, false).on_click(cx.listener(|settings, _, _, cx| {
                                settings.disable_logs = false;
                                cx.notify();
                            })))
                            .child(
                                action_button("settings-logs-disable", "Disable Logs", false, false)
                                    .bg(rgb(ERROR_TONE))
                                    .border_color(rgb(ERROR_TONE))
                                    .on_click(cx.listener(|settings, _, _, cx| {
                                        settings.disable_logs = false;
                                        settings.shell.update(cx, |shell, cx| shell.set_logs_enabled(false, cx));
                                    })),
                            ),
                    )
                    .into_any_element(),
            );
        }
        vec![div().w_full().max_w(px(700.)).mx_auto().mt(px(30.)).p(px(15.)).rounded(px(15.)).bg(rgb(CARD)).children(card).into_any_element()]
    }

    fn render_version(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        let status = self.shell.read(cx).update.status.clone();
        let text = status.text(|ms| {
            use chrono::TimeZone;
            chrono::Local.timestamp_millis_opt(ms as i64).single().map(|t| t.format("%d.%m.%Y, %H:%M:%S").to_string()).unwrap_or_default()
        });
        let busy = matches!(status.state, State::Checking | State::Downloading);
        let button = if status.state == State::Installed {
            action_button("settings-update-restart", "Перезапустить и обновить", true, false)
                .on_click(cx.listener(|settings, _, _, cx| settings.shell.update(cx, |shell, cx| shell.restart_for_update(cx))))
        } else {
            let label = match status.state {
                State::Checking => "Проверка…",
                State::Downloading => "Загрузка…",
                _ => "Проверить обновления",
            };
            let disabled = busy || status.state == State::Disabled;
            action_button("settings-update-check", label, false, disabled)
                .when(!disabled, |d| d.on_click(cx.listener(|settings, _, _, cx| settings.shell.update(cx, |shell, cx| shell.check_updates(cx)))))
        };
        let percent = status.percent();
        vec![div()
            .p(px(30.))
            .flex()
            .flex_col()
            .child(div().text_size(px(20.)).font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child("Version"))
            .child(div().mt(px(4.)).text_size(px(14.)).text_color(rgb(MUTED)).child(format!("termian {}", termian_native::updater::current_version())))
            .child(
                div()
                    .mt(px(28.))
                    .max_w(px(420.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(div().text_size(px(20.)).font_weight(FontWeight::MEDIUM).text_color(rgb(theme::TEXT)).child("Обновления"))
                    .child(div().text_size(px(14.)).text_color(rgb(MUTED)).child(text))
                    .when(status.state == State::Downloading, |d| {
                        d.child(
                            div()
                                .h(px(6.))
                                .rounded(px(3.))
                                .overflow_hidden()
                                .bg(rgba(0x7f7f7f33))
                                .child(div().h_full().rounded(px(3.)).bg(rgb(theme::ACCENT)).when_some(percent, |d, p| d.w(gpui::relative(p as f32 / 100.))).when(percent.is_none(), |d| d.w_full().opacity(0.5))),
                        )
                    })
                    .when_some(status.error.clone().filter(|_| status.state == State::Error), |d, error| d.child(div().text_size(px(13.)).text_color(rgb(ERROR_TONE)).child(error)))
                    .child(div().mt(px(4.)).flex().gap(px(8.)).child(button)),
            )
            .into_any_element()]
    }

    fn render_not_ported(&self, tab: Tab) -> Vec<gpui::AnyElement> {
        vec![section(vec![
            section_title(tab.title()).into_any_element(),
            body_text("Этот раздел настроек ещё не перенесён в нативный клиент: пока он доступен в legacy-клиенте.").into_any_element(),
        ])
        .into_any_element()]
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.tab {
            Tab::Account => self.render_account(cx),
            Tab::Sftp => self.render_sftp(cx),
            Tab::Logs => self.render_logs(cx),
            Tab::Version => self.render_version(cx),
            Tab::Terminal => self.render_terminal(cx),
            tab @ Tab::Shortcuts => self.render_not_ported(tab),
        };
        let tabs = Tab::ALL.into_iter().map(|tab| {
            let selected = tab == self.tab;
            div()
                .id(SharedString::from(format!("settings-tab-{tab:?}")))
                .h(px(36.))
                .mx(px(10.))
                .my(px(4.))
                .px(px(15.))
                .flex()
                .items_center()
                .rounded(px(10.))
                .text_size(px(14.))
                .text_color(rgb(theme::TEXT))
                .when(selected, |d| d.bg(rgb(theme::SURFACE_ACTIVE)))
                .when(!selected, |d| d.hover(|s| s.bg(rgb(theme::SURFACE))))
                .child(tab.title())
                .on_click(cx.listener(move |settings, _, _, cx| {
                    settings.commit_edit(cx);
                    settings.menu = None;
                    settings.tab = tab;
                    cx.notify();
                }))
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(FOREGROUND))
            .text_color(rgb(theme::TEXT))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key))
            .child(
                div()
                    .h(px(80.))
                    .flex_none()
                    .px(px(20.))
                    .pt(px(25.))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .text_size(px(18.))
                    .font_weight(FontWeight::BOLD)
                    .child("Settings"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(div().w(px(220.)).h_full().flex_none().py(px(4.)).border_r_1().border_color(rgb(BORDER)).children(tabs))
                    .child(div().id("settings-content").flex_1().h_full().overflow_y_scroll().px(px(20.)).bg(rgb(theme::BACKGROUND)).children(content)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_follow_the_legacy_settings_routes() {
        let titles: Vec<_> = Tab::ALL.iter().map(|t| t.title()).collect();
        assert_eq!(titles, ["Account", "Terminal", "SFTP", "Logs", "Shortcuts", "Version"]);
    }
}
