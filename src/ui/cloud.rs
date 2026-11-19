//! «Своё облако»: the legacy `self-cloud-ui.cjs` dialogs and the header
//! buttons. The settings form keeps secrets write-only; Sync, Upload and
//! Download first show what would move, then run only when confirmed, with
//! the conflict choice offered but never made automatically. Network work
//! runs off the UI thread; the hourly sync follows the `auto_sync` switch.

use std::collections::HashSet;
use std::time::Duration;

use gpui::{div, prelude::*, px, rgb, rgba, svg, AnimationExt, AnyElement, Context, Div, FontWeight, KeyDownEvent, SharedString, Stateful, Window};
use termian_native::log_backup;
use termian_native::self_cloud::{self, ConfigChanges, Header, Preview, Status};
use termian_native::theme;
use termian_native::vault::Vault;

use super::assets::icon;
use super::dialog::Key;
use super::text_input::{self, Mode};
use crate::Shell;

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const AUTO_SYNC_FIRST: Duration = Duration::from_secs(60);
const AUTO_SYNC_INTERVAL: Duration = Duration::from_secs(60 * 60);

const BORDER: u32 = 0x454950;
const PANEL: u32 = 0x202328;
const GROUP: u32 = 0x292d38;
const UPLOAD_TONE: u32 = 0xf0bc6b;
const DOWNLOAD_TONE: u32 = 0x6ac9fd;
const ERROR_TONE: u32 = 0xec706f;
/// Legacy title-bar badge text (`#131626`).
const BADGE_TEXT: u32 = 0x131626;

const HELP: &str = "Токен и пароли сохраняются в конфиге этого профиля. Сохранённые значения не передаются в интерфейс. Чтобы заменить секрет, введите новый; пустое поле оставляет прежний.";
const AUTO_SYNC: &str = "Синхронизировать автоматически каждый час (изменения устройства и облака объединяются)";
const SAVED_SECRET: &str = "Пароль сохранён. Поле — только для замены.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Sync,
    Upload,
    Download,
}

impl Operation {
    fn title(self) -> &'static str {
        match self {
            Operation::Sync => "Синхронизация",
            Operation::Upload => "Отправить изменения",
            Operation::Download => "Получить изменения",
        }
    }

    fn warning(self) -> &'static str {
        match self {
            Operation::Download => "Данные приложения будут заменены снимком из облака. Перед заменой будет создана зашифрованная резервная копия.",
            Operation::Sync => "Изменения устройства и облака будут объединены: новые, изменённые и удалённые записи переносятся в обе стороны. Перед применением изменений из облака создаётся зашифрованная резервная копия.",
            Operation::Upload => "Снимок в облаке будет полностью заменён текущими данными. Изменения других устройств будут затёрты.",
        }
    }

    fn confirm(self) -> &'static str {
        match self {
            Operation::Download => "Получить и заменить",
            Operation::Sync => "Синхронизировать",
            Operation::Upload => "Отправить и заменить",
        }
    }
}

/// The status line under the fields.
#[derive(Clone, Debug, PartialEq)]
enum Out {
    Working(String),
    Success(String),
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Url,
    Token,
    Passphrase,
    BasicUser,
    BasicPassword,
}

#[derive(Debug, Default)]
struct ConfigForm {
    url: String,
    token: String,
    passphrase: String,
    basic: bool,
    basic_user: String,
    basic_password: String,
    /// What is saved, never the values themselves.
    has_token: bool,
    has_passphrase: bool,
    has_basic_password: bool,
    field: Option<Field>,
}

impl ConfigForm {
    fn from_vault(vault: &Vault) -> Self {
        let mut form = ConfigForm { field: Some(Field::Url), ..ConfigForm::default() };
        form.load(vault);
        form
    }

    /// Metadata of the saved settings; typed secrets are cleared.
    fn load(&mut self, vault: &Vault) {
        let saved = vault.cloud.clone().unwrap_or_default();
        self.url = saved.url;
        self.basic = !saved.basic_user.is_empty();
        self.basic_user = saved.basic_user;
        self.has_token = !saved.token.is_empty();
        self.has_passphrase = !saved.passphrase.is_empty();
        self.has_basic_password = !saved.basic_password.is_empty();
        self.token.clear();
        self.passphrase.clear();
        self.basic_password.clear();
    }

    fn value(&mut self, field: Field) -> Option<&mut String> {
        Some(match field {
            Field::Url => &mut self.url,
            Field::Token => &mut self.token,
            Field::Passphrase => &mut self.passphrase,
            Field::BasicUser if self.basic => &mut self.basic_user,
            Field::BasicPassword if self.basic => &mut self.basic_password,
            _ => return None,
        })
    }

