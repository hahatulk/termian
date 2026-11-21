use serde_json::{json, Value};

use super::{text, App};
use crate::{
    session_log,
    settings_menu::{AppItem, Tab},
    shortcuts,
    terminal_settings::{self as ts, TerminalSettings},
    vault::Vault,
    workspace_template::Target,
};

pub(super) fn menus() -> Value {
    let pc = shortcuts::pc_preset(false);
    let keys = shortcuts::ACTIONS
        .iter()
        .filter_map(|&(id, title)| {
            if !matches!(
                id,
                "pasteToTerminal"
                    | "localTerminal"
                    | "close"
                    | "bottom"
                    | "top"
                    | "forwarding"
                    | "newTab"
                    | "toggleSplitViewBroadcastMode"
                    | "snippets"
            ) {
                return None;
            }
            let shortcut = pc.shortcut(id)?;
            Some(json!({"id":id, "title":title, "keys":shortcut}))
        })
        .collect::<Vec<_>>();
    json!({
        "app": AppItem::COMMON.map(|item| json!({"id":item.id(), "title":item.title()})),
        "tabs": Tab::ALL.map(|tab| json!({"id":tab.title().to_ascii_lowercase(), "title":tab.title()})),
        "shortcuts": keys,
        "logPeriods": session_log::PERIODS.map(|(_, title, _, period)| json!({
            "title":title, "hours":period.as_secs() / 3600
        })),
    })
}

pub(super) fn themes() -> Value {
    json!({"themes":ts::themes().iter().map(|theme| json!({
        "name":theme.name,
        "background":theme.background,
        "foreground":theme.foreground,
        "colors":theme.swatches(),
    })).collect::<Vec<_>>()})
}

pub(super) fn theme_name(name: &str) -> Result<&'static str, String> {
    ts::themes()
        .iter()
        .find(|theme| theme.name.eq_ignore_ascii_case(name))
        .map(|theme| theme.name.as_str())
        .ok_or_else(|| "Theme not found.".into())
}

pub(super) fn for_host(vault: &Vault, host: Option<u64>) -> TerminalSettings {
    let mut settings = vault.terminal.clone();
    if let Some(name) = host
        .and_then(|id| vault.host(id))
        .and_then(|host| host.settings.theme_name())
    {
        settings.color_scheme = name.to_owned();
    }
    settings
}

impl App {
    pub(super) fn terminal_settings(&self) -> Value {
        let t = &self.vault.terminal;
        json!({
            "font":t.font_size, "minFont":ts::MIN_FONT_SIZE, "maxFont":ts::MAX_FONT_SIZE,
            "theme":t.theme().name, "emulation":t.emulation(), "emulations":ts::EMULATION_TYPES,
            "rightClickPaste":t.copy_on_select, "bellSound":t.bell_sound, "brightenBold":t.brighten_bold,
            "keepAlive":ts::keep_alive(&self.vault.foreign),
            "scrollback":t.scrollback_field(), "pqc":t.pqc_kex,
        })
    }

    pub(super) fn save_terminal_setting(&mut self, args: &Value) -> Result<Value, String> {
        let mut vault = self.vault.clone();
        let value = &args["value"];
        let boolean = || {
            value
                .as_bool()
                .ok_or_else(|| "Select Enabled or Disabled.".to_owned())
        };
        let number = || {
            value
                .as_u64()
                .ok_or_else(|| "Enter a non-negative whole number.".to_owned())
        };
        let t = &mut vault.terminal;
        match text(args, "field") {
            "font" => t.set_font_size(number()?.min(u64::from(ts::MAX_FONT_SIZE)) as u32),
            "theme" => t.color_scheme = theme_name(text(args, "value"))?.to_lowercase(),
            "rightClickPaste" => t.copy_on_select = boolean()?,
            "bellSound" => t.bell_sound = boolean()?,
            "brightenBold" => t.brighten_bold = boolean()?,
            "emulation" => {
                let kind = text(args, "value");
                if !ts::EMULATION_TYPES.contains(&kind) {
                    return Err("Select a terminal emulation type.".into());
                }
                t.emulation = kind.into();
            }
            "scrollback" => t.set_scrollback_field(number()?),
            "pqc" => t.pqc_kex = boolean()?,
            "keepAlive" => {
                ts::set_keep_alive(&mut vault.foreign, number()?.min(i32::MAX as u64) as u32)
            }
            _ => return Err("Unknown terminal setting.".into()),
        }
        self.save(vault)?;
        self.apply_terminal_settings();
        Ok(self.terminal_settings())
    }

    pub(super) fn apply_terminal_settings(&mut self) {
        for connection in self.sessions.values_mut() {
            let host = match connection.source {
                Some(Target::Host(id)) => Some(id),
                _ => None,
            };
            connection.screen.apply(&for_host(&self.vault, host));
            connection.dirty = true;
        }
    }
}
