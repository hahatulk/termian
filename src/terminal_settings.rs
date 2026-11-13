//! Settings → Terminal, with the legacy defaults (`reducers$2` in the legacy
//! bundle): emulation, font, theme, keyword highlighting, scrollback and the
//! rest. Legacy keeps them on the device, so do we; only the keepalive
//! interval travels in the snapshot (`settings.keep_alive_interval`).

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Legacy `minFontSize` / `maxFontSize`.
pub const MIN_FONT_SIZE: u32 = 4;
pub const MAX_FONT_SIZE: u32 = 40;
/// Legacy `maxScrollbackSize`: what «0» in the field stands for.
pub const MAX_SCROLLBACK: u64 = 4_294_967_295;
/// The emulator keeps at most this many lines (alacritty's own limit).
pub const SCROLLBACK_LIMIT: usize = 100_000;
pub const EMULATION_TYPES: [&str; 3] = ["xterm-256color", "xterm-16color", "xterm"];
/// Legacy `fontFamilies`, in the order the picker lists them.
pub const FONT_FAMILIES: [&str; 17] = [
    "Source Code Pro",
    "Source Code Pro Medium",
    "Operator Mono Book",
    "Operator Mono Medium",
    "Fira Mono",
    "Fira Mono Medium",
    "Inconsolata-g",
    "Anonymous Pro",
    "Ubuntu Mono",
    "Droid Sans Mono",
    "Dejavu Sans Mono",
    "PT Mono",
    "Andale Mono",
    "Cascadia Code",
    "Fira Code",
    "JetBrains Mono",
    "Meslo",
];
/// The snapshot key legacy reads with `getKeepAliveInterval`.
pub const KEEP_ALIVE_KEY: &str = "keep_alive_interval";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalSettings {
    pub autoreconnect: bool,
    pub shell_history_import: bool,
    /// Installs the shell hooks that record commands (Shell History).
    pub shell_integration: bool,
    /// «Select text to copy & Right click to paste».
    pub copy_on_select: bool,
    pub bell_sound: bool,
    pub option_is_meta: bool,
    pub brighten_bold: bool,
    pub emulation: String,
    pub font_family: String,
    pub font_size: u32,
    /// Lower-case theme name, as legacy stores `colorScheme`.
    pub color_scheme: String,
    pub keyword_highlighting: bool,
    pub detect_os: bool,
    /// Shell of local terminals; `None` is the system default.
    pub local_shell: Option<String>,
    /// Paths the user typed into «Local Terminal Path».
    pub custom_shells: Vec<String>,
    /// Lines kept; [`MAX_SCROLLBACK`] means «0» (as many as possible).
    pub scrollback: u64,
    pub pqc_kex: bool,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            autoreconnect: true,
            shell_history_import: true,
            shell_integration: true,
            copy_on_select: false,
            bell_sound: false,
            option_is_meta: true,
            brighten_bold: false,
            emulation: EMULATION_TYPES[0].into(),
            font_family: FONT_FAMILIES[0].into(),
            font_size: 14,
            color_scheme: "termius dark".into(),
            keyword_highlighting: true,
            detect_os: true,
            local_shell: None,
            custom_shells: Vec::new(),
            scrollback: 40_000,
            pqc_kex: true,
        }
    }
}

impl TerminalSettings {
    pub fn set_font_size(&mut self, size: u32) {
        self.font_size = size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
    }

    /// The Scrollback field: legacy shows 0 for the maximum.
    pub fn scrollback_field(&self) -> u64 {
        if self.scrollback == MAX_SCROLLBACK { 0 } else { self.scrollback }
    }

    /// On blur legacy stores `n || max`; values above the maximum read as 0.
    pub fn set_scrollback_field(&mut self, value: u64) {
        self.scrollback = if value == 0 || value > MAX_SCROLLBACK { MAX_SCROLLBACK } else { value };
    }

    /// Lines the emulator keeps.
    pub fn scrollback_lines(&self) -> usize {
        (self.scrollback.min(SCROLLBACK_LIMIT as u64)) as usize
    }

    pub fn emulation(&self) -> &str {
        if EMULATION_TYPES.contains(&self.emulation.as_str()) { &self.emulation } else { EMULATION_TYPES[0] }
    }

