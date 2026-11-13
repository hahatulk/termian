//! Inline autocomplete at the shell prompt (legacy «hydrogen» autocomplete,
//! without its AI and file-system sources): what the user typed since the
//! prompt is matched against Shell History (this host's first) and snippets.

use crate::vault::{HostId, Vault};

/// Suggestions shown at most.
pub const LIMIT: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    History,
    Snippet(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub text: String,
    pub source: Source,
}

impl Suggestion {
    /// What is still to be typed after `typed`.
    pub fn rest<'a>(&'a self, typed: &str) -> &'a str {
        self.text.strip_prefix(typed).unwrap_or("")
    }
}

/// Candidates for one pane, rebuilt when history or snippets change.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Candidates {
    items: Vec<Suggestion>,
}

impl Candidates {
    /// This host's commands, then other commands, then snippet scripts;
    /// newest first, each text once.
    pub fn for_host(vault: &Vault, host: Option<HostId>) -> Self {
        let own = vault.shell_history.iter().filter(|c| host.is_some() && c.host == host);
        let rest = vault.shell_history.iter().filter(|c| host.is_none() || c.host != host);
        let mut items: Vec<Suggestion> = Vec::new();
        for command in own.chain(rest) {
            if !items.iter().any(|s| s.text == command.script) {
                items.push(Suggestion { text: command.script.clone(), source: Source::History });
            }
        }
        for snippet in &vault.snippets {
            let script = snippet.script.trim();
            if !script.is_empty() && !script.contains('\n') && !items.iter().any(|s| s.text == script) {
                items.push(Suggestion { text: script.to_owned(), source: Source::Snippet(snippet.display_label().to_owned()) });
            }
        }
        Self { items }
    }

    /// Commands that start with `typed` and are longer, up to [`LIMIT`].
    /// Nothing for an empty or whitespace-only prefix.
    pub fn matching(&self, typed: &str) -> Vec<Suggestion> {
        if typed.trim().is_empty() {
            return Vec::new();
        }
        self.items.iter().filter(|s| s.text.len() > typed.len() && s.text.starts_with(typed)).take(LIMIT).cloned().collect()
    }
}

/// The line typed since the prompt, as far as the terminal can tell from
/// the keys it sent; `None` once it cannot (cursor keys, completion).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    text: Option<String>,
}

impl Line {
    /// A new prompt: an empty line.
    pub fn reset(&mut self) {
        self.text = Some(String::new());
    }

    /// A command started: nothing is typed at a prompt.
    pub fn stop(&mut self) {
        self.text = None;
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Follows one key: printable text, Backspace, Ctrl+U / Ctrl+C clear;
    /// anything else (arrows, Tab, Ctrl+A …) makes the line unknown.
    pub fn key(&mut self, key: &str, typed: Option<&str>, control: bool) {
        let Some(text) = self.text.as_mut() else { return };
        match (key, control) {
            ("backspace", false) => {
                text.pop();
            }
            ("u" | "c", true) => text.clear(),
            ("enter", _) => self.text = None,
            (_, false) => match typed {
                Some(chars) if !chars.chars().any(char::is_control) => text.push_str(chars),
                _ => self.text = None,
            },
            _ => self.text = None,
        }
    }

    /// Text inserted from a suggestion or a paste.
    pub fn insert(&mut self, inserted: &str) {
        if let Some(text) = self.text.as_mut() {
            if inserted.contains(['\n', '\r']) {
                self.text = None;
            } else {
                text.push_str(inserted);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{demo, ShellCommand};

    fn vault() -> Vault {
        let mut v = demo();
        v.shell_history = vec![
            ShellCommand { script: "git status".into(), completed_at: 3, rating: 1, host: Some(10) },
            ShellCommand { script: "git log --oneline".into(), completed_at: 2, rating: 1, host: Some(11) },
            ShellCommand { script: "git pull".into(), completed_at: 1, rating: 1, host: None },
        ];
        v
    }

    #[test]
    fn this_hosts_history_comes_first_then_others_and_snippets() {
        let candidates = Candidates::for_host(&vault(), Some(11));
        let texts: Vec<String> = candidates.matching("git").into_iter().map(|s| s.text).collect();
        assert_eq!(texts, ["git log --oneline", "git status", "git pull"]);
        let snippets = candidates.matching("kubectl");
        assert_eq!(snippets[0].source, Source::Snippet("get pods".into()));
        assert!(candidates.matching("").is_empty() && candidates.matching("  ").is_empty());
        assert!(candidates.matching("git status").is_empty(), "nothing longer to offer");
        assert_eq!(candidates.matching("git s")[0].rest("git s"), "tatus");
    }

    #[test]
    fn the_typed_line_follows_keys_until_it_cannot() {
        let mut line = Line::default();
        line.key("g", Some("g"), false);
        assert_eq!(line.text(), None, "nothing before a prompt");
        line.reset();
        for c in ["g", "i", "t", "x"] {
            line.key(c, Some(c), false);
        }
        line.key("backspace", None, false);
        assert_eq!(line.text(), Some("git"));
        line.insert(" status");
        assert_eq!(line.text(), Some("git status"));
        line.key("u", None, true);
        assert_eq!(line.text(), Some(""));
        line.key("left", None, false);
        assert_eq!(line.text(), None, "cursor keys lose track");
        line.reset();
        line.insert("a\nb");
        assert_eq!(line.text(), None);
    }
}