    /// The browser's `reportValidity` checks of the legacy form.
    fn missing(&self) -> Option<&'static str> {
        if self.url.trim().is_empty() {
            Some("Введите адрес сервера")
        } else if self.token.is_empty() && !self.has_token {
            Some("Введите токен доступа")
        } else if self.passphrase.is_empty() && !self.has_passphrase {
            Some("Введите пароль шифрования")
        } else if !self.passphrase.is_empty() && self.passphrase.chars().count() < 12 {
            Some("Пароль шифрования должен содержать не менее 12 символов")
        } else if self.basic && self.basic_user.trim().is_empty() {
            Some("Введите логин Basic auth")
        } else if self.basic && self.basic_password.is_empty() && !self.has_basic_password {
            Some("Введите пароль Basic auth")
        } else {
            None
        }
    }

    fn changes(&self) -> ConfigChanges {
        ConfigChanges {
            url: self.url.clone(),
            token: self.token.clone(),
            passphrase: self.passphrase.clone(),
            basic_user: if self.basic { self.basic_user.trim().to_owned() } else { String::new() },
            basic_password: if self.basic { self.basic_password.clone() } else { String::new() },
        }
    }

    fn next(&mut self, back: bool) {
        let order: Vec<Field> = [Field::Url, Field::Token, Field::Passphrase].into_iter().chain(self.basic.then_some([Field::BasicUser, Field::BasicPassword]).into_iter().flatten()).collect();
        let at = self.field.and_then(|f| order.iter().position(|o| *o == f)).unwrap_or(0);
        self.field = Some(order[if back { (at + order.len() - 1) % order.len() } else { (at + 1) % order.len() }]);
    }
}

/// Sync, Upload or Download.
#[derive(Debug)]
struct Run {
    operation: Operation,
    configured: bool,
    /// What the preview panel shows; `None` before the first check.
    panel: Option<Status>,
    /// Conflicts found by a confirmed sync.
    checked_at: Option<i64>,
    confirm: bool,
    /// Groups the user opened or closed, by position.
    toggled: HashSet<usize>,
}

impl Run {
    fn checked(&mut self, status: Status, at: i64) {
        self.panel = Some(status);
        self.checked_at = Some(at);
        self.confirm = true;
    }

    /// Applies a finished operation; the status text is returned.
    fn finished(&mut self, result: &Result<String, String>) -> Out {
        self.toggled.clear();
        match result {
            Ok(text) => {
                self.panel = Some(Status::Checked(Preview::Synced));
                Out::Success(text.clone())
            }
            Err(error) => {
                self.confirm = true;
                Out::Error(error.clone())
            }
        }
    }
}

#[derive(Debug)]
enum Body {
    Config(ConfigForm),
    Run(Run),
}

#[derive(Debug)]
pub struct CloudDialog {
    body: Body,
    out: Option<Out>,
    busy: bool,
}

/// Own-cloud state of the main window.
#[derive(Default)]
pub struct CloudState {
    dialog: Option<CloudDialog>,
    pub header: Header,
    /// One cloud operation at a time, like the legacy queue.
    running: bool,
    started: bool,
    /// Session log backup: one run at a time; stopped for an old server
    /// until restart, paused for an hour after a credential error (as legacy).
    logs_running: bool,
    logs_unsupported: bool,
    logs_paused_until: Option<std::time::Instant>,
}

/// Dark hover text for the header buttons.
pub(crate) struct Tooltip(pub String);

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().font_family(theme::UI_FONT).px_2().py_1().rounded_md().bg(rgb(0x000000)).text_xs().text_color(rgb(0xffffff)).child(self.0.clone())
    }
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn clock(at: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(at, 0).single().map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_default()
}

fn configured(vault: &Vault) -> bool {
    vault.cloud.as_ref().is_some_and(|c| c.validate().is_ok())
}

/// Takes the synced copy unless the vault was edited while the cloud worked
/// on it; then the old merge base stays and the next sync carries both the
/// edit and what came from the cloud. Device-local history is always kept: a
/// session may end meanwhile.
fn adopt(live: &mut Vault, started: &Vault, mut synced: Vault) -> bool {
    let edited = Vault { history: Vec::new(), ..live.clone() } != Vault { history: Vec::new(), ..started.clone() };
    if edited {
        return false;
    }
    synced.history = std::mem::take(&mut live.history);
    *live = synced;
    true
}

fn button(id: &str, label: &str, primary: bool, disabled: bool) -> Stateful<Div> {
    div()
        .id(SharedString::from(id.to_owned()))
        .min_h(px(32.))
        .px(px(12.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(if primary { theme::ACCENT } else { BORDER }))
        .bg(rgb(if primary { theme::ACCENT } else { theme::SURFACE_ACTIVE }))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(0xffffff))
        .when(!disabled, |d| d.hover(|s| s.opacity(0.85)))
        .when(disabled, |d| d.opacity(0.45))
        .child(label.to_owned())
}

fn notice(text: &str) -> Div {
    div().px(px(12.)).py(px(10.)).rounded(px(6.)).border_1().border_color(rgb(BORDER)).bg(rgb(PANEL)).text_xs().text_color(rgb(theme::TEXT)).child(text.to_owned())
}

fn muted(text: impl Into<SharedString>) -> Div {
    div().mt(px(8.)).text_xs().text_color(rgb(theme::TEXT_MUTED)).child(text.into())
}