    pub fn theme(&self) -> &'static Theme {
        theme(&self.color_scheme)
    }

    /// Adds a typed shell path to the list and selects it.
    pub fn add_shell(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            return;
        }
        if !self.custom_shells.iter().any(|p| p == path) {
            self.custom_shells.push(path.to_owned());
        }
        self.local_shell = Some(path.to_owned());
    }

    /// Removes a typed path; the default shell takes over if it was selected.
    pub fn remove_shell(&mut self, path: &str) {
        self.custom_shells.retain(|p| p != path);
        if self.local_shell.as_deref() == Some(path) {
            self.local_shell = None;
        }
    }
}

/// Keepalive seconds from the snapshot settings; 0 is off.
pub fn keep_alive(foreign: &serde_json::Value) -> u32 {
    let value = &foreign["settings"][KEEP_ALIVE_KEY];
    value.as_str().and_then(|s| s.trim().parse().ok()).or_else(|| value.as_u64().map(|n| n.min(u32::MAX as u64) as u32)).unwrap_or(0)
}

/// Legacy writes the interval as a string and removes the key for 0.
pub fn set_keep_alive(foreign: &mut serde_json::Value, seconds: u32) {
    if !foreign.is_object() {
        *foreign = serde_json::json!({});
    }
    if !foreign["settings"].is_object() {
        foreign["settings"] = serde_json::json!({});
    }
    let settings = foreign["settings"].as_object_mut().expect("an object");
    if seconds == 0 {
        settings.remove(KEEP_ALIVE_KEY);
    } else {
        settings.insert(KEEP_ALIVE_KEY.into(), serde_json::Value::String(seconds.to_string()));
    }
}

/// Keepalive field text → seconds, as legacy cleans it (digits only, capped).
pub fn parse_keep_alive(text: &str) -> u32 {
    let digits: String = text.chars().filter(char::is_ascii_digit).collect();
    digits.parse::<u64>().map(|n| n.min(i32::MAX as u64) as u32).unwrap_or(0)
}

/// Shells offered in «Local Terminal Path»: the default, then `/etc/shells`.
pub fn detected_shells() -> Vec<String> {
    let mut shells = vec![default_shell()];
    if let Ok(text) = std::fs::read_to_string("/etc/shells") {
        for line in text.lines().map(str::trim).filter(|l| l.starts_with('/')) {
            if !shells.iter().any(|s| s == line) {
                shells.push(line.to_owned());
            }
        }
    }
    shells
}

pub fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into())
    }
}

// ----------------------------------------------------------------- themes --

/// One legacy colour scheme (`colorSchemesArray`).
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub palette: [u32; 16],
    pub background: u32,
    pub foreground: u32,
    pub cursor: u32,
    pub selection: u32,
    /// Selection opacity (the `#rrggbbaa` alpha), 0–255.
    pub selection_alpha: u8,
}

