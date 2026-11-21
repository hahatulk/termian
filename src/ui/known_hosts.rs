//! Known Hosts as in legacy: the host keys this client trusts (its
//! `known_hosts` file) plus those read from the legacy profile, «Import»
//! from `~/.ssh/known_hosts` (or a chosen file), and Remove from the card's
//! right-click menu after «Remove known host».

use termian_native::language::text as tr;
use gpui::{div, prelude::*, px, AnyElement, Context, Div, MouseButton, MouseDownEvent, PathPromptOptions, Pixels, Point, Window};
use termian_native::known_hosts::{self, Entry};
use termian_native::theme;

use super::dialog::{self, Kind};
use super::host_editor::{menu_panel, menu_row};
use super::hosts::{card, section_title, tile, tool_button};
use super::sections::{empty_state, page};
use crate::Shell;

#[derive(Clone)]
enum Item {
    /// A line of the native `known_hosts` file.
    File(Entry),
    /// A legacy-profile record (no key was imported).
    Legacy(u64),
}

pub struct KnownMenu {
    item: Item,
    position: Point<Pixels>,
    confirm_remove: bool,
}

impl Shell {
    pub fn render_known_hosts(&self, window: &gpui::Window, cx: &mut Context<Self>) -> Div {
        let file = known_hosts::read(&self.store.known_hosts_path());
        let legacy = &self.vault.known_hosts;
        let body = if file.is_empty() && legacy.is_empty() {
            empty_state("known_hosts", tr("No known hosts"), tr("Host keys you trust appear here after the first connection."))
        } else {
            // File entries have no ids: their place in the file stands for one, after the legacy ones.
            let base = legacy.iter().map(|e| e.id).max().unwrap_or(0) + 1;
            let file: Vec<(u64, _)> = file.into_iter().enumerate().map(|(i, e)| (base + i as u64, e)).collect();
            let file = self.listed("knownHosts", &file, |(_, e)| e.hostnames.as_str(), |(id, _)| *id);
            let legacy = self.listed("knownHosts", legacy, |e| e.host.as_str(), |e| e.id);
            let mut cards = Vec::new();
            for (i, entry) in file.into_iter().map(|(id, e)| (*id, e)) {
                let item = Item::File(entry.clone());
                let listener = cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.known_menu = Some(KnownMenu { item: item.clone(), position: event.position, confirm_remove: false });
                    cx.notify();
                });
                cards.push(card(format!("known-file-{i}").into(), tile("known_hosts", theme::TILE_BLUE), entry.hostnames.clone(), entry.key_type.clone()).on_mouse_down(MouseButton::Right, listener));
            }
            for entry in legacy {
                let item = Item::Legacy(entry.id);
                let listener = cx.listener(move |shell, event: &MouseDownEvent, _, cx| {
                    shell.known_menu = Some(KnownMenu { item: item.clone(), position: event.position, confirm_remove: false });
                    cx.notify();
                });
                cards.push(card(format!("known-{}", entry.id).into(), tile("known_hosts", theme::TILE_BLUE), entry.host.clone(), String::new()).on_mouse_down(MouseButton::Right, listener));
            }
            div().child(section_title(tr("Known Hosts"))).child(self.cards("knownHosts").children(cards))
        };
        let import = tool_button("import-known", "download", tr("IMPORT")).on_click(cx.listener(|shell, _, window, cx| shell.import_known_hosts(window, cx)));
        page("known-scroll", vec![import], self.listing_controls("knownHosts", true, false, window, cx), body)
    }

    /// Legacy reads `~/.ssh/known_hosts`, and asks for a file when there is none.
    fn import_known_hosts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let target = self.store.known_hosts_path();
        let ssh = dirs::home_dir().unwrap_or_default().join(".ssh").join("known_hosts");
        if ssh.is_file() {
            if let Err(error) = known_hosts::import(&ssh, &target) {
                self.vault_error = Some(format!("Could not import known hosts: {error}"));
            }
            cx.notify();
            return;
        }
        let answer = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some(tr("Import").into()) });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = answer.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let result = known_hosts::import(&path, &target);
            let _ = this.update_in(cx, |shell, _, cx| {
                if let Err(error) = result {
                    shell.vault_error = Some(format!("Could not import known hosts: {error}"));
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn remove_known_host(&mut self, item: &Item, cx: &mut Context<Self>) {
        match item {
            Item::File(entry) => {
                if let Err(error) = known_hosts::remove(&self.store.known_hosts_path(), std::slice::from_ref(entry)) {
                    self.vault_error = Some(format!("Could not remove the known host: {error}"));
                }
            }
            Item::Legacy(id) => {
                self.vault.known_hosts.retain(|k| k.id != *id);
                self.save_vault();
            }
        }
        self.known_menu = None;
        cx.notify();
    }

    pub fn render_known_menu(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.known_menu.as_ref()?;
        if menu.confirm_remove {
            let close = dialog::close_button("known-remove-close").on_click(cx.listener(|shell, _, _, cx| {
                shell.known_menu = None;
                cx.notify();
            }));
            let item = menu.item.clone();
            return Some(
                dialog::backdrop(dialog::card(tr("Remove known host"), false, close, dialog::text("Are you sure you want to remove 1 known host?"), vec![dialog::button("known-remove-confirm", tr("Remove"), Kind::Error, false)
                    .on_click(cx.listener(move |shell, _, _, cx| shell.remove_known_host(&item, cx)))
                    .into_any_element()]))
                .into_any_element(),
            );
        }
        let items = menu_panel("known-menu").child(menu_row("known-menu-remove".into(), tr("Remove"), true, false).on_click(cx.listener(|shell, _, _, cx| {
            if let Some(menu) = &mut shell.known_menu {
                menu.confirm_remove = true;
            }
            cx.notify();
        })));
        let viewport = window.viewport_size();
        let at = Point { x: menu.position.x.min(viewport.width - px(230.)), y: menu.position.y.min(viewport.height - px(60.)) };
        Some(
            gpui::deferred(
                div()
                    .id("known-menu-dismiss")
                    .absolute()
                    .inset_0()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, cx.listener(|shell, _, _, cx| {
                        shell.known_menu = None;
                        cx.notify();
                    }))
                    .child(gpui::anchored().position(at).child(super::motion::popover(items, "known-menu-enter"))),
            )
            .into_any_element(),
        )
    }
}
