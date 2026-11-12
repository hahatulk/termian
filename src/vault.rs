//! Vault contents shown on the Hosts screen: groups (nestable) and hosts.

pub type GroupId = u64;
pub type HostId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: GroupId,
    pub label: String,
    pub parent: Option<GroupId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Host {
    pub id: HostId,
    pub label: String,
    pub address: String,
    /// Distribution reported by the server (`debian`, `ubuntu`, ...), drives the icon.
    pub os: Option<String>,
    pub group: Option<GroupId>,
    pub username: Option<String>,
    pub tags: Vec<String>,
}

impl Host {
    /// Card subtitle as in the legacy UI: protocol, user, then tags.
    pub fn subtitle(&self) -> String {
        std::iter::once("ssh")
            .chain(self.username.as_deref())
            .chain(self.tags.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        [&self.label, &self.address].iter().any(|field| field.to_lowercase().contains(&query))
            || self.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vault {
    pub groups: Vec<Group>,
    pub hosts: Vec<Host>,
}

impl Vault {
    pub fn group(&self, id: GroupId) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Groups directly inside `parent` (`None` is the vault root).
    pub fn child_groups(&self, parent: Option<GroupId>) -> Vec<&Group> {
        self.groups.iter().filter(|g| g.parent == parent).collect()
    }

    /// Hosts in `group` and all of its subgroups; `None` lists every host.
    pub fn hosts_under(&self, group: Option<GroupId>) -> Vec<&Host> {
        self.hosts.iter().filter(|h| group.is_none() || self.is_within(h.group, group)).collect()
    }

    /// Path from the root to `group`, inclusive.
    pub fn breadcrumbs(&self, group: Option<GroupId>) -> Vec<&Group> {
        let mut path = Vec::new();
        let mut current = group.and_then(|id| self.group(id));
        while let Some(g) = current {
            if path.iter().any(|p: &&Group| p.id == g.id) {
                break; // corrupt data with a parent cycle
            }
            path.push(g);
            current = g.parent.and_then(|id| self.group(id));
        }
        path.reverse();
        path
    }

    /// Hosts whose label, address or tags contain `query`, case-insensitively.
    pub fn search(&self, query: &str) -> Vec<&Host> {
        let query = query.trim();
        self.hosts.iter().filter(|h| query.is_empty() || h.matches(query)).collect()
    }

    fn is_within(&self, mut group: Option<GroupId>, ancestor: Option<GroupId>) -> bool {
        for _ in 0..=self.groups.len() {
            if group == ancestor {
                return true;
            }
            group = match group.and_then(|id| self.group(id)) {
                Some(g) => g.parent,
                None => return false,
            };
        }
        false
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn host(id: HostId, label: &str, group: Option<GroupId>) -> Host {
        Host {
            id,
            label: label.into(),
            address: format!("10.0.0.{id}"),
            os: None,
            group,
            username: None,
            tags: vec![],
        }
    }

    fn group(id: GroupId, label: &str, parent: Option<GroupId>) -> Group {
        Group { id, label: label.into(), parent }
    }

    /// work ⊃ Ugoloc; private is separate.
    fn vault() -> Vault {
        Vault {
            groups: vec![group(1, "work", None), group(2, "private", None), group(3, "Ugoloc", Some(1))],
            hosts: vec![
                host(10, "Termian", Some(2)),
                host(11, "Office VPN", Some(1)),
                host(12, "Ugoloc Prod", Some(3)),
                host(13, "loose", None),
            ],
        }
    }

    fn labels(hosts: Vec<&Host>) -> Vec<&str> {
        hosts.into_iter().map(|h| h.label.as_str()).collect()
    }

    #[test]
    fn root_lists_top_level_groups_and_every_host() {
        let v = vault();
        let groups: Vec<_> = v.child_groups(None).iter().map(|g| g.label.as_str()).collect();
        assert_eq!(groups, ["work", "private"]);
        assert_eq!(v.hosts_under(None).len(), 4);
    }

    #[test]
    fn group_lists_hosts_of_its_subgroups_too() {
        let v = vault();
        assert_eq!(labels(v.hosts_under(Some(1))), ["Office VPN", "Ugoloc Prod"]);
        assert_eq!(labels(v.hosts_under(Some(3))), ["Ugoloc Prod"]);
    }

    #[test]
    fn breadcrumbs_run_from_root() {
        let v = vault();
        let path: Vec<_> = v.breadcrumbs(Some(3)).iter().map(|g| g.label.as_str()).collect();
        assert_eq!(path, ["work", "Ugoloc"]);
        assert!(v.breadcrumbs(None).is_empty());
    }

    #[test]
    fn parent_cycles_do_not_hang() {
        let v = Vault { groups: vec![group(1, "a", Some(2)), group(2, "b", Some(1))], hosts: vec![host(5, "h", Some(1))] };
        assert_eq!(v.breadcrumbs(Some(1)).len(), 2);
        assert!(v.hosts_under(Some(99)).is_empty());
    }

    #[test]
    fn search_matches_label_address_and_tags_case_insensitively() {
        let mut v = vault();
        v.hosts[1].tags = vec!["prod".into()];
        assert_eq!(labels(v.search("ugoloc")), ["Ugoloc Prod"]);
        assert_eq!(labels(v.search("10.0.0.13")), ["loose"]);
        assert_eq!(labels(v.search("PROD")), ["Office VPN", "Ugoloc Prod"]);
        assert_eq!(v.search("  ").len(), 4);
    }

    #[test]
    fn subtitle_is_protocol_user_then_tags() {
        let mut h = host(1, "x", None);
        assert_eq!(h.subtitle(), "ssh");
        h.username = Some("root".into());
        h.tags = vec!["prod".into(), "test".into()];
        assert_eq!(h.subtitle(), "ssh, root, prod, test");
    }
}

/// Sample vault mirroring the legacy UI screenshots; used by `TERMIAN_DEMO=1`
/// so the UI can be checked without touching the legacy profile or keychain.
pub fn demo() -> Vault {
    let group = |id, label: &str, parent| Group { id, label: label.into(), parent };
    let host = |id, label: &str, os: &str, group, user: Option<&str>, tags: &[&str]| Host {
        id,
        label: label.into(),
        address: format!("192.0.2.{id}"),
        os: Some(os.into()),
        group,
        username: user.map(Into::into),
        tags: tags.iter().map(|t| t.to_string()).collect(),
    };
    Vault {
        groups: vec![
            group(1, "work", None),
            group(2, "private", None),
            group(3, "Cenix", Some(1)),
            group(4, "Ugoloc", Some(1)),
            group(5, "Mybas", Some(1)),
        ],
        hosts: vec![
            host(10, "Termian", "debian", Some(2), Some("root"), &[]),
            host(11, "home lab", "ubuntu", Some(2), Some("hahatulk"), &[]),
            host(12, "firstvds vpn", "debian", Some(2), Some("hahatulk"), &[]),
            host(13, "Kazgau IB SIEM", "debian", Some(1), None, &["kazgau", "prod"]),
            host(14, "Office VPN", "ubuntu", Some(1), None, &[]),
            host(15, "Office server", "ubuntu", Some(1), Some("hahatulk"), &["prod"]),
            host(16, "Landing", "ubuntu", Some(1), None, &["prod"]),
            host(17, "Cenix Dev", "debian", Some(3), None, &["test"]),
            host(18, "Cenix Dev2", "debian", Some(3), None, &["test"]),
            host(19, "Ugoloc Prod DB", "debian", Some(4), None, &["prod"]),
            host(20, "Ugoloc Prod", "debian", Some(4), None, &["prod"]),
            host(21, "Ugoloc Test", "debian", Some(4), None, &["test"]),
            host(22, "Beget VPN", "ubuntu", None, Some("hahatulk"), &[]),
        ],
    }
}
