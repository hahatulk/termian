//! The actions of Settings → Shortcuts and the key bindings made from the
//! current scheme. Terminal actions are bound in the `Terminal` context; the
//! rest act on the main window.

use gpui::{App, Context, KeyBinding, Window};
use termian_native::split_tree::Direction;
use termian_native::shortcuts::{self, ShortcutSettings};
use termian_native::tabs::TabKind;

use super::hosts::Section;
use super::palette::OpenPalette;
use super::settings::{OpenSettings, Quit};
use crate::Shell;

gpui::actions!(
    termian,
    [
        CopyFromTerminal,
        PasteToTerminal,
        SelectAllInTerminal,
        OpenHostsPage,
        OpenLocalTerminal,
        OpenSerialConnection,
        CloseTab,
        NextTab,
        PreviousTab,
        OpenPortForwarding,
        JumpTo,
        NewTab,
        ClearTerminalBuffer,
        OpenTerminalSearch,
        ToggleSidepanel,
        ToggleBroadcastMode,
        OpenSnippets,
        SwitchToTab1,
        SwitchToTab2,
        SwitchToTab3,
        SwitchToTab4,
        SwitchToTab5,
        SwitchToTab6,
        SwitchToTab7,
        SwitchToTab8,
        SwitchToTab9,
        FocusPaneUp,
        FocusPaneDown,
        FocusPaneLeft,
        FocusPaneRight,
        FocusWindowControls,
    ]
);

fn scheme_context(keys: &str, terminal: bool) -> Option<&'static str> {
    match (keys.split('-').any(|key| key == "ctrl"), terminal) {
        (true, true) => Some("Terminal && !raw_control"),
        (true, false) => Some("!raw_control"),
        (false, true) => Some("Terminal"),
        (false, false) => None,
    }
}

/// The binding of one legacy action type: the action and its key context.
fn binding(kind: &str, keys: &str) -> Option<KeyBinding> {
    let terminal = scheme_context(keys, true);
    let window = scheme_context(keys, false);
    Some(match kind {
        "copyFromTerminal" => KeyBinding::new(keys, CopyFromTerminal, terminal),
        "pasteToTerminal" => KeyBinding::new(keys, PasteToTerminal, terminal),
        "selectAllInTerminal" => KeyBinding::new(keys, SelectAllInTerminal, terminal),
        "clearTerminalBuffer" => KeyBinding::new(keys, ClearTerminalBuffer, terminal),
        "openTerminalSearch" => KeyBinding::new(keys, OpenTerminalSearch, terminal),
        "toggleSidepanel" => KeyBinding::new(keys, ToggleSidepanel, terminal),
        "terminals" => KeyBinding::new(keys, OpenHostsPage, window),
        "localTerminal" => KeyBinding::new(keys, OpenLocalTerminal, window),
        "serialConnection" => KeyBinding::new(keys, OpenSerialConnection, window),
        "close" => KeyBinding::new(keys, CloseTab, window),
        "bottom" => KeyBinding::new(keys, NextTab, window),
        "top" => KeyBinding::new(keys, PreviousTab, window),
        "forwarding" => KeyBinding::new(keys, OpenPortForwarding, window),
        "commandPalette" => KeyBinding::new(keys, OpenPalette, window),
        "jumpTo" => KeyBinding::new(keys, JumpTo, window),
        "newTab" => KeyBinding::new(keys, NewTab, window),
        "toggleSplitViewBroadcastMode" => KeyBinding::new(keys, ToggleBroadcastMode, window),
        "snippets" => KeyBinding::new(keys, OpenSnippets, window),
        _ => return None,
    })
}

fn tab_binding(keys: &str, number: usize) -> KeyBinding {
    let context = scheme_context(keys, false);
    match number {
        1 => KeyBinding::new(keys, SwitchToTab1, context),
        2 => KeyBinding::new(keys, SwitchToTab2, context),
        3 => KeyBinding::new(keys, SwitchToTab3, context),
        4 => KeyBinding::new(keys, SwitchToTab4, context),
        5 => KeyBinding::new(keys, SwitchToTab5, context),
        6 => KeyBinding::new(keys, SwitchToTab6, context),
        7 => KeyBinding::new(keys, SwitchToTab7, context),
        8 => KeyBinding::new(keys, SwitchToTab8, context),
        _ => KeyBinding::new(keys, SwitchToTab9, context),
    }
}

/// Replaces the key bindings with the app keys and the current scheme's.
pub fn bind(settings: &ShortcutSettings, cx: &mut App) {
    cx.clear_key_bindings();
    cx.bind_keys(bindings(settings));
}