#[derive(Deserialize)]
struct RawTheme {
    name: String,
    palette: Vec<String>,
    background: String,
    foreground: String,
    cursor: String,
    selection: String,
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa` → colour and alpha.
pub fn parse_color(text: &str) -> Option<(u32, u8)> {
    let hex = text.trim().trim_start_matches('#');
    let expand = |s: &str| s.chars().flat_map(|c| [c, c]).collect::<String>();
    let hex = if hex.len() == 3 { expand(hex) } else { hex.to_owned() };
    match hex.len() {
        6 => Some((u32::from_str_radix(&hex, 16).ok()?, 255)),
        8 => Some((u32::from_str_radix(&hex[..6], 16).ok()?, u8::from_str_radix(&hex[6..], 16).ok()?)),
        _ => None,
    }
}

pub fn themes() -> &'static [Theme] {
    static THEMES: OnceLock<Vec<Theme>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let raw: Vec<RawTheme> = serde_json::from_str(include_str!("terminal_themes.json")).expect("bundled themes");
        raw.into_iter()
            .filter_map(|t| {
                let color = |s: &str| parse_color(s).map(|(c, _)| c);
                let mut palette = [0u32; 16];
                for (slot, text) in palette.iter_mut().zip(&t.palette) {
                    *slot = color(text)?;
                }
                let (selection, selection_alpha) = parse_color(&t.selection)?;
                Some(Theme { palette, background: color(&t.background)?, foreground: color(&t.foreground)?, cursor: color(&t.cursor)?, selection, selection_alpha, name: t.name })
            })
            .collect()
    })
}

/// By lower-case name, falling back to Termius Dark like `safeGetColorScheme`.
pub fn theme(name: &str) -> &'static Theme {
    let all = themes();
    all.iter().find(|t| t.name.eq_ignore_ascii_case(name)).unwrap_or(&all[0])
}

// ------------------------------------------------------ keyword highlights --

pub const HIGHLIGHT_GROUPS: [&str; 6] = ["Error", "Warning", "OK", "Info", "Debug", "IP address & MAC"];

/// Colours of the groups for a theme (legacy `Ie`): five palettes by theme.
pub fn highlight_colors(theme: &str) -> [u32; 6] {
    const DARK: [u32; 6] = [0xF25E61, 0xFFCB00, 0x81D254, 0x2091F6, 0x8686FF, 0xD2549A];
    const LIGHT: [u32; 6] = [0xFF000F, 0xE36D00, 0x009618, 0x0029FF, 0x9E00E9, 0xEE0064];
    const WARM: [u32; 6] = [0xFF9698, 0xFFB55B, 0x9FFFCE, 0x56C2FF, 0xB4B4FF, 0xFF9FD4];
    const OCEAN: [u32; 6] = [0xFF7679, 0xF2FF5B, 0x6CFFB3, 0x7AF7FF, 0xCBA1FF, 0xFF82EB];
    const AEROGEL: [u32; 6] = [0xB20004, 0xFFC700, 0x39FF27, 0x0000FF, 0x8000FF, 0xBC0093];
    match theme.to_lowercase().as_str() {
        "termius light" | "basic" | "man page" | "novel" | "solarized light" | "nord light" => LIGHT,
        "grass" | "red sands" => WARM,
        "ocean" => OCEAN,
        "silver aerogel" => AEROGEL,
        _ => DARK,
    }
}

struct Rule {
    group: usize,
    regex: regex::Regex,
    /// Characters that must not touch the match on either side (the legacy
    /// look-behind / look-ahead, which `regex` has no syntax for).
    fence: Option<fn(char) -> bool>,
}

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    fn word(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }
    fn dash(c: char) -> bool {
        c == '-'
    }
    RULES.get_or_init(|| {
        let rule = |group, pattern: &str, fence| Rule { group, regex: regex::Regex::new(pattern).expect("legacy rule"), fence };
        vec![
            rule(0, r"(?i)[ :<|\[]error[0-9]?[ :>|\]]", None),
            rule(1, r"(?i)[ :<|\[](warning|warn)[0-9]?[ :>|\]]", None),
            rule(2, r"(?i)[ :<|\[](ok|yes|success)[ :>|\]]", None),
            rule(3, r"(?i)[ :<|\[]info[0-9]?[ :>|\]]", None),
            rule(4, r"(?i)[ :<|\[]debug[0-9]?[ :>|\]]", None),
            rule(5, r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}", Some(word)),
            rule(5, r"(?i)(?:[A-F0-9]{1,4}:){7}[A-F0-9]{1,4}", None),
            rule(5, r"(?i)([0-9A-F]{1,4}:){1,5}(:[0-9A-F]{1,4}){1,4}", Some(word)),
            rule(5, r"(?i)([A-F0-9]{2}[:.-]){5}[A-F0-9]{2}", None),
            rule(5, r"(?i)([A-F0-9]{4}[.-]){2}[A-F0-9]{4}", Some(dash)),
        ]
    })
}

/// Character ranges of `line` to recolour, with their colour, in the order
/// the legacy highlighter applies them (later rules paint over earlier ones).
pub fn highlights(line: &str, colors: &[u32; 6]) -> Vec<(usize, usize, u32)> {
    let mut out = Vec::new();
    for rule in rules() {
        let mut from = 0;
        while from <= line.len() {
            let Some(found) = rule.regex.find_at(line, from) else { break };
            let fenced = rule.fence.is_some_and(|touches| {
                line[..found.start()].chars().next_back().is_some_and(touches) || line[found.end()..].chars().next().is_some_and(touches)
            });
            if fenced {
                // Legacy would look for the next match from the next position.
                from = found.start() + line[found.start()..].chars().next().map_or(1, char::len_utf8);
                continue;
            }
            if !found.as_str().is_empty() {
                let start = line[..found.start()].chars().count();
                out.push((start, start + found.as_str().chars().count(), colors[rule.group]));
            }
            from = found.end().max(found.start() + 1);
        }
    }
    out
}

// ------------------------------------------------------------------- fonts --

/// A legacy font family: the bundled files and how they are named inside
/// (family and weight, as the platform font loader reports them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontFace {
    pub label: &'static str,
    pub regular: (&'static str, u16),
    pub bold: (&'static str, u16),
    /// File names in the legacy `src/ui/assets` (the `@font-face` sources).
    pub files: &'static [&'static str],
}

const SAUCE: &str = "SauceCodePro Nerd Font Mono";
const OPERATOR: &str = "Operator Mono";
const FIRA_MONO: &str = "FiraMono Nerd Font Mono";

pub const FONT_FACES: [FontFace; 17] = [
    FontFace { label: "Source Code Pro", regular: (SAUCE, 400), bold: (SAUCE, 700), files: &["SauceCodeProNerdFontMono-Regular-a075caa1.ttf", "SauceCodeProNerdFontMono-Bold-33bdfcf8.ttf"] },
    FontFace { label: "Source Code Pro Medium", regular: (SAUCE, 500), bold: (SAUCE, 700), files: &["SauceCodeProNerdFontMono-Medium-cc62f20a.ttf", "SauceCodeProNerdFontMono-Bold-33bdfcf8.ttf"] },
    FontFace { label: "Operator Mono Book", regular: (OPERATOR, 325), bold: (OPERATOR, 400), files: &["OperatorMono-Book-8b7c2d70.otf", "OperatorMono-Bold-9efd8643.otf"] },
    FontFace { label: "Operator Mono Medium", regular: (OPERATOR, 350), bold: (OPERATOR, 400), files: &["OperatorMono-Medium-67ac1792.otf", "OperatorMono-Bold-9efd8643.otf"] },
    FontFace { label: "Fira Mono", regular: (FIRA_MONO, 400), bold: (FIRA_MONO, 700), files: &["FiraMonoNerdFontMono-Regular-8f953740.otf", "FiraMonoNerdFontMono-Bold-8923d43c.otf"] },
    FontFace { label: "Fira Mono Medium", regular: (FIRA_MONO, 500), bold: (FIRA_MONO, 700), files: &["FiraMonoNerdFontMono-Medium-0aa28652.otf", "FiraMonoNerdFontMono-Bold-8923d43c.otf"] },
    FontFace { label: "Inconsolata-g", regular: ("Inconsolata Nerd Font Mono", 400), bold: ("Inconsolata Nerd Font Mono", 700), files: &["InconsolataNerdFontMono-Regular-8e959977.ttf", "InconsolataNerdFontMono-Bold-ef7f1722.ttf"] },
    FontFace { label: "Anonymous Pro", regular: ("AnonymicePro Nerd Font Mono", 400), bold: ("AnonymicePro Nerd Font Mono", 700), files: &["AnonymiceProNerdFontMono-Regular-fcf2f9ba.ttf", "AnonymiceProNerdFontMono-Bold-b69192d7.ttf"] },
    FontFace { label: "Ubuntu Mono", regular: ("UbuntuMono Nerd Font Mono", 400), bold: ("UbuntuMono Nerd Font Mono", 700), files: &["UbuntuMonoNerdFontMono-Regular-e6d08eec.ttf", "UbuntuMonoNerdFontMono-Bold-b998edcd.ttf"] },
    FontFace { label: "Droid Sans Mono", regular: ("DroidSansM Nerd Font Mono", 400), bold: ("DroidSansM Nerd Font Mono", 700), files: &["DroidSansMNerdFontMono-Regular-0eed7946.otf"] },
    FontFace { label: "Dejavu Sans Mono", regular: ("DejaVuSansM Nerd Font Mono", 400), bold: ("DejaVuSansM Nerd Font Mono", 700), files: &["DejaVuSansMNerdFontMono-Regular-02ad4ecb.ttf", "DejaVuSansMNerdFontMono-Bold-967b90db.ttf"] },
    FontFace { label: "PT Mono", regular: ("PT Mono", 400), bold: ("PT Mono", 700), files: &["PTM55FT-cbe732b3.ttf"] },
    FontFace { label: "Andale Mono", regular: ("Andale Mono", 400), bold: ("Andale Mono", 700), files: &["AndaleMono-48d9bc61.ttf"] },
    FontFace { label: "Cascadia Code", regular: ("CaskaydiaCove Nerd Font Mono", 400), bold: ("CaskaydiaCove Nerd Font Mono", 700), files: &["CaskaydiaCoveNerdFontMono-Regular-f30b07c7.ttf", "CaskaydiaCoveNerdFontMono-Bold-d73327e9.ttf"] },
    FontFace { label: "Fira Code", regular: ("FiraCode Nerd Font", 400), bold: ("FiraCode Nerd Font Mono", 700), files: &["FiraCodeNerdFont-Regular-7e49cee3.ttf", "FiraCodeNerdFontMono-Bold-08b86e12.ttf"] },
    FontFace { label: "JetBrains Mono", regular: ("JetBrainsMono Nerd Font Mono", 400), bold: ("JetBrainsMono Nerd Font Mono", 700), files: &["JetBrainsMonoNerdFontMono-Regular-276cf05f.ttf", "JetBrainsMonoNerdFontMono-Bold-1a606ae3.ttf"] },
    FontFace { label: "Meslo", regular: ("MesloLGS Nerd Font Mono", 400), bold: ("MesloLGS Nerd Font Mono", 700), files: &["MesloLGSNerdFontMono-Regular-7fc866e6.ttf", "MesloLGSNerdFontMono-Bold-cf2b2171.ttf"] },
];

/// A bundled face, or a typed system font name used as it is.
pub fn font_face(label: &str) -> FontFace {
    FONT_FACES.iter().find(|f| f.label.eq_ignore_ascii_case(label)).copied().unwrap_or(FontFace { label: "", regular: ("", 400), bold: ("", 700), files: &[] })
}

/// Where the bundled font files are: `TERMIAN_FONTS_DIR`, the app bundle's
/// `Resources/fonts` or `fonts` next to the EXE, else the legacy assets of a
/// source checkout.
pub fn font_dirs() -> Vec<std::path::PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("TERMIAN_FONTS_DIR") {
        dirs.push(dir.into());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("../Resources/fonts"));
            dirs.push(parent.join("fonts"));
        }
    }
    dirs.push(concat!(env!("CARGO_MANIFEST_DIR"), "/legacy/src/ui/assets").into());
    dirs
}

/// Whether every file of a bundled face can be found.
pub fn font_available(face: &FontFace) -> bool {
    let dirs = font_dirs();
    face.files.iter().all(|name| dirs.iter().any(|dir| dir.join(name).is_file()))
}

/// The files of a face that exist, read.
pub fn font_files(face: &FontFace) -> Vec<Vec<u8>> {
    let dirs = font_dirs();
    face.files.iter().filter_map(|name| dirs.iter().find_map(|dir| std::fs::read(dir.join(name)).ok())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_legacy_reducers() {
        let s = TerminalSettings::default();
        assert!(s.autoreconnect && s.shell_history_import && s.option_is_meta && s.keyword_highlighting && s.detect_os && s.pqc_kex);
        assert!(!s.copy_on_select && !s.bell_sound && !s.brighten_bold);
        assert_eq!((s.emulation.as_str(), s.font_family.as_str(), s.font_size, s.color_scheme.as_str(), s.scrollback), ("xterm-256color", "Source Code Pro", 14, "termius dark", 40_000));
        let old: TerminalSettings = serde_json::from_str(r#"{"font_size": 18}"#).unwrap();
        assert_eq!((old.font_size, old.autoreconnect), (18, true), "missing fields take defaults");
    }

    #[test]
    fn scrollback_and_font_size_obey_the_legacy_fields() {
        let mut s = TerminalSettings::default();
        s.set_scrollback_field(0);
        assert_eq!((s.scrollback, s.scrollback_field(), s.scrollback_lines()), (MAX_SCROLLBACK, 0, SCROLLBACK_LIMIT));
        s.set_scrollback_field(500);
        assert_eq!((s.scrollback_field(), s.scrollback_lines()), (500, 500));
        s.set_font_size(1);
        assert_eq!(s.font_size, MIN_FONT_SIZE);
        s.set_font_size(99);
        assert_eq!(s.font_size, MAX_FONT_SIZE);
        s.emulation = "vt100".into();
        assert_eq!(s.emulation(), "xterm-256color");
    }

    #[test]
    fn keepalive_lives_in_the_snapshot_settings_as_legacy_writes_it() {
        let mut foreign = serde_json::Value::Null;
        assert_eq!(keep_alive(&foreign), 0);
        set_keep_alive(&mut foreign, 45);
        assert_eq!(foreign["settings"][KEEP_ALIVE_KEY], "45");
        assert_eq!(keep_alive(&foreign), 45);
        set_keep_alive(&mut foreign, 0);
        assert!(foreign["settings"].get(KEEP_ALIVE_KEY).is_none());
        assert_eq!(parse_keep_alive("1a2"), 12);
        assert_eq!(parse_keep_alive("99999999999"), i32::MAX as u32);
    }

    #[test]
    fn all_legacy_themes_load() {
        assert_eq!(themes().len(), 64);
        let dark = theme("termius dark");
        assert_eq!((dark.background, dark.foreground, dark.palette[1], dark.selection, dark.selection_alpha), (0x141729, 0x21b568, 0xf24e50, 0x21b568, 0x80));
        assert_eq!(theme("No Such Theme").name, "Termius Dark");
        assert_eq!(theme("TERMIUS LIGHT").name, "Termius Light");
        assert_eq!(parse_color("#abc"), Some((0xaabbcc, 255)));
    }

    #[test]
    fn keywords_are_found_as_the_legacy_rules_find_them() {
        let colors = highlight_colors("termius dark");
        let found = |line: &str| highlights(line, &colors).into_iter().map(|(a, b, _)| line.chars().skip(a).take(b - a).collect::<String>()).collect::<Vec<_>>();
        assert_eq!(found("[ERROR] disk full"), ["[ERROR]"]);
        assert_eq!(found("level warn: low"), [" warn:"]);
        assert_eq!(found("terror and errors"), Vec::<String>::new());
        assert_eq!(found("inet 192.168.1.10/24"), ["192.168.1.10"]);
        assert_eq!(found("v1.2.3.4a"), Vec::<String>::new(), "a version is not an address");
        assert_eq!(found("ether 3c:22:fb:01:aa:9e"), ["3c:22:fb:01:aa:9e"]);
        assert_eq!(found("addr fe80::1:2"), ["fe80::1:2"]);
        assert_eq!(found("cisco 0011.2233.4455"), ["0011.2233.4455"]);
        assert_eq!(highlight_colors("Ocean")[0], 0xFF7679);
        let (start, end, color) = highlights("× [info] ok", &colors)[0];
        assert_eq!((start, end, color), (2, 8, 0x2091F6), "character positions, not bytes");
    }

    #[test]
    fn shells_and_fonts() {
        let mut s = TerminalSettings::default();
        s.add_shell(" /opt/homebrew/bin/fish ");
        s.add_shell("/opt/homebrew/bin/fish");
        assert_eq!((s.custom_shells.len(), s.local_shell.as_deref()), (1, Some("/opt/homebrew/bin/fish")));
        s.remove_shell("/opt/homebrew/bin/fish");
        assert_eq!((s.custom_shells.len(), s.local_shell.clone()), (0, None));
        assert!(!detected_shells().is_empty());
        assert_eq!(FONT_FACES.map(|f| f.label), FONT_FAMILIES);
        assert_eq!(font_face("fira code").bold.0, "FiraCode Nerd Font Mono");
        assert!(font_face("Menlo").files.is_empty());
        assert!(font_available(&font_face("JetBrains Mono")), "the legacy assets of this checkout");
    }
}
