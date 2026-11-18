//! The Saved-cell draft in Logs. Escape drops it; accepting an unchanged
//! cell does not add a bookmark unless its bookmark icon opened the editor.

use std::ops::Range;

use crate::vault::{bounded_log_comment, HistoryEntry, LogRef, Vault};

/// Clickable links in the read-only cell. Never turn file or script schemes
/// into actions; only explicit HTTP(S) URLs are opened by the viewer.
pub fn links(text: &str) -> Vec<(Range<usize>, String)> {
    use std::sync::OnceLock;
    static URL: OnceLock<regex::Regex> = OnceLock::new();
    let pattern = URL.get_or_init(|| regex::Regex::new(r#"https?://[^\s<>\"']+"#).unwrap());
    pattern.find_iter(text).filter_map(|found| {
        let candidate = found.as_str().trim_end_matches(['.', ',', ';', '!', '?']);
        let url = url::Url::parse(candidate).ok()?;
        url.host_str()?;
        Some((found.start()..found.start() + candidate.len(), url.into()))
    }).collect()
}

pub struct Draft {
    pub id: u64,
    log: Option<LogRef>,
    original: String,
    bookmark: bool,
    text: String,
    anchor: usize,
    cursor: usize,
}

impl Draft {
    pub fn new(entry: &HistoryEntry, bookmark: bool) -> Self {
        let text = bounded_log_comment(&entry.comment).to_owned();
        let cursor = text.len();
        Self { id: entry.id, log: entry.log.clone(), original: text.clone(), bookmark, text, anchor: cursor, cursor }
    }

    pub fn text(&self) -> &str { &self.text }
    pub fn cursor(&self) -> usize { self.cursor }
    pub fn selection(&self) -> Range<usize> { self.anchor.min(self.cursor)..self.anchor.max(self.cursor) }

    fn boundary(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) { offset -= 1; }
        offset
    }

    pub fn move_to(&mut self, offset: usize, extend: bool) {
        self.cursor = self.boundary(offset);
        if !extend { self.anchor = self.cursor; }
    }

    pub fn move_horizontal(&mut self, right: bool, extend: bool) {
        let selection = self.selection();
        let offset = if !extend && !selection.is_empty() {
            if right { selection.end } else { selection.start }
        } else if right {
            self.text[self.cursor..].chars().next().map_or(self.cursor, |ch| self.cursor + ch.len_utf8())
        } else {
            self.text[..self.cursor].char_indices().next_back().map_or(0, |(i, _)| i)
        };
        self.move_to(offset, extend);
    }

    pub fn delete(&mut self, backward: bool) {
        if self.selection().is_empty() { self.move_horizontal(!backward, true); }
        self.replace(None, "");
    }

    /// GPUI's platform text input ranges are UTF-16; the draft uses UTF-8.
    pub fn utf16_range(&self, range: Range<usize>) -> Range<usize> {
        self.text[..self.boundary(range.start)].encode_utf16().count()..self.text[..self.boundary(range.end)].encode_utf16().count()
    }

    pub fn byte_range(&self, range: Range<usize>) -> Range<usize> {
        let byte = |offset| {
            let mut units = 0;
            self.text.char_indices().find_map(|(i, ch)| {
                units += ch.len_utf16();
                (units > offset).then_some(i)
            }).unwrap_or(self.text.len())
        };
        byte(range.start)..byte(range.end)
    }

    /// Replaces a UTF-8 range (or the current selection) and leaves the cursor
    /// after the insertion. The suffix is preserved even at the length limit.
    pub fn replace(&mut self, range: Option<Range<usize>>, text: &str) {
        let range = range.unwrap_or_else(|| self.selection());
        let remaining = crate::vault::LOG_COMMENT_LIMIT.saturating_sub(self.text[..range.start].encode_utf16().count() + self.text[range.end..].encode_utf16().count());
        let mut units = 0;
        let end = text.char_indices().find_map(|(i, ch)| {
            units += ch.len_utf16();
            (units > remaining).then_some(i)
        }).unwrap_or(text.len());
        self.text.replace_range(range.clone(), &text[..end]);
        self.cursor = range.start + end;
        self.anchor = self.cursor;
    }

    /// A draft never recreates a connection removed by Settings → Account.
    pub fn save(self, vault: &mut Vault) -> bool {
        let same_log = self.log.is_some() && vault.history.iter().any(|entry| entry.id == self.id && entry.log == self.log);
        same_log && (self.bookmark || self.text != self.original) && vault.save_log_comment(self.id, &self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{HistoryEntry, LogRef, Vault};

    #[test]
    fn a_stale_draft_cannot_annotate_a_different_log_reusing_the_history_id() {
        let mut vault = Vault::default();
        let id = vault.record_connection(HistoryEntry { log: Some(LogRef { name: "old".into(), key: "k".into() }), ..HistoryEntry::default() });
        let mut draft = Draft::new(vault.logs(false)[0], false);
        draft.replace(None, "late edit");
        vault.history.clear();
        let reused = vault.record_connection(HistoryEntry { log: Some(LogRef { name: "new".into(), key: "k".into() }), ..HistoryEntry::default() });
        assert_eq!(reused, id);
        assert!(!draft.save(&mut vault));
        assert_eq!(vault.logs(false)[0].comment, "");
    }

    #[test]
    fn comment_links_only_open_explicit_web_urls() {
        let text = "См. https://example.test/run?q=1 и http://example.test/docs. javascript:alert(1) file:///tmp/secret";
        let links = links(text);
        assert_eq!(links.len(), 2);
        assert_eq!(&text[links[0].0.clone()], "https://example.test/run?q=1");
        assert_eq!(links[0].1, "https://example.test/run?q=1");
        assert_eq!(&text[links[1].0.clone()], "http://example.test/docs");
    }

    #[test]
    fn unicode_selection_replacement_and_deletion_keep_valid_offsets() {
        let mut draft = Draft::new(&HistoryEntry { comment: "я😀z".into(), ..HistoryEntry::default() }, false);
        assert_eq!(draft.utf16_range(draft.selection()), 4..4);
        assert_eq!(draft.utf16_range(1..99), 0..4, "stale hit-test offsets are clamped to scalar boundaries");
        let emoji = draft.byte_range(1..3);
        assert_eq!(&draft.text()[emoji.clone()], "😀");
        draft.replace(Some(emoji), "✅\n");
        assert_eq!(draft.text(), "я✅\nz");
        draft.move_horizontal(false, true);
        assert_eq!(&draft.text()[draft.selection()], "\n");
        draft.delete(true);
        assert_eq!(draft.text(), "я✅z");
        draft.delete(true);
        assert_eq!(draft.text(), "яz");
        draft.delete(false);
        assert_eq!(draft.text(), "я");
        draft.move_to(0, false);
        draft.delete(true);
        assert_eq!(draft.text(), "я");
        draft.move_to(draft.text().len(), true);
        draft.replace(None, "replacement");
        assert_eq!(draft.text(), "replacement");
        let mut full = Draft::new(&HistoryEntry { comment: "a".repeat(2000), ..HistoryEntry::default() }, false);
        full.move_to(0, false);
        full.replace(None, "😀");
        assert_eq!(full.text(), "a".repeat(2000), "a full comment cannot grow");
        full.replace(Some(0..2), "😀x");
        assert!(full.text().starts_with("😀aaa"));
        assert_eq!(full.text().encode_utf16().count(), 2000);
    }

    #[test]
    fn accepting_a_comment_and_cancelling_a_draft_match_the_saved_cell() {
        let mut vault = Vault::default();
        vault.record_connection(HistoryEntry { log: Some(LogRef { name: "a".into(), key: "k".into() }), ..HistoryEntry::default() });
        assert!(!Draft::new(vault.logs(false)[0], false).save(&mut vault), "opening an empty cell alone is not a bookmark");
        assert!(Draft::new(vault.logs(false)[0], true).save(&mut vault), "accepting the bookmark icon allows an empty comment");
        let mut draft = Draft::new(vault.logs(false)[0], false);
        draft.replace(None, "  Выпуск\nпроверен ✅  ");
        assert_eq!(vault.logs(false)[0].comment, "", "typing does not persist a draft");
        assert!(draft.save(&mut vault));
        assert_eq!(vault.logs(false)[0].comment, "Выпуск\nпроверен ✅");
        let mut cancelled = Draft::new(vault.logs(false)[0], false);
        cancelled.replace(None, "discard me");
        drop(cancelled);
        assert_eq!(vault.logs(false)[0].comment, "Выпуск\nпроверен ✅");
        let draft = Draft::new(vault.logs(false)[0], false);
        vault.history.clear();
        assert!(!draft.save(&mut vault), "cleanup cannot be undone by a stale editor");
        assert!(vault.history.is_empty());
    }
}