fn bindings(settings: &ShortcutSettings) -> Vec<KeyBinding> {
    let mut bindings = vec![KeyBinding::new("f6", FocusWindowControls, Some("MainWindow && !Terminal"))];
    #[cfg(target_os = "macos")]
    bindings.extend([KeyBinding::new("cmd-,", OpenSettings, None), KeyBinding::new("cmd-q", Quit, None)]);
    #[cfg(not(target_os = "macos"))]
    bindings.push(KeyBinding::new("ctrl-,", OpenSettings, Some("!Terminal")));
    let Some((_, scheme)) = settings.current_scheme() else { return bindings };
    let tab_mod = shortcuts::modifier_keystroke(&scheme.tab_mod);
    if !tab_mod.is_empty() {
        bindings.extend((1..=9).map(|n| tab_binding(&format!("{tab_mod}-{n}"), n)));
    }
    // «Move focus between Split View panes»: the modifier with the arrows.
    let pane_mod = shortcuts::modifier_keystroke(&scheme.pane_mod);
    if !pane_mod.is_empty() {
        let context = scheme_context(&pane_mod, true);
        bindings.push(KeyBinding::new(&format!("{pane_mod}-up"), FocusPaneUp, context));
        bindings.push(KeyBinding::new(&format!("{pane_mod}-down"), FocusPaneDown, context));
        bindings.push(KeyBinding::new(&format!("{pane_mod}-left"), FocusPaneLeft, context));
        bindings.push(KeyBinding::new(&format!("{pane_mod}-right"), FocusPaneRight, context));
    }
    bindings.extend(scheme.shortcuts.iter().filter_map(|b| {
        let keys = shortcuts::keystroke(b.shortcut.as_deref()?)?;
        binding(&b.kind, &keys)
    }));
    bindings
}

impl Shell {
    fn show_section(&mut self, section: Section, window: &mut Window, cx: &mut Context<Self>) {
        let vaults = self.tabs.all()[0].id;
        self.tabs.activate(vaults);
        self.section = section;
        window.focus(&self.root_focus);
        cx.notify();
    }

    fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        let tab = self.tabs.active().clone();
        match tab.kind {
            TabKind::Session { .. } => self.close_terminal_tab(tab.id),
            _ => self.tabs.close(tab.id),
        }
        cx.notify();
    }

    fn switch_tab(&mut self, number: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.activate_nth(number);
        self.focus_active(window, cx);
    }

    fn cycle_tab(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.cloud_keyboard_scope().is_some() || self.palette.is_some() || self.list.menu.is_some() || self.app_menu.is_some() { return; }
        self.tabs.cycle(step);
        self.focus_active(window, cx);
    }

    /// A terminal tab takes the keyboard; other tabs give it to the window.
    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.focused_view(self.tabs.active().id) {
            Some(view) => view.read(cx).focus(window),
            None => window.focus(&self.root_focus),
        }
        cx.notify();
    }

    /// Settings → Shortcuts changed: saves and rebinds the keys.
    pub fn update_shortcuts(&mut self, change: impl FnOnce(&mut ShortcutSettings), cx: &mut Context<Self>) {
        match self.vaults.update_settings(&mut self.vault, |vault| change(&mut vault.shortcuts)) {
            Ok(()) => bind(&self.vault.shortcuts, cx),
            Err(error) => self.vault_error = Some(error),
        }
        cx.notify();
    }

    /// Main-window handlers of the scheme's actions.
    pub fn on_shortcut_actions(&self, root: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        use gpui::InteractiveElement as _;
        root.on_action(cx.listener(|shell, _: &OpenHostsPage, window, cx| shell.show_section(Section::Hosts, window, cx)))
            .on_action(cx.listener(|shell, _: &OpenPortForwarding, window, cx| shell.show_section(Section::PortForwarding, window, cx)))
            .on_action(cx.listener(|shell, _: &OpenSnippets, window, cx| shell.show_section(Section::Snippets, window, cx)))
            .on_action(cx.listener(|shell, _: &OpenLocalTerminal, window, cx| shell.open_local_terminal(window, cx)))
            .on_action(cx.listener(|shell, _: &OpenSerialConnection, window, cx| shell.open_serial(window, cx)))
            .on_action(cx.listener(|shell, _: &CloseTab, _, cx| shell.close_active_tab(cx)))
            .on_action(cx.listener(|shell, _: &NextTab, window, cx| shell.cycle_tab(1, window, cx)))
            .on_action(cx.listener(|shell, _: &PreviousTab, window, cx| shell.cycle_tab(-1, window, cx)))
            .on_action(cx.listener(|shell, _: &FocusWindowControls, window, _| {
                if shell.cloud_keyboard_scope().is_none() && shell.palette.is_none() && shell.list.menu.is_none() && shell.app_menu.is_none() {
                    window.focus(&shell.root_focus);
                    window.focus_next();
                }
            }))
            .on_action(cx.listener(|shell, _: &JumpTo, window, cx| shell.open_palette(String::new(), window, cx)))
            .on_action(cx.listener(|shell, _: &ToggleSidepanel, window, cx| shell.toggle_side_panel(window, cx)))
            .on_action(cx.listener(|shell, _: &ToggleBroadcastMode, _, cx| shell.toggle_broadcast(cx)))
            .on_action(cx.listener(|shell, _: &FocusPaneUp, window, cx| shell.focus_pane(Direction::Up, window, cx)))
            .on_action(cx.listener(|shell, _: &FocusPaneDown, window, cx| shell.focus_pane(Direction::Down, window, cx)))
            .on_action(cx.listener(|shell, _: &FocusPaneLeft, window, cx| shell.focus_pane(Direction::Left, window, cx)))
            .on_action(cx.listener(|shell, _: &FocusPaneRight, window, cx| shell.focus_pane(Direction::Right, window, cx)))
            .on_action(cx.listener(|shell, _: &NewTab, window, cx| {
                shell.tabs.open(TabKind::Launcher);
                window.focus(&shell.launcher_focus);
                cx.notify();
            }))
            .on_action(cx.listener(|shell, _: &SwitchToTab1, window, cx| shell.switch_tab(1, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab2, window, cx| shell.switch_tab(2, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab3, window, cx| shell.switch_tab(3, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab4, window, cx| shell.switch_tab(4, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab5, window, cx| shell.switch_tab(5, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab6, window, cx| shell.switch_tab(6, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab7, window, cx| shell.switch_tab(7, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab8, window, cx| shell.switch_tab(8, window, cx)))
            .on_action(cx.listener(|shell, _: &SwitchToTab9, window, cx| shell.switch_tab(9, window, cx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{KeyContext, Keymap, Keystroke};
    use std::any::TypeId;

    #[test]
    fn explicit_shortcuts_override_the_preset_and_pane_or_tab_modifiers() {
        let contexts = [KeyContext::parse("MainWindow").unwrap(), KeyContext::parse("Terminal").unwrap()];
        let unselected = [contexts[0].clone(), KeyContext::parse("Terminal raw_control").unwrap()];
        let action = |map: &Keymap, stroke, context: &[KeyContext]| {
            map.bindings_for_input(&[Keystroke::parse(stroke).unwrap()], context).0
                .first().map(|binding| binding.action().as_any().type_id())
        };
        let mut settings = ShortcutSettings::default();
        settings.select(Some(shortcuts::PC_PRESET));
        let pc = Keymap::new(bindings(&settings));
        assert_eq!(action(&pc, "ctrl-shift-v", &contexts), Some(TypeId::of::<PasteToTerminal>()));

        settings.set_shortcut("pasteToTerminal", Some("ctrl+v".into()));
        settings.set_shortcut("openTerminalSearch", Some("ctrl+1".into()));
        settings.set_shortcut("bottom", Some("ctrl+alt+→".into()));
        settings.set_shortcut("copyFromTerminal", None);
        settings.set_shortcut("selectAllInTerminal", Some("meta+a".into()));
        let custom = Keymap::new(bindings(&settings));
        assert_eq!(action(&custom, "ctrl-v", &contexts), Some(TypeId::of::<PasteToTerminal>()));
        assert_eq!(action(&custom, "ctrl-1", &contexts), Some(TypeId::of::<OpenTerminalSearch>()));
        assert_eq!(action(&custom, "ctrl-alt-right", &contexts), Some(TypeId::of::<NextTab>()));
        assert_eq!(action(&custom, "ctrl-v", &contexts[..1]), None);
        assert_eq!(action(&custom, "ctrl-2", &contexts[..1]), Some(TypeId::of::<SwitchToTab2>()));
        assert_eq!(action(&custom, "cmd-a", &unselected), Some(TypeId::of::<SelectAllInTerminal>()));
        for key in ["ctrl-v", "ctrl-1", "ctrl-2", "ctrl-alt-right", "ctrl-alt-left"] {
            assert_eq!(action(&custom, key, &unselected), None, "{key}");
        }
        assert_eq!(action(&pc, "ctrl-shift-v", &unselected), None);
        for key in ["ctrl-shift-c", "ctrl-shift-v", "ctrl-tab", "f6"] {
            assert_eq!(action(&custom, key, &contexts), None, "{key}");
        }
        assert_eq!(action(&custom, "f6", &contexts[..1]), Some(TypeId::of::<FocusWindowControls>()));

        settings.select(None);
        let disabled = Keymap::new(bindings(&settings));
        for key in ["ctrl-v", "ctrl-1", "ctrl-alt-right", "ctrl-shift-v", "ctrl-tab"] {
            assert_eq!(action(&disabled, key, &contexts), None, "{key}");
        }
    }
}
