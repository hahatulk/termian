//! The legacy Vaults toolbar controls: «Grid / List» view, sorting
//! («A-z», «Z-a», «Newest to oldest», «Oldest to newest») and the Hosts tag
//! filter. Kept per section on this device, as the legacy `sorting` and
//! layout settings are.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::vault::Host;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewMode {
    #[default]
    Grid,
    List,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    AZ,
    ZA,
    /// The legacy default.
    #[default]
    Newest,
    Oldest,
}

impl SortOrder {
    pub const ALL: [SortOrder; 4] = [SortOrder::AZ, SortOrder::ZA, SortOrder::Newest, SortOrder::Oldest];

    pub fn label(self) -> &'static str {
        match self {
            SortOrder::AZ => "A-z",
            SortOrder::ZA => "Z-a",
            SortOrder::Newest => "Newest to oldest",
            SortOrder::Oldest => "Oldest to newest",
        }
    }

    /// Sorts by label, or by age. `age` grows with newer items: records have
    /// no creation date here, so ids (which grow as records are added) stand
    /// for it, and hosts put their last connection first, as legacy
    /// `hostInteractions` does.
    pub fn sort<T, K: Ord>(self, items: &mut [T], label: impl Fn(&T) -> &str, age: impl Fn(&T) -> K) {
        match self {
            SortOrder::AZ => items.sort_by_cached_key(|i| label(i).to_lowercase()),
            SortOrder::ZA => {
                items.sort_by_cached_key(|i| label(i).to_lowercase());
                items.reverse();
            }
            SortOrder::Newest => items.sort_by_cached_key(|i| std::cmp::Reverse(age(i))),
            SortOrder::Oldest => items.sort_by_cached_key(|i| age(i)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listing {
    #[serde(default)]
    pub view: ViewMode,
    #[serde(default)]
    pub sort: SortOrder,
}

/// Per section name (`hosts`, `keychain`, ...).
pub type ListingSettings = BTreeMap<String, Listing>;

/// A host passes the tag filter when no tag is chosen or it has one of them.
pub fn has_tags(host: &Host, tags: &[String]) -> bool {
    tags.is_empty() || host.tags.iter().any(|t| tags.contains(t))
}

/// Every tag used by a host, once, alphabetically.
pub fn all_tags(hosts: &[Host]) -> Vec<String> {
    let mut tags: Vec<String> = hosts.iter().flat_map(|h| h.tags.iter().cloned()).collect();
    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::tests::host;

    #[test]
    fn sorts_by_name_or_by_age() {
        let names = |order: SortOrder| {
            let mut items = vec![(2, "beta"), (1, "Alpha"), (3, "gamma")];
            order.sort(&mut items, |i| i.1, |i| i.0);
            items.into_iter().map(|i| i.1).collect::<Vec<_>>()
        };
        assert_eq!(names(SortOrder::AZ), ["Alpha", "beta", "gamma"], "case does not matter");
        assert_eq!(names(SortOrder::ZA), ["gamma", "beta", "Alpha"]);
        assert_eq!(names(SortOrder::Newest), ["gamma", "beta", "Alpha"]);
        assert_eq!(names(SortOrder::Oldest), ["Alpha", "beta", "gamma"]);
        assert_eq!(Listing::default(), Listing { view: ViewMode::Grid, sort: SortOrder::Newest });

        // Hosts: the last connection first, then the newest never used.
        let mut hosts = vec![(1, None), (2, Some(50)), (3, None), (4, Some(90))];
        SortOrder::Newest.sort(&mut hosts, |_| "", |h: &(u64, Option<i64>)| (h.1, h.0));
        assert_eq!(hosts.iter().map(|h| h.0).collect::<Vec<_>>(), [4, 2, 3, 1]);
    }

    #[test]
    fn filters_hosts_by_any_chosen_tag() {
        let tagged = |id, tags: &[&str]| Host { tags: tags.iter().map(|t| t.to_string()).collect(), ..host(id, "h", None) };
        let hosts = [tagged(1, &["prod", "web"]), tagged(2, &["test"]), tagged(3, &[])];
        assert_eq!(all_tags(&hosts), ["prod", "test", "web"]);
        let chosen = |tags: &[&str]| hosts.iter().filter(|h| has_tags(h, &tags.iter().map(|t| t.to_string()).collect::<Vec<_>>())).map(|h| h.id).collect::<Vec<_>>();
        assert_eq!(chosen(&[]), [1, 2, 3]);
        assert_eq!(chosen(&["web", "test"]), [1, 2]);
    }
}