impl Shell {
    /// `None` opens the settings, as the legacy `open()` without an operation.
    pub fn open_cloud(&mut self, operation: Option<Operation>, window: &mut Window, cx: &mut Context<Self>) {
        text_input::reset(cx);
        if self.cloud.dialog.is_some() {
            return;
        }
        let body = match operation {
            None => Body::Config(ConfigForm::from_vault(&self.vault)),
            Some(operation) => Body::Run(Run { operation, configured: configured(&self.vault), panel: None, checked_at: None, confirm: false, toggled: HashSet::new() }),
        };
        let checking = matches!(&body, Body::Run(run) if run.configured);
        let out = match &body {
            Body::Run(run) if !run.configured => Some(Out::Working("Сначала сохраните адрес сервера, токен и пароль в настройках своего облака.".into())),
            Body::Run(_) => Some(Out::Working("Проверяю изменения в облаке и на устройстве…".into())),
            Body::Config(_) => None,
        };
        self.cloud.dialog = Some(CloudDialog { body, out, busy: checking });
        window.focus(&self.cloud_focus);
        if checking {
            self.check_cloud(true, cx);
        }
        cx.notify();
    }

    fn close_cloud(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.cloud.dialog.as_ref().is_some_and(|d| d.busy) {
            return;
        }
        self.cloud.dialog = None;
        window.focus(&self.root_focus);
        cx.notify();
    }

