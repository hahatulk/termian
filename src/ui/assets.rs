//! Icons bundled into the binary; SVGs come from the legacy UI bundle.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        const ICONS: &[(&str, &[u8])] = &[
            $((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../../assets/icons/", $name, ".svg")))),*
        ];
    };
}

icons!(
    "hosts", "keychain", "port_forwarding", "snippets", "known_hosts", "logs", "group", "debian", "ubuntu", "ping",
    "linux", "terminal", "serial", "new_host", "search", "grid", "tag", "sync", "download", "upload",
    "close", "plus", "sftp", "identity", "file", "folder", "link", "arrow_left", "arrow_right", "local", "connect",
    "check", "alert", "edit", "dots", "pf_local", "pf_remote", "pf_dynamic", "bookmark", "bookmark_filled", "snippets_package", "side_panel", "theme", "spinner_arc", "person", "cloud_sync", "cloud_upload", "cloud_download", "workspace", "broadcast", "expand", "collapse", "calendar", "list", "sorting", "vaults", "caret_down", "agent", "backspace", "charset", "env", "collapse_right", "navigate", "new_tab", "users", "privacy_lock", "privacy_open",
);

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS.iter().find(|(name, _)| *name == path).map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS.iter().filter(|(name, _)| name.starts_with(path)).map(|(name, _)| (*name).into()).collect())
    }
}

pub fn icon(name: &str) -> SharedString {
    format!("icons/{name}.svg").into()
}
