//! Settings → Shortcuts (legacy `index-347b6706` and `ShortcutsManager`):
//! the «Mac hotkeys» / «PC hotkeys» presets, user schemes made by editing a
//! preset («<name> copy»), the current scheme or «Disabled», and the key
//! strings in the legacy form (`meta+shift+s`). Device-local, as legacy keeps
//! them in `localStorage`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Legacy `getAvailableActions`, in the order the tab lists them: type and name.
pub const ACTIONS: [(&str, &str); 18] = [
    ("copyFromTerminal", "Copy from Terminal"),
    ("pasteToTerminal", "Paste to Terminal"),
    ("selectAllInTerminal", "Select All in Terminal"),
    ("terminals", "Open Hosts Page"),
    ("localTerminal", "Open Local Terminal"),
    ("serialConnection", "Open Serial Connection"),
    ("close", "Close Tab"),
    ("bottom", "Next Tab"),
    ("top", "Previous Tab"),
    ("forwarding", "Open Port Forwarding"),
    ("commandPalette", "Open Command Palette"),
    ("jumpTo", "Jump To"),
    ("newTab", "New Tab"),
    ("clearTerminalBuffer", "Clear Terminal Buffer"),
    ("openTerminalSearch", "Open Terminal Search"),
    ("toggleSidepanel", "Show Terminal Side-panel"),
    ("toggleSplitViewBroadcastMode", "Switch the Broadcast Mode"),
    ("snippets", "Open Snippets"),
];

pub const MAC_PRESET: &str = "Mac hotkeys";
pub const PC_PRESET: &str = "PC hotkeys";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    #[serde(rename = "type")]
    pub kind: String,
    /// `None` is «Disabled».
    pub shortcut: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scheme {
    pub shortcuts: Vec<Binding>,
    /// «Switch to specific Tab [1…9]» modifier.
    #[serde(rename = "mod")]
    pub tab_mod: String,
    /// «Move focus between Split View panes» modifier.
    #[serde(rename = "paneMod")]
    pub pane_mod: String,
    #[serde(default)]
    pub editable: bool,
    #[serde(default, rename = "parentScheme", skip_serializing_if = "Option::is_none")]
    pub parent_scheme: Option<String>,
}

impl Scheme {
    pub fn shortcut(&self, kind: &str) -> Option<&str> {
        self.shortcuts.iter().find(|b| b.kind == kind).and_then(|b| b.shortcut.as_deref())
    }
}

fn preset(pairs: &[(&str, Option<&str>)], tab_mod: &str, pane_mod: &str) -> Scheme {
    Scheme {
        shortcuts: pairs.iter().map(|(kind, shortcut)| Binding { kind: (*kind).into(), shortcut: shortcut.map(str::to_owned) }).collect(),
        tab_mod: tab_mod.into(),
        pane_mod: pane_mod.into(),
        editable: false,
        parent_scheme: None,
    }
}

/// Legacy `MAC_HOTKEYS`.
pub fn mac_preset() -> Scheme {
    preset(
        &[
            ("copyFromTerminal", Some("meta+c")),
            ("pasteToTerminal", Some("meta+v")),
            ("selectAllInTerminal", Some("meta+a")),
            ("toggleSidepanel", Some("meta+s")),
            ("localTerminal", Some("meta+l")),
            ("toggleSplitViewBroadcastMode", Some("meta+b")),
            ("serialConnection", Some("meta+alt+s")),
            ("clearTerminalBuffer", None),
            ("openTerminalSearch", Some("meta+f")),
            ("terminals", None),
            ("close", Some("meta+w")),
            ("snippets", Some("meta+shift+s")),
            ("forwarding", Some("meta+p")),
            ("commandPalette", Some("meta+k")),
            ("jumpTo", Some("meta+j")),
            ("newTab", Some("meta+t")),
            ("bottom", Some("meta+shift+]")),
            ("top", Some("meta+shift+[")),
        ],
        "meta",
        "meta+alt",
    )
}

pub fn pc_preset() -> Scheme {
    preset(
        &[
            ("copyFromTerminal", Some("ctrl+shift+c")),
            ("pasteToTerminal", Some("ctrl+shift+v")),
            ("selectAllInTerminal", Some("ctrl+shift+a")),
            ("toggleSidepanel", Some("ctrl+s")),
            ("localTerminal", Some("ctrl+l")),
            ("toggleSplitViewBroadcastMode", Some("ctrl+alt+b")),
            ("serialConnection", Some("ctrl+alt+s")),
            ("clearTerminalBuffer", None),
            ("openTerminalSearch", Some("ctrl+shift+f")),
            ("terminals", None),
            ("close", Some("ctrl+shift+w")),
            ("snippets", Some("ctrl+shift+s")),
            ("forwarding", Some("ctrl+p")),
            ("commandPalette", Some("ctrl+k")),
            ("jumpTo", Some("ctrl+j")),
            ("newTab", Some("ctrl+t")),
            ("bottom", Some("alt+→")),
            ("top", Some("alt+←")),
        ],
        "ctrl",
        "ctrl+alt",
    )
}

