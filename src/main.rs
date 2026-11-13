mod ui;

use gpui::{
    div, prelude::*, px, rgb, size, svg, App, Application, Bounds, Context, Div, Entity, FocusHandle, SharedString, Stateful,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use std::collections::HashMap;
use termian_native::session::{self, SshTarget};
use termian_native::tabs::TabId;
use termian_native::vault::HostId;
use ui::terminal_view::{Connection, TerminalEvent, TerminalView};
use termian_native::legacy::{cipher::FieldCipher, profile};
use termian_native::tabs::{Tab, TabKind, Tabs};
use termian_native::store::Store;
use termian_native::theme;
use termian_native::session_log;
use termian_native::vault::{GroupId, HistoryEntry, LogRef, Vault};

/// Seconds since the Unix epoch.
fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
use ui::assets::{icon, Assets};
use ui::hosts::Section;

/// Room left of the tab strip for the macOS traffic-light buttons.
#[cfg(target_os = "macos")]
const WINDOW_CONTROLS_INSET: f32 = 80.;
#[cfg(not(target_os = "macos"))]
const WINDOW_CONTROLS_INSET: f32 = 12.;

pub struct Shell {
    tabs: Tabs,
    vault: Vault,
    store: Option<Store>,
    /// Encrypted session logs (`session-logs-v2`).
    logs_dir: std::path::PathBuf,
    vault_error: Option<String>,
    host_editor: Option<ui::host_editor::HostEditor>,
    host_menu: Option<ui::host_editor::HostMenu>,
    key_editor: Option<ui::key_editor::KeyEditor>,
    identity_editor: Option<ui::identity_editor::IdentityEditor>,
    keychain_menu: Option<gpui::Point<gpui::Pixels>>,
    group_editor: Option<ui::group_editor::GroupEditor>,
    group_menu: Option<ui::group_editor::GroupMenu>,
    launcher: ui::launcher::Launcher,
    logs_view: ui::logs::LogsView,
    rules: ui::forwarding::Rules,
    forward_focus: FocusHandle,
    launcher_focus: FocusHandle,
    palette: Option<ui::palette::Palette>,
    palette_focus: FocusHandle,
    /// Holds the keyboard when nothing else does, so shortcuts like ⌘K work.
    root_focus: FocusHandle,
    editor_focus: FocusHandle,
    section: Section,
    group: Option<GroupId>,
    terminals: HashMap<TabId, Entity<TerminalView>>,
    /// The history entry (and log) behind each session tab.
    tab_history: HashMap<TabId, u64>,
    query: String,
    search_focus: FocusHandle,
    sftp: ui::sftp::SftpState,
    sftp_filter_focus: FocusHandle,
    sftp_answer_focus: FocusHandle,
    sftp_dialog_focus: FocusHandle,
    cloud: ui::cloud::CloudState,
    update: ui::update::UpdateState,
    main_window: Option<gpui::WindowHandle<Shell>>,
    /// Bundled terminal fonts already given to the text system.
    loaded_fonts: std::collections::HashSet<&'static str>,
    settings_window: Option<gpui::WindowHandle<ui::settings::SettingsWindow>>,
    cloud_focus: FocusHandle,
}

/// Reads hosts from the legacy profile without modifying it.
/// `TERMIAN_LEGACY_INDEXEDDB` overrides the profile location.
fn load_legacy_vault() -> Result<Vault, String> {
    let dir = std::env::var_os("TERMIAN_LEGACY_INDEXEDDB")
        .map(Into::into)
        .or_else(profile::default_indexeddb_dir)
        .ok_or("legacy profile not found")?;
    let cipher = FieldCipher::from_local_key(&profile::read_local_key()?)?;
    profile::load_vault(&dir, &cipher)
}

/// The native vault (ADR 0002), imported from the legacy profile on first run.
/// `TERMIAN_DEMO` uses a throwaway demo vault and never touches the keychain.
fn open_vault() -> (Vault, Option<Store>, Option<String>) {
    let (store, import): (Result<Store, String>, fn() -> Result<Vault, String>) = if std::env::var_os("TERMIAN_DEMO").is_some() {
        (Ok(Store::demo()), || Ok(termian_native::vault::demo()))
    } else {
        (Store::open_default(), load_legacy_vault)
    };
    match store {
        Ok(store) => match store.load_or_import(import) {
            Ok(vault) => (vault, Some(store), None),
            Err(error) => (Vault::default(), None, Some(error)),
        },
        // Without a store the legacy hosts are still shown, read-only.
        Err(error) => (import().unwrap_or_default(), None, Some(format!("changes cannot be saved: {error}"))),
    }
}

impl Shell {
    fn new(cx: &mut Context<Self>) -> Self {
        let (vault, store, vault_error) = open_vault();
        let logs_dir = if std::env::var_os("TERMIAN_DEMO").is_some() {
            std::env::temp_dir().join("termian-native-demo").join("session-logs-v2")
        } else {
            session_log::default_dir()
        };
        // Logs older than 60 days are deleted at start (FR-L5).
        session_log::prune(&logs_dir, std::time::SystemTime::now(), session_log::RETENTION);
        Self { tabs: Tabs::default(), vault, store, logs_dir, vault_error, section: Section::Hosts,
            host_editor: None,
            host_menu: None,
            key_editor: None,
            identity_editor: None,
            keychain_menu: None,
            group_editor: None,
            group_menu: None,
            launcher: Default::default(),
            logs_view: Default::default(),
            rules: Default::default(),
            forward_focus: cx.focus_handle(),
            launcher_focus: cx.focus_handle(),
            palette: None,
            palette_focus: cx.focus_handle(),
            root_focus: cx.focus_handle(),
            editor_focus: cx.focus_handle(),
            group: None,
            terminals: HashMap::new(),
            tab_history: HashMap::new(),
            query: String::new(),
            search_focus: cx.focus_handle(),
            sftp: ui::sftp::SftpState::new(),
            sftp_filter_focus: cx.focus_handle(),
            sftp_answer_focus: cx.focus_handle(),
            sftp_dialog_focus: cx.focus_handle(),
            cloud: Default::default(),
            update: Default::default(),
            main_window: None,
            loaded_fonts: Default::default(),
            settings_window: None,
            cloud_focus: cx.focus_handle(),
        }
    }

    fn render_tab(&self, tab: &Tab, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = tab.id;
        let active = id == self.tabs.active().id;
        let icon_name = match tab.kind {
            TabKind::Vaults => "keychain",
            TabKind::Sftp => "sftp",
            TabKind::Session { .. } => "terminal",
            TabKind::Launcher => "plus",
        };
        let leading = if tab.closable() {
            div()
                .id(SharedString::from(format!("close-{id:?}")))
                .child(svg().path(icon("close")).size(px(12.)).text_color(rgb(theme::TEXT)))
                .on_click(cx.listener(move |shell, _, _, cx| {
                    cx.stop_propagation();
                    shell.close_terminal_tab(id);
                    cx.notify();
                }))
        } else {
            div().id(SharedString::from(format!("icon-{id:?}"))).child(svg().path(icon(icon_name)).size(px(14.)).text_color(rgb(theme::TEXT)))
        };
        div()
            .id(SharedString::from(format!("tab-{id:?}")))
            .h(px(30.))
            .when(tab.closable(), |s| s.w(px(230.)))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .rounded_md()
            .text_sm()
            .text_color(rgb(theme::TEXT))
            .bg(rgb(if active { theme::SURFACE_ACTIVE } else { theme::SURFACE }))
            .on_click(cx.listener(move |shell, _, window, cx| {
                shell.tabs.activate(id);
                if let Some(view) = shell.terminals.get(&id) {
                    view.read(cx).focus(window);
                }
                cx.notify();
            }))
            .child(leading)
            .child(div().truncate().child(SharedString::from(tab.title().to_owned())))
    }
}

impl Shell {
    fn add_terminal(&mut self, title: String, view: Entity<TerminalView>, history: Option<u64>, cx: &mut Context<Self>) {
        let id = self.tabs.open(TabKind::Session { title });
        cx.subscribe(&view, move |shell, _, event: &TerminalEvent, cx| {
            if let TerminalEvent::Os(os) = event {
                shell.host_os_detected(history, os);
                return;
            }
            shell.session_ended(history);
            if let TerminalEvent::Close = event {
                shell.close_terminal_tab(id);
            }
            cx.notify();
        })
        .detach();
        if let Some(history) = history {
            self.tab_history.insert(id, history);
        }
        self.terminals.insert(id, view);
        cx.notify();
    }

    /// «Detect OS»: a saved host without a system gets the one its server reported.
    fn host_os_detected(&mut self, history: Option<u64>, os: &str) {
        let host = history.and_then(|h| self.vault.history.iter().find(|e| e.id == h)).and_then(|e| e.host);
        if let Some(host) = host.and_then(|id| self.vault.hosts.iter_mut().find(|h| h.id == id)).filter(|h| h.os.is_none()) {
            host.os = Some(os.to_owned());
            self.save_vault();
        }
    }

    /// Writes a session's end time into its Logs row, once.
    fn session_ended(&mut self, history: Option<u64>) {
        if let Some(entry) = history.and_then(|h| self.vault.history_entry_mut(h)).filter(|e| e.disconnected_at.is_none()) {
            entry.disconnected_at = Some(now());
            self.save_vault();
        }
    }

    /// Closes a tab; dropping its terminal ends the session and finishes its log.
    fn close_terminal_tab(&mut self, id: TabId) {
        let history = self.tab_history.remove(&id);
        self.session_ended(history);
        self.tabs.close(id);
        self.terminals.remove(&id);
    }

    /// Adds a history entry with a fresh encrypted session log (FR-L1, FR-L2).
    fn record_session(&mut self, host: Option<HostId>, label: &str, subtitle: String) -> (u64, Option<session_log::SessionLog>) {
        let key = session_log::new_key();
        let name = format!("{}-{}", now(), self.vault.history.iter().map(|e| e.id).max().unwrap_or(0) + 1);
        let log = if self.vault.logs_disabled { None } else { session_log::SessionLog::start(&self.logs_dir, &name, key).ok() };
        let entry = HistoryEntry {
            host,
            connected_at: now(),
            label: label.to_owned(),
            subtitle,
            log: log.as_ref().map(|_| LogRef { name, key: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key) }),
            ..HistoryEntry::default()
        };
        let id = self.vault.record_connection(entry);
        self.save_vault();
        (id, log)
    }

    pub fn open_terminal(&mut self, title: String, session: session::Session, window: &mut Window, cx: &mut Context<Self>) {
        let (history, log) = self.record_session(None, &title, "local".into());
        let settings = self.vault.terminal.clone();
        let view = cx.new(|cx| TerminalView::new(session, &settings, window, cx).with_log(log));
        self.add_terminal(title, view, Some(history), cx);
    }

    /// Opens an SSH tab that shows the connection screen until the shell is ready.
    pub fn open_ssh(&mut self, label: String, os: Option<String>, target: SshTarget, window: &mut Window, cx: &mut Context<Self>) {
        self.open_ssh_for(None, label, os, target, window, cx);
    }

    fn open_ssh_for(&mut self, host: Option<HostId>, label: String, os: Option<String>, mut target: SshTarget, window: &mut Window, cx: &mut Context<Self>) {
        target.options = session::SshOptions::from_vault(&self.vault);
        // Legacy runs the detection only for saved hosts whose system is unknown.
        let unknown = host.and_then(|id| self.vault.hosts.iter().find(|h| h.id == id)).is_some_and(|h| h.os.is_none());
        target.options.detect_os = self.vault.terminal.detect_os && unknown;
        let subtitle = std::iter::once("ssh").chain(target.username.as_deref()).collect::<Vec<_>>().join(", ");
        let (history, log) = self.record_session(host, &label, subtitle);
        let connection = Connection { label: label.clone(), os, target };
        let settings = self.vault.terminal.clone();
        let view = cx.new(|cx| TerminalView::ssh(connection, &settings, window, cx).with_log(log));
        self.add_terminal(label, view, Some(history), cx);
    }

    pub fn connect_host(&mut self, host: HostId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.vault.hosts.iter().find(|h| h.id == host) else { return };
        let target = SshTarget::for_host(entry, &self.vault);
        let (label, os) = (entry.display_label().to_owned(), entry.os.clone());
        self.open_ssh_for(Some(host), label, os, target, window, cx);
    }

    /// Opens a session log read-only in a new tab (FR-L4).
    pub fn open_log(&mut self, history: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.vault.history.iter().find(|e| e.id == history) else { return };
        let Some(log) = &entry.log else { return };
        let key: Option<[u8; 32]> = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &log.key).ok().and_then(|k| k.try_into().ok());
        let (data, note) = match key.map(|key| session_log::read(&self.logs_dir, &log.name, &key)) {
            Some(Ok((data, error))) => (data, error.map(str::to_owned)),
            Some(Err(error)) => (Vec::new(), Some(error)),
            None => (Vec::new(), Some(session_log::UNREADABLE_LOG.to_owned())),
        };
        let title = entry.label.clone();
        let settings = self.vault.terminal.clone();
        let view = cx.new(|cx| TerminalView::replay(data, note.as_deref(), &settings, window, cx));
        self.add_terminal(title, view, None, cx);
    }

    /// Runs a snippet in the most recently opened terminal, starting a local one if none is open.
    pub fn run_in_terminal(&mut self, script: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.terminals.is_empty() {
            self.open_local_terminal(window, cx);
        }
        let Some((&id, view)) = self.terminals.iter().max_by_key(|(id, _)| **id) else { return };
        let view = view.clone();
        self.tabs.activate(id);
        view.update(cx, |view, _| view.run(script));
        view.read(cx).focus(window);
        cx.notify();
    }

    pub fn open_local_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = &self.vault.terminal;
        match session::local_shell(terminal.local_shell.as_deref(), terminal.emulation(), 80, 24) {
            Ok(session) => self.open_terminal("Local".into(), session, window, cx),
            Err(error) => self.vault_error = Some(format!("Could not start a local shell: {error}")),
        }
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs: Vec<_> = self.tabs.all().iter().map(|tab| self.render_tab(tab, cx)).collect();
        let toolbar = self.render_cloud_buttons(cx);
        let content = match self.tabs.active().kind {
            TabKind::Vaults => self.render_vaults(window, cx),
            TabKind::Sftp => self.render_sftp(window, cx).into_any_element(),
            TabKind::Session { .. } => match self.terminals.get(&self.tabs.active().id) {
                Some(view) => view.clone().into_any_element(),
                None => div().into_any_element(),
            },
            TabKind::Launcher => self.render_launcher(window, cx),
        };

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(theme::BACKGROUND))
            .child(
                div()
                    .h(px(44.))
                    .flex_none()
                    .pl(px(WINDOW_CONTROLS_INSET))
                    .pr_3()
                    .flex()
                    .items_center()
                    .gap_1()
                    .bg(rgb(theme::TITLE_BAR))
                    .children(tabs)
                    .child(
                        div()
                            .id("new-tab")
                            .px_2()
                            .child(svg().path(icon("plus")).size(px(14.)).text_color(rgb(theme::TEXT_MUTED)))
                            .on_click(cx.listener(|shell, _, window, cx| {
                                shell.tabs.open(TabKind::Launcher);
                                window.focus(&shell.launcher_focus);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .children(toolbar),
            )
            .child(div().flex_1().min_h_0().child(content))
            .when(matches!(self.tabs.active().kind, TabKind::Sftp), |d| d.children(self.render_sftp_overlay(window, cx)))
            .when(matches!(self.tabs.active().kind, TabKind::Vaults), |d| {
                d.children(self.render_host_menu(window, cx)).children(self.render_group_menu(window, cx))
            })
            .track_focus(&self.root_focus)
            .children(self.render_palette(window, cx))
            .children(self.render_update_notice(cx))
            .children(self.render_cloud(window, cx))
            .on_action(cx.listener(|shell, _: &ui::palette::OpenPalette, window, cx| shell.open_palette(String::new(), window, cx)))
            .map(|root| self.on_shortcut_actions(root, cx))
    }
}

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1440.), px(860.)), cx);
        let main = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Termian".into()),
                    appears_transparent: cfg!(target_os = "macos"),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let shell = cx.new(Shell::new);
                shell.update(cx, |shell, cx| {
                    shell.start_cloud(cx);
                    shell.start_updates(cx);
                    shell.load_terminal_font(cx);
                });
                let focus = shell.read(cx).root_focus.clone();
                window.focus(&focus);
                shell
            },
        )
        .expect("failed to open the main window");
        let _ = main.update(cx, |shell, _, _| shell.main_window = Some(main));
        ui::settings::install_menus(main, cx);
        if let Ok(settings) = main.read(cx).map(|shell| shell.vault.shortcuts.clone()) {
            ui::shortcut_actions::bind(&settings, cx);
        }
        cx.activate(true);
    });
}