    /// Starts the background check and the hourly sync, once per window.
    pub fn start_cloud(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.cloud.started, true) {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(1800)).await;
            loop {
                let alive = this.update(cx, |shell, cx| {
                    if !shell.cloud.header.paused() {
                        shell.check_cloud(false, cx);
                    }
                });
                if alive.is_err() {
                    break;
                }
                cx.background_executor().timer(POLL_INTERVAL).await;
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(log_backup::FIRST_RUN).await;
            loop {
                if this.update(cx, |shell, cx| shell.backup_logs(cx)).is_err() {
                    break;
                }
                cx.background_executor().timer(log_backup::INTERVAL).await;
            }
        })
        .detach();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTO_SYNC_FIRST).await;
            loop {
                let alive = this.update(cx, |shell, cx| shell.auto_sync(cx));
                if alive.is_err() {
                    break;
                }
                cx.background_executor().timer(AUTO_SYNC_INTERVAL).await;
            }
        })
        .detach();
    }

    /// A session ended: its log is complete, back it up shortly.
    pub fn backup_logs_soon(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(log_backup::AFTER_SESSION).await;
            let _ = this.update(cx, |shell, cx| shell.backup_logs(cx));
        })
        .detach();
    }

    /// Silent: it never touches the sync status or the header badges. It
    /// follows the hourly auto-sync switch, as the legacy backup does.
    fn backup_logs(&mut self, cx: &mut Context<Self>) {
        let state = &self.cloud;
        if state.dialog.as_ref().is_some_and(|d| d.busy && matches!(d.body, Body::Config(_)))
            || state.logs_running || state.logs_unsupported || state.logs_paused_until.is_some_and(|t| std::time::Instant::now() < t) {
            return;
        }
        let Some(cloud) = self.vault.cloud.as_ref().filter(|c| c.auto_sync).and_then(|c| c.validate().ok()) else { return };
        let history = self.vault.history.clone();
        if history.iter().all(|e| e.log.is_none()) {
            return;
        }
        self.cloud.logs_running = true;
        let logs_dir = self.logs_dir.clone();
        let task = cx.background_spawn(async move { log_backup::backup(&cloud, &logs_dir, &history) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, _| {
                shell.cloud.logs_running = false;
                match result {
                    Ok(outcome) => shell.cloud.logs_unsupported = outcome.unsupported,
                    Err(message) => {
                        if self_cloud::is_credential_error(&message) {
                            shell.cloud.logs_paused_until = Some(std::time::Instant::now() + log_backup::INTERVAL);
                        }
                        let message = shell.vault.cloud.as_ref().map_or(message.clone(), |c| c.redact(&message));
                        eprintln!("[Termian] Session log backup failed: {}", message.chars().take(240).collect::<String>());
                    }
                }
            });
        })
        .detach();
    }

    /// Hourly merge sync.
    fn auto_sync(&mut self, cx: &mut Context<Self>) {
        let enabled = self.vault.cloud.as_ref().is_some_and(|c| c.auto_sync) && configured(&self.vault);
        if !enabled || self.cloud.header.paused() || self.cloud.running || self.cloud.dialog.is_some() {
            return;
        }
        self.run_cloud(Operation::Sync, cx);
    }

    /// Read-only comparison; `for_dialog` also fills an open dialog's panel.
    pub fn check_cloud(&mut self, for_dialog: bool, cx: &mut Context<Self>) {
        if self.cloud.running {
            return;
        }
        if !configured(&self.vault) {
            self.cloud.header.set(Status::Checked(Preview::Unconfigured));
            return;
        }
        self.cloud.running = true;
        self.cloud.header.set(Status::Checking);
        let vault = self.vault.clone();
        let task = cx.background_spawn(async move { self_cloud::preview(&vault) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, cx| {
                shell.cloud.running = false;
                let status = match result {
                    Ok(preview) => Status::Checked(preview),
                    Err(message) => Status::Error { credentials: self_cloud::is_credential_error(&message), message },
                };
                shell.cloud.header.set(status.clone());
                if let Some(dialog) = &mut shell.cloud.dialog {
                    if let Body::Run(run) = &mut dialog.body {
                        if for_dialog || run.panel.is_some() {
                            run.checked(status, now());
                            if for_dialog {
                                dialog.out = None;
                                dialog.busy = false;
                            }
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Runs a confirmed operation on a copy of the vault and adopts the result.
    fn run_cloud(&mut self, operation: Operation, cx: &mut Context<Self>) {
        if self.cloud.running {
            return;
        }
        self.cloud.running = true;
        if operation == Operation::Sync {
            self.cloud.header.set(Status::Syncing);
        }
        let started = self.vault.clone();
        let mut vault = started.clone();
        let backups = self_cloud::default_backup_dir();
        let task = cx.background_spawn(async move {
            let result = match operation {
                Operation::Sync => self_cloud::sync(&mut vault, &backups),
                Operation::Upload => self_cloud::upload(&mut vault),
                Operation::Download => self_cloud::download(&mut vault, &backups),
            };
            (vault, result)
        });
        cx.spawn(async move |this, cx| {
            let (vault, result) = task.await;
            let _ = this.update(cx, |shell, cx| {
                shell.cloud.running = false;
                let mut result = result;
                if result.is_ok() {
                    let mut candidate = shell.vault.clone();
                    if !adopt(&mut candidate, &started, vault) {
                        result = Err("Конфигурация изменилась во время операции. Проверьте изменения и повторите.".into());
                    } else if let Err(error) = shell.store.save(&candidate) {
                        result = Err(format!("Не удалось сохранить результат операции в Vault: {error}"));
                    } else {
                        shell.vault = candidate;
                    }
                }
                match &result {
                    Ok(_) => {
                        shell.cloud.header.synced_at = Some(now());
                        shell.cloud.header.set(Status::Checked(Preview::Synced));
                    }
                    Err(message) => {
                        shell.cloud.header.set(Status::Error { credentials: self_cloud::is_credential_error(message), message: message.clone() });
                    }
                }
                if let Some(dialog) = &mut shell.cloud.dialog {
                    if let Body::Run(run) = &mut dialog.body {
                        dialog.out = Some(run.finished(&result));
                        dialog.busy = false;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn confirm_cloud(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.cloud.dialog else { return };
        let Body::Run(run) = &mut dialog.body else { return };
        if self.cloud.running || dialog.busy || !run.confirm {
            return;
        }
        let operation = run.operation;
        run.confirm = false;
        dialog.busy = true;
        dialog.out = Some(Out::Working("Выполняется…".into()));
        self.run_cloud(operation, cx);
        cx.notify();
    }

    fn save_cloud_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(CloudDialog { body: Body::Config(form), out, busy }) = &mut self.cloud.dialog else { return };
        if *busy { return; }
        let candidate = form.missing().map_or_else(|| self_cloud::candidate_config(&self.vault, form.changes()), |error| Err(error.into()));
        let candidate = match candidate {
            Ok(candidate) => candidate,
            Err(error) => {
                *out = Some(Out::Error(error));
                cx.notify();
                return;
            }
        };
        let rotating = self.vault.cloud.as_ref().is_some_and(|saved| {
            !saved.passphrase.is_empty() && saved.passphrase != candidate.passphrase
                && termian_native::sync::server_key(&saved.url) == termian_native::sync::server_key(&candidate.url)
        });
        if !rotating {
            self.commit_cloud_config(candidate, false, cx);
            return;
        }
        if self.cloud.running || self.cloud.logs_running {
            *out = Some(Out::Error("Дождитесь завершения операции с облаком и повторите.".into()));
            cx.notify();
            return;
        }
        *busy = true;
        self.cloud.running = true;
        let confirm = window.prompt(gpui::PromptLevel::Warning, "Сменить пароль шифрования?",
            Some("Сначала перешифруйте снимок в веб-панели сервера. Клиент проверит новый пароль и будет использовать его для следующих отправок."),
            &["Проверить и сохранить", "Отмена"], cx);
        cx.spawn(async move |this, cx| {
            let result = if confirm.await == Ok(0) {
                let config = candidate.clone();
                Some(cx.background_executor().spawn(async move { self_cloud::verify_cloud_password(&config) }).await)
            } else {
                None
            };
            let _ = this.update(cx, |shell, cx| {
                shell.cloud.running = false;
                if let Some(dialog) = &mut shell.cloud.dialog {
                    dialog.busy = false;
                    if let Some(Err(error)) = &result {
                        dialog.out = Some(Out::Error(error.clone()));
                    }
                }
                if matches!(result, Some(Ok(()))) {
                    shell.commit_cloud_config(candidate, true, cx);
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }

    fn commit_cloud_config(&mut self, candidate: termian_native::cloud::CloudConfig, rotated: bool, cx: &mut Context<Self>) {
        let Some(CloudDialog { body: Body::Config(form), out, .. }) = &mut self.cloud.dialog else { return };
        let previous = self.vault.cloud.replace(candidate);
        if let Err(error) = self.store.save(&self.vault) {
            self.vault.cloud = previous;
            *out = Some(Out::Error(error));
            cx.notify();
            return;
        }
        form.load(&self.vault);
        *out = Some(Out::Success(if rotated { "Новый пароль сохранён. Следующие отправки будут зашифрованы им." } else { "Настройки сохранены" }.into()));
        if rotated {
            self.cloud.logs_paused_until = None;
        }
        self.cloud.header.reset();
        self.check_cloud(false, cx);
        cx.notify();
    }

    fn cloud_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let Some(dialog) = &mut self.cloud.dialog else { return };
        let keystroke = &event.keystroke;
        if keystroke.key == "escape" {
            self.close_cloud(window, cx);
            return;
        }
        if dialog.busy {
            return;
        }
        match &mut dialog.body {
            Body::Config(form) => {
                if keystroke.key == "tab" {
                    form.next(keystroke.modifiers.shift);
                    cx.notify();
                    return;
                }
                let field = form.field;
                let mode = if matches!(field, Some(Field::Token | Field::Passphrase | Field::BasicPassword)) { Mode::Secret } else { Mode::Line };
                let value = field.and_then(|f| form.value(f));
                match text_input::edit(("cloud", field), value, keystroke, mode, cx) {
                    Key::Enter => self.save_cloud_config(window, cx),
                    Key::Edited => cx.notify(),
                    _ => {}
                }
            }
            Body::Run(_) => {
                if keystroke.key == "enter" {
                    self.confirm_cloud(cx);
                }
            }
        }
    }

    pub fn render_cloud_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let header = &self.cloud.header;
        let title = header.sync_title(clock);
        let sync_icon = svg().path(icon("cloud_sync")).size(px(18.)).text_color(rgb(theme::TEXT));
        div().id("cloud-sync").relative().size(px(30.)).flex().items_center().justify_center().rounded_md()
            .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
            .tooltip(move |_, cx| cx.new(|_| Tooltip(title.clone())).into())
            .child(if header.syncing() {
                sync_icon.with_animation(
                    "cloud-syncing",
                    gpui::Animation::new(Duration::from_secs(1)).repeat(),
                    |icon, delta| icon.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                ).into_any_element()
            } else {
                sync_icon.into_any_element()
            })
            .children(header.sync_badge().map(|(mark, tone)| {
                div().absolute().top(px(-2.)).right(px(-2.)).size(px(13.)).rounded_full().bg(rgb(tone)).flex().items_center().justify_center().text_size(px(9.)).font_weight(FontWeight::BOLD).text_color(rgb(BADGE_TEXT)).child(mark)
            }))
            .on_click(cx.listener(|shell, _, window, cx| shell.open_cloud(Some(Operation::Sync), window, cx)))
            .into_any_element()
    }

    fn cloud_input(&self, field: Field, label: &str, value: &str, secret: bool, placeholder: &str, hint: Option<&str>, enabled: bool, cx: &mut Context<Self>) -> Div {
        let Some(CloudDialog { body: Body::Config(form), busy, .. }) = &self.cloud.dialog else { return div() };
        let focused = form.field == Some(field) && enabled && !busy;
        let key = ("cloud", Some(field));
        let mode = if secret { Mode::Secret } else { Mode::Line };
        let input = div()
            .id(SharedString::from(format!("cloud-field-{field:?}")))
            .h(px(36.)).px(px(11.)).flex().items_center().rounded(px(6.)).border_1()
            .border_color(rgb(if focused { theme::ACCENT } else { BORDER }))
            .bg(rgb(theme::SURFACE_ACTIVE)).text_size(px(13.)).font_weight(FontWeight::NORMAL)
            .when(!enabled || *busy, |d| d.opacity(0.5))
            .child(text_input::text(key, value, placeholder, focused, mode, cx));
        let input = if enabled && !busy {
            text_input::field(input, key, mode, move |shell, window, _| {
                if let Some(CloudDialog { body: Body::Config(form), .. }) = &mut shell.cloud.dialog {
                    form.field = Some(field);
                }
                window.focus(&shell.cloud_focus);
            }, cx)
        } else {
            input
        };
        div().flex().flex_col().gap(px(7.)).text_xs().font_weight(FontWeight::MEDIUM)
            .text_color(rgb(theme::TEXT)).child(label.to_owned()).child(input)
            .children(hint.map(|hint| div().text_size(px(11.)).font_weight(FontWeight::NORMAL).text_color(rgb(theme::TEXT_MUTED)).child(hint.to_owned())))
    }

    fn render_cloud_config(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(CloudDialog { body: Body::Config(form), busy, .. }) = &self.cloud.dialog else { return vec![] };
        let token_hint = if form.has_token { "Токен сохранён. Поле — только для замены." } else { "Токен ещё не сохранён." };
        let passphrase_hint = if form.has_passphrase { SAVED_SECRET } else { "Пароль ещё не сохранён." };
        let basic_hint = form.has_basic_password.then_some(SAVED_SECRET);
        // The legacy Settings window holds this switch; the native client has no Settings yet.
        let auto_sync = configured(&self.vault).then(|| self.vault.cloud.as_ref().is_some_and(|c| c.auto_sync));
        let basic = form.basic;
        let disabled = *busy;
        vec![
            self.cloud_input(Field::Url, "Адрес сервера", &form.url, false, "https://cloud.example/v1/snapshot", None, true, cx).into_any_element(),
            self.cloud_input(Field::Token, "Токен доступа", &form.token, true, "Введите токен сервера", Some(token_hint), true, cx).into_any_element(),
            self.cloud_input(Field::Passphrase, "Пароль шифрования", &form.passphrase, true, "Не менее 12 символов", Some(passphrase_hint), true, cx).into_any_element(),
            div()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .rounded(px(6.))
                .border_1()
                .border_color(rgb(0x3a4050))
                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(rgb(theme::TEXT)).child("HTTP Basic auth"))
                .child(
                    super::dialog::checkbox("cloud-basic", "Сервер за прокси с Basic auth", basic).text_xs().on_click(cx.listener(move |shell, _, _, cx| {
                        if disabled {
                            return;
                        }
                        if let Some(CloudDialog { body: Body::Config(form), .. }) = &mut shell.cloud.dialog {
                            form.basic = !form.basic;
                            if !form.basic && matches!(form.field, Some(Field::BasicUser | Field::BasicPassword)) {
                                form.field = Some(Field::Url);
                            }
                        }
                        cx.notify();
                    })),
                )
                .child(self.cloud_input(Field::BasicUser, "Логин", &form.basic_user, false, "", None, basic, cx))
                .child(self.cloud_input(Field::BasicPassword, "Пароль", &form.basic_password, true, "", basic_hint, basic, cx))
                .into_any_element(),
            div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(HELP).into_any_element(),
        ]
        .into_iter()
        .chain(auto_sync.map(|on| {
            super::dialog::checkbox("cloud-auto-sync", AUTO_SYNC, on)
                .text_xs()
                .on_click(cx.listener(move |shell, _, _, cx| {
                    if disabled {
                        return;
                    }
                    if let Some(cloud) = &mut shell.vault.cloud {
                        cloud.auto_sync = !cloud.auto_sync;
                    }
                    shell.save_vault();
                    cx.notify();
                }))
                .into_any_element()
        }))
        .collect()
    }

    fn render_cloud_panel(&self, run: &Run, cx: &mut Context<Self>) -> Option<Div> {
        let status = run.panel.as_ref()?;
        let (headline, tone) = match status {
            Status::Checking => ("Проверяю изменения…", theme::TEXT),
            Status::Syncing => ("Синхронизация…", theme::TEXT),
            Status::Error { .. } => ("Не удалось проверить состояние синхронизации", ERROR_TONE),
            Status::Checked(preview) => (
                preview.headline(),
                match preview {
                    Preview::Synced => self_cloud::TONE_OK,
                    Preview::Conflicts(_) => ERROR_TONE,
                    Preview::Changes(_) => match preview.counts() {
                        (_, 0) => UPLOAD_TONE,
                        (0, _) => DOWNLOAD_TONE,
                        _ => ERROR_TONE,
                    },
                    _ => theme::TEXT,
                },
            ),
        };
        let mut panel = div()
            .id("cloud-preview")
            .p(px(12.))
            .max_h(px(330.))
            .overflow_y_scroll()
            .rounded(px(8.))
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(PANEL))
            .text_xs()
            .text_color(rgb(theme::TEXT))
            .child(div().text_size(px(13.)).font_weight(FontWeight::BOLD).text_color(rgb(tone)).child(headline));
        if run.operation == Operation::Sync && matches!(status, Status::Checked(Preview::Conflicts(_))) {
            panel = panel.child(muted("Синхронизация разрешит конфликты автоматически. Если запись изменена на обеих сторонах, сохранится версия из облака. Если запись изменена на одной стороне и удалена на другой, сохранится изменённая версия."));
        }
        if let Status::Error { message, credentials } = status {
            panel = panel.child(muted(message.clone()));
            if *credentials {
                panel = panel.child(muted("Автоматическая проверка приостановлена, чтобы не продлевать блокировку. Исправьте токен или пароль в настройках своего облака."));
            }
        }
        if let Status::Checked(preview) = status {
            let groups = preview.groups();
            if !groups.is_empty() {
                panel = panel.child(div().mt(px(12.)).text_size(px(11.)).text_color(rgb(theme::TEXT_MUTED)).child(self_cloud::overview(&groups)));
                let cards = groups.into_iter().enumerate().map(|(index, group)| {
                    let open = (group.total <= 8) != run.toggled.contains(&index);
                    let tone = match group.direction {
                        "На устройстве" => UPLOAD_TONE,
                        "В облаке" => DOWNLOAD_TONE,
                        _ => ERROR_TONE,
                    };
                    let title = group.title();
                    let contents = group.categories.into_iter().map(|category| {
                        let label = if category.records.len() > 1 { format!("{} ×{}", category.label, category.records.len()) } else { category.label.to_owned() };
                        let with_details = category.records.iter().any(|(_, detail)| !detail.is_empty());
                        let names: AnyElement = if with_details {
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .children(category.records.into_iter().map(|(name, detail)| {
                                    div().child(format!("«{name}»")).when(!detail.is_empty(), |d| d.child(div().mt(px(2.)).text_size(px(11.)).text_color(rgb(theme::TEXT_MUTED)).child(detail)))
                                }))
                                .into_any_element()
                        } else {
                            div().child(category.records.iter().map(|(name, _)| format!("«{name}»")).collect::<Vec<_>>().join(", ")).into_any_element()
                        };
                        div().flex().gap(px(8.)).child(div().w(px(150.)).flex_none().text_size(px(11.)).text_color(rgb(theme::TEXT_MUTED)).child(label)).child(div().flex_1().min_w_0().child(names))
                    });
                    div()
                        .rounded(px(7.))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(GROUP))
                        .child(
                            div()
                                .id(SharedString::from(format!("cloud-group-{index}")))
                                .min_h(px(37.))
                                .px(px(11.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(div().font_weight(FontWeight::SEMIBOLD).text_color(rgb(tone)).child(title))
                                .child(div().flex_1())
                                .child(div().text_size(px(11.)).text_color(rgb(theme::TEXT_MUTED)).child(group.direction))
                                .child(svg().path(icon("caret_down")).size(px(10.)).text_color(rgb(theme::TEXT_MUTED)).when(open, |s| s.with_transformation(gpui::Transformation::rotate(gpui::percentage(0.5)))))
                                .on_click(cx.listener(move |shell, _, _, cx| {
                                    if let Some(CloudDialog { body: Body::Run(run), .. }) = &mut shell.cloud.dialog {
                                        if !run.toggled.remove(&index) {
                                            run.toggled.insert(index);
                                        }
                                    }
                                    cx.notify();
                                })),
                        )
                        .when(open, |d| d.child(div().px(px(11.)).pt(px(10.)).pb(px(11.)).flex().flex_col().gap(px(8.)).border_t_1().border_color(rgb(BORDER)).children(contents)))
                });
                panel = panel.child(div().mt(px(8.)).mb(px(4.)).flex().flex_col().gap(px(8.)).children(cards));
            }
        }
        if let Some(at) = run.checked_at {
            panel = panel.child(div().mt(px(8.)).opacity(0.7).child(format!("Проверено: {}", clock(at))));
        }
        Some(div().child(panel))
    }

    pub fn render_cloud(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let dialog = self.cloud.dialog.as_ref()?;
        let busy = dialog.busy;
        let (title, fields, actions): (&str, Vec<AnyElement>, Vec<AnyElement>) = match &dialog.body {
            Body::Config(_) => (
                "Своё облако",
                self.render_cloud_config(cx),
                vec![button("cloud-save", if self.vault.cloud.is_some() && matches!(&dialog.body, Body::Config(form) if !form.passphrase.is_empty()) {
                    "Сменить пароль"
                } else {
                    "Сохранить настройки"
                }, true, busy).on_click(cx.listener(|shell, _, window, cx| shell.save_cloud_config(window, cx))).into_any_element()],
            ),
            Body::Run(run) => {
                let mut fields: Vec<AnyElement> = self.render_cloud_panel(run, cx).into_iter().map(IntoElement::into_any_element).collect();
                if run.panel.is_some() {
                    fields.push(notice(run.operation.warning()).into_any_element());
                }
                let mut actions = Vec::new();
                if !run.configured {
                    actions.push(
                        button("cloud-configure", "Настроить своё облако", true, busy)
                            .on_click(cx.listener(|shell, _, window, cx| {
                                shell.close_cloud(window, cx);
                                shell.open_cloud(None, window, cx);
                            }))
                            .into_any_element(),
                    );
                }
                if run.confirm {
                    actions.push(button("cloud-confirm", run.operation.confirm(), true, busy).on_click(cx.listener(|shell, _, _, cx| shell.confirm_cloud(cx))).into_any_element());
                }
                (run.operation.title(), fields, actions)
            }
        };
        let out = dialog.out.clone().map(|out| {
            let (text, tone) = match out {
                Out::Working(text) => (text, theme::TEXT_MUTED),
                Out::Success(text) => (text, self_cloud::TONE_OK),
                Out::Error(text) => (text, ERROR_TONE),
            };
            div().text_xs().text_color(rgb(tone)).child(text)
        });
        let _ = window;
        let card = div()
            .id("cloud-dialog")
            .w(px(520.))
            .max_w_full()
            .max_h(px(760.))
            .overflow_y_scroll()
            .rounded(px(12.))
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(theme::BACKGROUND))
            .shadow_lg()
            .text_color(rgb(theme::TEXT))
            .track_focus(&self.cloud_focus)
            .on_key_down(cx.listener(|shell, event, window, cx| shell.cloud_key(event, window, cx)))
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(px(24.))
                    .pt(px(24.))
                    .pb(px(18.))
                    .flex()
                    .items_start()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(div().size(px(38.)).flex_none().rounded(px(9.)).bg(rgb(theme::SURFACE_ACTIVE)).flex().items_center().justify_center().child(svg().path(icon("upload")).size(px(22.)).text_color(rgb(theme::ACCENT))))
                            .child(div().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child(title.to_owned())),
                    )
                    .child(
                        div()
                            .id("cloud-close-x")
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(5.))
                            .hover(|s| s.bg(rgb(theme::SURFACE_ACTIVE)))
                            .child(svg().path(icon("close")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                            .on_click(cx.listener(|shell, _, window, cx| shell.close_cloud(window, cx))),
                    ),
            )
            .child(div().px(px(24.)).pb(px(20.)).flex().flex_col().gap(px(16.)).children(fields).children(out))
            .child(
                div()
                    .px(px(24.))
                    .py(px(16.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .id("cloud-close")
                            .min_h(px(32.))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .rounded(px(6.))
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(theme::TEXT_MUTED))
                            .when(busy, |d| d.opacity(0.45))
                            .child("Закрыть")
                            .on_click(cx.listener(|shell, _, window, cx| shell.close_cloud(window, cx))),
                    )
                    .child(div().flex().gap(px(8.)).children(actions)),
            );
        Some(
            div()
                .id("cloud-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0x0000008a))
                .on_click(cx.listener(|shell, _, window, cx| shell.close_cloud(window, cx)))
                .child(card),
        )
    }
}

/// Changes on both sides, the legacy `conflict` headline.
#[cfg(test)]
mod tests {
    use super::*;
    use termian_native::cloud::CloudConfig;

    fn run(operation: Operation) -> Run {
        Run { operation, configured: true, panel: None, checked_at: None, confirm: false, toggled: HashSet::new() }
    }

    #[test]
    fn the_settings_form_never_shows_saved_secrets() {
        let vault = Vault {
            cloud: Some(CloudConfig { url: "https://sync.example".into(), token: "secret-token".into(), passphrase: "a long passphrase".into(), basic_user: "me".into(), basic_password: "pw".into(), auto_sync: true }),
            ..Vault::default()
        };
        let mut form = ConfigForm::from_vault(&vault);
        assert_eq!((form.url.as_str(), form.token.as_str(), form.passphrase.as_str(), form.basic_password.as_str()), ("https://sync.example", "", "", ""));
        assert!(form.has_token && form.has_passphrase && form.has_basic_password && form.basic);
        assert_eq!(form.missing(), None, "saved secrets satisfy the required fields");
        form.passphrase = "short".into();
        assert_eq!(form.missing(), Some("Пароль шифрования должен содержать не менее 12 символов"));
        form.passphrase.clear();
        form.basic = false;
        assert_eq!(form.changes().basic_user, "", "turning Basic auth off clears it");
        assert!(form.value(Field::BasicUser).is_none(), "disabled fields take no input");
        let mut fresh = ConfigForm::from_vault(&Vault::default());
        fresh.url = "https://sync.example".into();
        assert_eq!(fresh.missing(), Some("Введите токен доступа"));
    }

    #[test]
    fn tab_moves_through_the_enabled_fields() {
        let mut form = ConfigForm { field: Some(Field::Passphrase), ..ConfigForm::default() };
        form.next(false);
        assert_eq!(form.field, Some(Field::Url));
        form.basic = true;
        form.next(true);
        assert_eq!(form.field, Some(Field::BasicPassword));
    }

    #[test]
    fn conflicting_preview_can_run_without_choosing_a_side() {
        let mut run = run(Operation::Sync);
        run.checked(Status::Checked(Preview::Conflicts(vec![])), 0);
        assert!(run.confirm);
        run.confirm = false;
        assert_eq!(run.finished(&Ok("Готово".into())), Out::Success("Готово".into()));
        assert_eq!(run.panel, Some(Status::Checked(Preview::Synced)));
        assert_eq!(run.finished(&Err("сбой".into())), Out::Error("сбой".into()));
        assert!(run.confirm);
    }

    #[test]
    fn adopting_a_synced_copy_keeps_live_history_and_edits() {
        let started = Vault::default();
        let mut live = Vault { history: vec![termian_native::vault::HistoryEntry { id: 7, ..Default::default() }], ..Vault::default() };
        let synced = Vault { hosts: termian_native::vault::demo().hosts, ..Vault::default() };
        assert!(adopt(&mut live, &started, synced.clone()));
        assert_eq!(live.history[0].id, 7);
        assert!(!live.hosts.is_empty());
        let mut edited = Vault { groups: termian_native::vault::demo().groups, ..Vault::default() };
        assert!(!adopt(&mut edited, &started, synced), "an edit made meanwhile is never overwritten");
        assert!(edited.hosts.is_empty() && !edited.groups.is_empty());
    }
}