/// Presets for this platform, first the default (legacy `defaultPresets`).
pub fn presets() -> Vec<(&'static str, Scheme)> {
    if cfg!(target_os = "macos") {
        vec![(MAC_PRESET, mac_preset()), (PC_PRESET, pc_preset())]
    } else {
        vec![(PC_PRESET, pc_preset())]
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Current {
    #[default]
    NotSet,
    Enabled(String),
    Disabled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutSettings {
    /// Schemes the user made; presets are added on read.
    pub schemes: BTreeMap<String, Scheme>,
    pub current: Current,
}

impl ShortcutSettings {
    /// Presets and user schemes by name (a stored preset name is the preset).
    pub fn schemes(&self) -> BTreeMap<String, Scheme> {
        let mut all: BTreeMap<String, Scheme> = self.schemes.iter().filter(|(_, s)| s.editable).map(|(n, s)| (n.clone(), s.clone())).collect();
        for (name, scheme) in presets() {
            all.insert(name.into(), scheme);
        }
        all
    }

    /// Names in the picker order: presets first, then user schemes.
    pub fn names(&self) -> Vec<String> {
        let presets: Vec<String> = presets().into_iter().map(|(n, _)| n.to_owned()).collect();
        let users = self.schemes.iter().filter(|(n, s)| s.editable && !presets.contains(n)).map(|(n, _)| n.clone());
        presets.iter().cloned().chain(users).collect()
    }

    fn default_name() -> String {
        presets()[0].0.to_owned()
    }

    /// The scheme in use; `None` when shortcuts are disabled.
    pub fn current_scheme(&self) -> Option<(String, Scheme)> {
        let all = self.schemes();
        let name = match &self.current {
            Current::Disabled => return None,
            Current::NotSet => Self::default_name(),
            Current::Enabled(name) if all.contains_key(name) => name.clone(),
            Current::Enabled(_) => Self::default_name(),
        };
        let scheme = all.get(&name).cloned()?;
        Some((name, scheme))
    }

    /// `None` disables shortcuts; an unknown name changes nothing.
    pub fn select(&mut self, name: Option<&str>) {
        match name {
            None => self.current = Current::Disabled,
            Some(name) if self.schemes().contains_key(name) => self.current = Current::Enabled(name.into()),
            Some(_) => {}
        }
    }

    pub fn is_editable(&self, name: &str) -> bool {
        self.schemes.get(name).is_some_and(|s| s.editable)
    }

    /// Renames a user scheme; an empty or preset name keeps the old one.
    pub fn rename(&mut self, from: &str, to: &str) {
        let to = to.trim();
        if to.is_empty() || from == to || !self.is_editable(from) || self.schemes().contains_key(to) {
            return;
        }
        if let Some(scheme) = self.schemes.remove(from) {
            self.schemes.insert(to.into(), scheme);
            if self.current == Current::Enabled(from.into()) {
                self.current = Current::Enabled(to.into());
            }
        }
    }

    /// Removes a user scheme; the default preset takes over if it was in use.
    pub fn delete(&mut self, name: &str) {
        if self.is_editable(name) {
            self.schemes.remove(name);
            if self.current == Current::Enabled(name.into()) {
                self.current = Current::Enabled(Self::default_name());
            }
        }
    }

    /// Changes the current scheme; a preset is first copied to «<name> copy».
    fn modify(&mut self, change: impl FnOnce(&mut Scheme)) {
        let Some((name, mut scheme)) = self.current_scheme() else { return };
        let name = if scheme.editable {
            name
        } else {
            let base = format!("{name} copy");
            let all = self.schemes();
            let mut candidate = base.clone();
            let mut n = 0;
            while all.contains_key(&candidate) {
                candidate = format!("{base} {n}");
                n += 1;
            }
            scheme.editable = true;
            scheme.parent_scheme = Some(name);
            candidate
        };
        change(&mut scheme);
        self.schemes.insert(name.clone(), scheme);
        self.current = Current::Enabled(name);
    }

    /// Binds `kind` to `shortcut`; the action that had it loses it.
    pub fn set_shortcut(&mut self, kind: &str, shortcut: Option<String>) {
        if !ACTIONS.iter().any(|(k, _)| *k == kind) {
            return;
        }
        let shortcut = shortcut.map(|s| normalize(&s));
        self.modify(|scheme| {
            if let Some(new) = &shortcut {
                for binding in scheme.shortcuts.iter_mut().filter(|b| b.shortcut.as_deref().map(normalize).as_deref() == Some(new.as_str())) {
                    binding.shortcut = None;
                }
            }
            match scheme.shortcuts.iter_mut().find(|b| b.kind == kind) {
                Some(binding) => binding.shortcut = shortcut,
                None => scheme.shortcuts.push(Binding { kind: kind.into(), shortcut }),
            }
        });
    }

    pub fn set_tab_mod(&mut self, modifier: &str) {
        let modifier = normalize(modifier);
        self.modify(|scheme| scheme.tab_mod = modifier);
    }

    pub fn set_pane_mod(&mut self, modifier: &str) {
        let modifier = normalize(modifier);
        self.modify(|scheme| scheme.pane_mod = modifier);
    }

    /// The action that already has `shortcut` («This will disable "…" shortcut»).
    pub fn conflict(&self, kind: &str, shortcut: &str) -> Option<&'static str> {
        let (_, scheme) = self.current_scheme()?;
        let shortcut = normalize(shortcut);
        scheme
            .shortcuts
            .iter()
            .find(|b| b.kind != kind && b.shortcut.as_deref().map(normalize).as_deref() == Some(shortcut.as_str()))
            .and_then(|b| ACTIONS.iter().find(|(k, _)| *k == b.kind).map(|(_, name)| *name))
    }
}

/// Modifier tokens in the legacy order.
const MODIFIERS: [&str; 4] = ["meta", "ctrl", "alt", "shift"];

fn modifier(token: &str) -> Option<&'static str> {
    match token.to_lowercase().as_str() {
        "meta" | "cmd" | "win" | "super" | "⌘" => Some("meta"),
        "ctrl" | "control" | "⌃" => Some("ctrl"),
        "alt" | "option" | "⌥" => Some("alt"),
        "shift" | "⇧" => Some("shift"),
        _ => None,
    }
}

/// Splits `meta+shift+]` into modifiers and the key; a trailing `+` key
/// (`ctrl++`) is kept.
fn parts(shortcut: &str) -> (Vec<&'static str>, Option<String>) {
    let (body, plus) = match shortcut.strip_suffix("++") {
        Some(body) => (body, true),
        None => (shortcut, false),
    };
    let mut mods = Vec::new();
    let mut key = plus.then(|| "+".to_owned());
    for token in body.split('+').filter(|t| !t.is_empty()) {
        match modifier(token) {
            Some(m) if !mods.contains(&m) => mods.push(m),
            Some(_) => {}
            None => key = Some(token.to_lowercase()),
        }
    }
    mods.sort_by_key(|m| MODIFIERS.iter().position(|x| x == m));
    (mods, key)
}

/// One spelling for comparisons and storage: `meta+shift+s`.
pub fn normalize(shortcut: &str) -> String {
    let (mods, key) = parts(shortcut);
    mods.into_iter().map(str::to_owned).chain(key).collect::<Vec<_>>().join("+")
}

/// Key caps to show, as legacy `getFormattedKeys` draws them.
pub fn display(shortcut: &str) -> Vec<String> {
    let mac = cfg!(target_os = "macos");
    let (mods, key) = parts(shortcut);
    let cap = |m: &str| -> String {
        match (m, mac) {
            ("meta", true) => "⌘",
            ("meta", false) => "Win",
            ("ctrl", true) => "⌃",
            ("ctrl", false) => "Ctrl",
            ("alt", true) => "⌥",
            ("alt", false) => "Alt",
            _ => "Shift",
        }
        .into()
    };
    let key = key.map(|k| {
        let mut chars = k.chars();
        chars.next().map(|c| c.to_uppercase().chain(chars).collect::<String>()).unwrap_or_default()
    });
    mods.into_iter().map(cap).chain(key).collect()
}

/// The GPUI keystroke (`cmd-shift-]`), or `None` for a modifier alone.
pub fn keystroke(shortcut: &str) -> Option<String> {
    let (mods, key) = parts(shortcut);
    let key = match key?.as_str() {
        "→" | "arrowright" => "right".to_owned(),
        "←" | "arrowleft" => "left".to_owned(),
        "↑" | "arrowup" => "up".to_owned(),
        "↓" | "arrowdown" => "down".to_owned(),
        "esc" => "escape".to_owned(),
        other => other.to_owned(),
    };
    let mods = mods.into_iter().map(|m| match m {
        "meta" => "cmd",
        "ctrl" => "ctrl",
        "alt" => "alt",
        _ => "shift",
    });
    Some(mods.map(str::to_owned).chain(std::iter::once(key)).collect::<Vec<_>>().join("-"))
}

/// `meta+alt` → `cmd-alt` for prefixing the tab digits and pane arrows.
pub fn modifier_keystroke(modifier: &str) -> String {
    let (mods, _) = parts(modifier);
    mods.into_iter()
        .map(|m| match m {
            "meta" => "cmd",
            "ctrl" => "ctrl",
            "alt" => "alt",
            _ => "shift",
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// A key press recorded in the shortcut field (legacy keydown handler):
/// modifiers, then the key, in the stored spelling.
pub fn record(control: bool, alt: bool, shift: bool, platform: bool, key: &str) -> String {
    let key = match key {
        "right" => "→",
        "left" => "←",
        "up" => "↑",
        "down" => "↓",
        other => other,
    };
    let mods = [(platform, "meta"), (control, "ctrl"), (alt, "alt"), (shift, "shift")];
    mods.iter().filter(|(on, _)| *on).map(|(_, m)| (*m).to_owned()).chain((!key.is_empty()).then(|| key.to_lowercase())).collect::<Vec<_>>().join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_match_the_legacy_tables() {
        let mac = mac_preset();
        assert_eq!((mac.shortcut("commandPalette"), mac.shortcut("terminals"), mac.tab_mod.as_str()), (Some("meta+k"), None, "meta"));
        assert_eq!(pc_preset().shortcut("copyFromTerminal"), Some("ctrl+shift+c"));
        for scheme in [mac, pc_preset()] {
            assert_eq!(scheme.shortcuts.len(), ACTIONS.len());
            assert!(ACTIONS.iter().all(|(kind, _)| scheme.shortcuts.iter().any(|b| b.kind == *kind)));
        }
        let settings = ShortcutSettings::default();
        assert_eq!(settings.current_scheme().unwrap().0, presets()[0].0, "not set: the first preset");
    }

    #[test]
    fn editing_a_preset_makes_a_copy_and_moves_a_taken_key() {
        let mut s = ShortcutSettings::default();
        let (preset, scheme) = s.current_scheme().unwrap();
        let taken = scheme.shortcut("newTab").unwrap().to_owned();
        assert_eq!(s.conflict("close", &taken), Some("New Tab"));
        s.set_shortcut("close", Some(taken.clone()));
        let (name, scheme) = s.current_scheme().unwrap();
        assert_eq!(name, format!("{preset} copy"));
        assert!(scheme.editable && scheme.parent_scheme.as_deref() == Some(preset.as_str()));
        assert_eq!((scheme.shortcut("close"), scheme.shortcut("newTab")), (Some(taken.as_str()), None));
        assert_eq!(s.schemes()[&preset], presets()[0].1, "the preset itself is untouched");

        // A second preset edit makes «… copy 0».
        s.select(Some(&preset));
        s.set_tab_mod("ctrl+alt");
        assert_eq!(s.current_scheme().unwrap().0, format!("{preset} copy 0"));
        assert_eq!(s.current_scheme().unwrap().1.tab_mod, "ctrl+alt");
    }

    #[test]
    fn schemes_are_renamed_deleted_and_disabled() {
        let mut s = ShortcutSettings::default();
        s.set_shortcut("snippets", None);
        let (copy, _) = s.current_scheme().unwrap();
        s.rename(&copy, "Mine");
        assert_eq!(s.current_scheme().unwrap().0, "Mine");
        s.rename("Mine", presets()[0].0);
        assert_eq!(s.current_scheme().unwrap().0, "Mine", "a preset name is refused");
        s.rename(presets()[0].0, "Other");
        assert!(s.names().contains(&presets()[0].0.to_owned()), "presets cannot be renamed");
        s.select(None);
        assert_eq!(s.current_scheme(), None);
        s.select(Some("Mine"));
        s.delete("Mine");
        assert_eq!(s.current_scheme().unwrap().0, presets()[0].0);
        assert!(!s.names().contains(&"Mine".to_owned()));
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<ShortcutSettings>(&json).unwrap(), s);
    }

    #[test]
    fn key_strings_convert_between_legacy_display_and_gpui() {
        assert_eq!(normalize("Cmd+Shift+S"), "meta+shift+s");
        assert_eq!(normalize("shift+meta+]"), "meta+shift+]");
        assert_eq!(keystroke("meta+shift+]").as_deref(), Some("cmd-shift-]"));
        assert_eq!(keystroke("alt+→").as_deref(), Some("alt-right"));
        assert_eq!(keystroke("ctrl++").as_deref(), Some("ctrl-+"));
        assert_eq!(keystroke("meta"), None);
        assert_eq!(modifier_keystroke("meta+alt"), "cmd-alt");
        assert_eq!(record(false, true, false, true, "s"), "meta+alt+s");
        assert_eq!(record(true, false, false, false, "left"), "ctrl+←");
        if cfg!(target_os = "macos") {
            assert_eq!(display("meta+shift+s"), ["⌘", "Shift", "S"]);
        } else {
            assert_eq!(display("ctrl+alt+b"), ["Ctrl", "Alt", "B"]);
        }
    }
}
