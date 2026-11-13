//! Command palette (⌘K) results, grouped as in the legacy palette:
//! Hosts, Tabs, then Quick connect.

use crate::tabs::{Tab, TabId};
use crate::vault::{HostId, Vault};

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Host { id: HostId, title: String, os: Option<String> },
    Tab { id: TabId, title: String },
    /// Connect to what was typed, as `user@host[:port]`.
    QuickConnect(String),
    LocalTerminal,
}

impl Item {
    pub fn title(&self) -> &str {
        match self {
            Item::Host { title, .. } | Item::Tab { title, .. } | Item::QuickConnect(title) => title,
            Item::LocalTerminal => "Local Terminal",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub title: &'static str,
    pub items: Vec<Item>,
}

/// Sections with at least one item, in the legacy order.
pub fn sections(vault: &Vault, tabs: &[Tab], query: &str) -> Vec<Section> {
    let query = query.trim();
    let lower = query.to_lowercase();
    let hosts = vault
        .search(query)
        .into_iter()
        .map(|h| Item::Host { id: h.id, title: h.display_label().to_owned(), os: h.os.clone() })
        .collect();
    let tabs = tabs
        .iter()
        .filter(|t| lower.is_empty() || t.title().to_lowercase().contains(&lower))
        .map(|t| Item::Tab { id: t.id, title: t.title().to_owned() })
        .collect();
    let mut quick = Vec::new();
    if !query.is_empty() {
        quick.push(Item::QuickConnect(query.to_owned()));
    }
    quick.push(Item::LocalTerminal);
    [("Hosts", hosts), ("Tabs", tabs), ("Quick connect", quick)]
        .into_iter()
        .filter(|(_, items): &(_, Vec<Item>)| !items.is_empty())
        .map(|(title, items)| Section { title, items })
        .collect()
}

/// All items in display order, for arrow-key selection.
pub fn flatten(sections: &[Section]) -> Vec<&Item> {
    sections.iter().flat_map(|s| s.items.iter()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::{TabKind, Tabs};

    #[test]
    fn groups_hosts_tabs_and_quick_connect() {
        let vault = crate::vault::demo();
        let mut tabs = Tabs::default();
        tabs.open(TabKind::Session { title: "Office server".into() });
        let found = sections(&vault, tabs.all(), "office");
        let titles: Vec<_> = found.iter().map(|s| s.title).collect();
        assert_eq!(titles, ["Hosts", "Tabs", "Quick connect"]);
        assert_eq!(found[0].items.len(), 2);
        assert_eq!(found[1].items, [Item::Tab { id: tabs.all()[2].id, title: "Office server".into() }]);
        assert_eq!(found[2].items, [Item::QuickConnect("office".into()), Item::LocalTerminal]);
        assert_eq!(flatten(&found).len(), 5);
    }

    #[test]
    fn an_empty_query_lists_everything_without_quick_connect() {
        let vault = crate::vault::demo();
        let tabs = Tabs::default();
        let found = sections(&vault, tabs.all(), "  ");
        assert_eq!(found[0].items.len(), vault.hosts.len());
        assert_eq!(found[1].items.len(), 2, "Vaults and SFTP");
        assert_eq!(found[2].items, [Item::LocalTerminal]);
        let none = sections(&vault, tabs.all(), "zzzz");
        assert_eq!(none.iter().map(|s| s.title).collect::<Vec<_>>(), ["Quick connect"]);
    }
}
