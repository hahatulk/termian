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
    "sftp_arrow", "sftp_chevron", "sftp_eye_off",
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

#[cfg(windows)]
pub(crate) fn set_window_icon(window: &gpui::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::{ptr, sync::OnceLock};
    use windows_sys::Win32::{
        Foundation::HWND,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            GetSystemMetrics, LoadImageW, SendMessageW, ICON_BIG, ICON_SMALL, IMAGE_ICON,
            SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
        },
    };

    const ICON_RESOURCE_ID: usize = 1;
    static WINDOW_ICONS: OnceLock<[(u32, isize); 2]> = OnceLock::new();

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get() as HWND;
    let icons = WINDOW_ICONS.get_or_init(|| {
        let module = unsafe { GetModuleHandleW(ptr::null()) };
        let resource = ptr::without_provenance(ICON_RESOURCE_ID);
        [
            (ICON_BIG, SM_CXICON, SM_CYICON),
            (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
        ]
        .map(|(kind, cx, cy)| {
            let (width, height) = unsafe { (GetSystemMetrics(cx), GetSystemMetrics(cy)) };
            let icon = unsafe { LoadImageW(module, resource, IMAGE_ICON, width, height, 0) };
            if icon.is_null() {
                eprintln!(
                    "[Termian] Could not load Windows icon {width}x{height}: {}",
                    std::io::Error::last_os_error()
                );
            }
            (kind, icon as isize)
        })
    });

    for &(kind, icon) in icons {
        if icon != 0 {
            unsafe { SendMessageW(hwnd, WM_SETICON, kind as usize, icon) };
        }
    }
}
