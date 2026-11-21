use serde::{Deserialize, Serialize};

use crate::{
    language::Language,
    listing::ListingSettings,
    privacy_password::PrivacyPassword,
    shortcuts::ShortcutSettings,
    terminal_settings::{self, TerminalSettings},
    updater::{Channel, Schedule},
    vault::Vault,
};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub terminal: TerminalSettings,
    pub listing: ListingSettings,
    pub shortcuts: ShortcutSettings,
    pub language: Language,
    pub update_channel: Channel,
    pub update_schedule: Schedule,
    pub logs_disabled: bool,
    pub privacy_password: Option<PrivacyPassword>,
}

impl AppSettings {
    pub fn from_vault(vault: &Vault) -> Self {
        Self {
            terminal: vault.terminal.clone(),
            listing: vault.listing.clone(),
            shortcuts: vault.shortcuts.clone(),
            language: vault.language,
            update_channel: vault.update_channel,
            update_schedule: vault.update_schedule,
            logs_disabled: vault.logs_disabled,
            privacy_password: vault.privacy_password.clone(),
        }
    }

    pub(super) fn migrate(vault: &Vault) -> Self {
        let mut settings = Self::from_vault(vault);
        if let Some(seconds) = terminal_settings::saved_keep_alive(&vault.foreign) {
            settings.terminal.keep_alive = seconds;
        }
        settings
    }

    pub fn apply(self, vault: &mut Vault) {
        vault.terminal = self.terminal;
        vault.listing = self.listing;
        vault.shortcuts = self.shortcuts;
        vault.language = self.language;
        vault.update_channel = self.update_channel;
        vault.update_schedule = self.update_schedule;
        vault.logs_disabled = self.logs_disabled;
        vault.privacy_password = self.privacy_password;
    }
}
