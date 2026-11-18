//! Colour tokens. Values marked "bundle" are taken from the legacy UI bundle
//! (legacy/src/ui/assets); the rest are sampled from screenshots until the
//! matching tokens are found in the bundle.

/// Sampled.
pub const TITLE_BAR: u32 = 0x151826;
/// Sampled.
pub const BACKGROUND: u32 = 0x1e2133;
/// Sampled: left navigation of the Vaults tab.
pub const SIDEBAR: u32 = 0x23263a;
/// Sampled: cards and inactive tabs.
pub const SURFACE: u32 = 0x292c3f;
/// Sampled.
pub const SURFACE_ACTIVE: u32 = 0x353950;
/// Sampled: search field.
pub const INPUT: u32 = 0x3a3e54;
/// Sampled.
pub const TEXT: u32 = 0xe6e8ef;
/// Bundle: muted label colour in main CSS.
pub const TEXT_MUTED: u32 = 0x8d91a5;
/// Bundle: accent green (default terminal theme cursor).
pub const ACCENT: u32 = 0x21b568;
/// Sampled: group, key and identity icon tiles.
pub const TILE_BLUE: u32 = 0x0c4a79;
/// Sampled: Debian host tile.
pub const TILE_DEBIAN: u32 = 0xd70a53;
/// Sampled: Ubuntu host tile.
pub const TILE_UBUNTU: u32 = 0xe95420;
/// Bundle: `--red`, destructive buttons and errors.
pub const ERROR: u32 = 0xf25e61;
/// Bundle: `--dark-grey-3`, dialog header.
pub const DIALOG_HEADER: u32 = 0x282b3d;
/// Bundle: `--dark-grey-5`, rows inside dialogs.
pub const DIALOG_ROW: u32 = 0x3e4257;

/// Bundle: interface font of the legacy UI (`CircularXX`, assets/fonts).
pub const UI_FONT: &str = "CircularXX";

/// Sampled: background of a terminal tab while it connects.
pub const CONNECTING: u32 = 0x1f3b2f;
/// Sampled: title and cross of a terminal tab while it connects.
pub const CONNECTING_TEXT: u32 = 0x3dbd6e;

/// Window colours on a terminal tab: legacy paints the tab bar, tabs and side
/// panel from the terminal theme instead of the app's own colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    pub bar: u32,
    pub tab: u32,
    pub tab_active: u32,
    pub panel: u32,
    pub text: u32,
    pub muted: u32,
}

impl Chrome {
    /// Tabs and panel are the terminal background lightened, text stays the app's.
    pub fn from_terminal(background: u32) -> Self {
        let tint = |amount: u8| crate::terminal::blend(0xffffff, background, amount);
        Self { bar: background, tab: tint(16), tab_active: tint(36), panel: tint(10), text: TEXT, muted: TEXT_MUTED }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_lightens_the_terminal_background_and_keeps_app_text() {
        let chrome = Chrome::from_terminal(0x000000);
        assert_eq!((chrome.bar, chrome.text), (0x000000, TEXT));
        assert!(chrome.panel < chrome.tab && chrome.tab < chrome.tab_active);
        assert_eq!(chrome.tab, 0x101010);
    }
}
