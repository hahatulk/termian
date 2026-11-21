use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::{OnceLock, atomic::{AtomicBool, Ordering}}};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Russian,
}

type Entries = HashMap<&'static str, &'static str>;
static RUSSIAN: AtomicBool = AtomicBool::new(false);
static CATALOG: OnceLock<(Entries, Entries)> = OnceLock::new();

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Russian];

    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Русский",
        }
    }

    pub fn text(self, source: &str) -> &str {
        let (en, ru) = CATALOG.get_or_init(|| {
            let mut en = Entries::new();
            let mut ru = Entries::new();
            for (english, russian) in include_str!("../assets/app/languages.tsv").lines().filter_map(|line| line.split_once('\t')) {
                en.insert(russian, english);
                ru.insert(english, russian);
            }
            (en, ru)
        });
        let catalog = if self == Self::Russian { ru } else { en };
        catalog.get(source).copied().unwrap_or(source)
    }
}

pub fn set(language: Language) {
    RUSSIAN.store(language == Language::Russian, Ordering::Relaxed);
}

pub fn current() -> Language {
    if RUSSIAN.load(Ordering::Relaxed) { Language::Russian } else { Language::English }
}

pub fn text(source: &str) -> &str {
    current().text(source)
}

#[macro_export]
macro_rules! ui_format {
    ($english:literal, $russian:literal $(, $($args:tt)*)?) => {{
        if $crate::language::current() == $crate::language::Language::Russian {
            format!($russian $(, $($args)*)?)
        } else {
            format!($english $(, $($args)*)?)
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_default_and_catalog_works_in_both_directions() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(Language::Russian.text("Settings"), "Настройки");
        assert_eq!(Language::English.text("Настройки"), "Settings");
        assert_eq!(Language::Russian.text("SSH"), "SSH");
    }
}
