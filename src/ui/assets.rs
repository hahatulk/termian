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
    "hosts", "keychain", "port_forwarding", "snippets", "known_hosts", "logs", "group", "debian", "ubuntu",
    "linux", "terminal", "serial", "new_host", "search", "grid", "tag", "chevron", "sync", "download", "upload",
    "close", "plus", "sftp", "identity",
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
