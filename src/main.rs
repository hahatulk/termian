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
use termian_native::theme;
use termian_native::vault::{GroupId, Vault};
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
    vault_error: Option<String>,
    section: Section,
    group: Option<GroupId>,
    terminals: HashMap<TabId, Entity<TerminalView>>,
    query: String,
    search_focus: FocusHandle,
    sftp: ui::sftp::SftpState,
    sftp_filter_focus: FocusHandle,
    sftp_answer_focus: FocusHandle,
    sftp_dialog_focus: FocusHandle,
}

/// Reads hosts from the legacy profile without modifying it.
/// `TERMIAN_LEGACY_INDEXEDDB` overrides the profile location; `TERMIAN_DEMO`
/// shows sample data instead.
fn load_legacy_vault() -> Result<Vault, String> {
    if std::env::var_os("TERMIAN_DEMO").is_some() {
        return Ok(termian_native::vault::demo());
    }
    let dir = std::env::var_os("TERMIAN_LEGACY_INDEXEDDB")
        .map(Into::into)
        .or_else(profile::default_indexeddb_dir)
        .ok_or("legacy profile not found")?;
    let cipher = FieldCipher::from_local_key(&profile::read_local_key()?)?;
    profile::load_vault(&dir, &cipher)
}

impl Shell {
    fn new(cx: &mut Context<Self>) -> Self {
        let (vault, vault_error) = match load_legacy_vault() {
            Ok(vault) => (vault, None),
            Err(error) => (Vault::default(), Some(error)),
        };
        Self { tabs: Tabs::default(), vault, vault_error, section: Section::Hosts,
            group: None,
            terminals: HashMap::new(),
            query: String::new(),
            search_focus: cx.focus_handle(),
            sftp: ui::sftp::SftpState::new(),
            sftp_filter_focus: cx.focus_handle(),
            sftp_answer_focus: cx.focus_handle(),
            sftp_dialog_focus: cx.focus_handle(),
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
                    shell.tabs.close(id);
                    shell.terminals.remove(&id);
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
    fn add_terminal(&mut self, title: String, view: Entity<TerminalView>, cx: &mut Context<Self>) {
        let id = self.tabs.open(TabKind::Session { title });
        cx.subscribe(&view, move |shell, _, event: &TerminalEvent, cx| match event {
            TerminalEvent::Close => {
                shell.tabs.close(id);
                shell.terminals.remove(&id);
                cx.notify();
            }
        })
        .detach();
        self.terminals.insert(id, view);
        cx.notify();
    }

    pub fn open_terminal(&mut self, title: String, session: session::Session, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.new(|cx| TerminalView::new(session, window, cx));
        self.add_terminal(title, view, cx);
    }

    /// Opens an SSH tab that shows the connection screen until the shell is ready.
    pub fn open_ssh(&mut self, label: String, os: Option<String>, target: SshTarget, window: &mut Window, cx: &mut Context<Self>) {
        let connection = Connection { label: label.clone(), os, target };
        let view = cx.new(|cx| TerminalView::ssh(connection, window, cx));
        self.add_terminal(label, view, cx);
    }

    pub fn connect_host(&mut self, host: HostId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(host) = self.vault.hosts.iter().find(|h| h.id == host) else { return };
        let target = SshTarget::new(&host.address, host.username.clone());
        let (label, os) = (host.label.clone(), host.os.clone());
        self.open_ssh(label, os, target, window, cx);
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
        match session::local_shell(80, 24) {
            Ok(session) => self.open_terminal("Local".into(), session, window, cx),
            Err(error) => self.vault_error = Some(format!("Could not start a local shell: {error}")),
        }
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs: Vec<_> = self.tabs.all().iter().map(|tab| self.render_tab(tab, cx)).collect();
        let toolbar = ["sync", "upload", "download"]
            .map(|name| div().px_2().child(svg().path(icon(name)).size(px(16.)).text_color(rgb(theme::TEXT))));
        let content = match self.tabs.active().kind {
            TabKind::Vaults => self.render_vaults(window, cx),
            TabKind::Sftp => self.render_sftp(window, cx).into_any_element(),
            TabKind::Session { .. } => match self.terminals.get(&self.tabs.active().id) {
                Some(view) => view.clone().into_any_element(),
                None => div().into_any_element(),
            },
            _ => div().into_any_element(),
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
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.tabs.open(TabKind::Launcher);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .children(toolbar),
            )
            .child(div().flex_1().min_h_0().child(content))
            .when(matches!(self.tabs.active().kind, TabKind::Sftp), |d| d.children(self.render_sftp_overlay(window, cx)))
    }
}

fn main() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
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
            |_, cx| cx.new(Shell::new),
        )
        .expect("failed to open the main window");
        cx.activate(true);
    });
}
