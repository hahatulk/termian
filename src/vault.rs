//! Vault contents shown on the Hosts screen: groups (nestable) and hosts.

use serde::{Deserialize, Serialize};

pub type GroupId = u64;
pub type HostId = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub label: String,
    pub parent: Option<GroupId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub label: String,
    pub address: String,
    /// Distribution reported by the server (`debian`, `ubuntu`, ...), drives the icon.
    pub os: Option<String>,
    pub group: Option<GroupId>,
    pub username: Option<String>,
    pub tags: Vec<String>,
    /// SSH port when it is not 22.
    #[serde(default)]
    pub port: Option<u16>,
    /// Saved in the native vault only; never read from the legacy profile.
    #[serde(default)]
    pub password: Option<String>,
}

impl Host {
    /// `address:port` for the SSH target; the address alone on port 22.
    pub fn destination(&self) -> String {
        match self.port {
            Some(port) if port != 22 => format!("{}:{port}", self.address),
            _ => self.address.clone(),
        }
    }

    /// Name on cards and tabs: the label, or the address when there is none.
    pub fn display_label(&self) -> &str {
        if self.label.trim().is_empty() { &self.address } else { &self.label }
    }

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SshKey {
    pub id: u64,
    pub label: String,
    /// Algorithm as shown on the card: `ED25519`, `ECDSA`, `RSA`.
    pub kind: String,
}

impl SshKey {
    /// Card subtitle as in the legacy UI.
    pub fn subtitle(&self) -> String {
        format!("Type {}", self.kind)
    }
}

/// Algorithm label for an OpenSSH public key line (`ssh-ed25519 AAAA...`).
pub fn key_kind(public_key: &str) -> String {
    let algorithm = public_key.split_whitespace().next().unwrap_or("");
    let name = algorithm.trim_start_matches("sk-").trim_start_matches("ssh-");
    if name.starts_with("ecdsa") {
        "ECDSA".into()
    } else if name.is_empty() {
        "Unknown".into()
    } else {
        name.split('@').next().unwrap_or(name).to_uppercase()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Identity {
    pub id: u64,
    pub label: String,
    pub has_password: bool,
    pub has_key: bool,
}

impl Identity {
    pub fn subtitle(&self) -> &'static str {
        match (self.has_password, self.has_key) {
            (true, true) => "Auth password and key",
            (true, false) => "Auth password",
            (false, true) => "Auth key",
            (false, false) => "No credentials",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: u64,
    pub label: String,
    pub script: String,
}

impl Snippet {
    /// The card shows the script under the label only when they differ.
    pub fn subtitle(&self) -> Option<&str> {
        let script = self.script.trim();
        (!script.is_empty() && script != self.label.trim()).then_some(script)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnownHost {
    pub id: u64,
    /// `host` or `[host]:port`, as in OpenSSH known_hosts.
    pub host: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vault {
    pub groups: Vec<Group>,
    pub hosts: Vec<Host>,
    pub keys: Vec<SshKey>,
    pub identities: Vec<Identity>,
    pub snippets: Vec<Snippet>,
    pub known_hosts: Vec<KnownHost>,
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

    /// A free id for a new record of any kind.
    pub fn next_id(&self) -> u64 {
        let ids = self.groups.iter().map(|g| g.id).chain(self.hosts.iter().map(|h| h.id));
        ids.chain(self.keys.iter().map(|k| k.id)).chain(self.identities.iter().map(|i| i.id)).chain(self.snippets.iter().map(|s| s.id)).max().unwrap_or(0) + 1
    }

    pub fn host(&self, id: HostId) -> Option<&Host> {
        self.hosts.iter().find(|h| h.id == id)
    }

    /// Adds the host, or replaces the one with the same id.
    pub fn save_host(&mut self, host: Host) {
        match self.hosts.iter_mut().find(|h| h.id == host.id) {
            Some(existing) => *existing = host,
            None => self.hosts.push(host),
        }
    }

    pub fn remove_host(&mut self, id: HostId) -> Option<Host> {
        let index = self.hosts.iter().position(|h| h.id == id)?;
        Some(self.hosts.remove(index))
    }

    /// Duplicate in the legacy host menu: a copy under a new id, right after the original.
    pub fn duplicate_host(&mut self, id: HostId) -> Option<HostId> {
        let index = self.hosts.iter().position(|h| h.id == id)?;
        let copy = Host { id: self.next_id(), ..self.hosts[index].clone() };
        let new_id = copy.id;
        self.hosts.insert(index + 1, copy);
        Some(new_id)
    }

    /// Tags used by any host, sorted, for the Tags suggestions.
    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.hosts.iter().flat_map(|h| h.tags.iter().cloned()).collect();
        tags.sort_by_key(|t| t.to_lowercase());
        tags.dedup();
        tags
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
            port: None,
            password: None,
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
            ..Default::default()
        }
    }

    fn labels(hosts: Vec<&Host>) -> Vec<&str> {
        hosts.into_iter().map(|h| h.label.as_str()).collect()
    }

    #[test]
    fn hosts_are_saved_duplicated_and_removed() {
        let mut v = vault();
        let id = v.next_id();
        v.save_host(Host { port: Some(2222), ..host(id, "", None) });
        assert_eq!(v.host(id).unwrap().display_label(), format!("10.0.0.{id}"));
        assert_eq!(v.host(id).unwrap().destination(), format!("10.0.0.{id}:2222"));
        let copy = v.duplicate_host(10).unwrap();
        let position = |id| v.hosts.iter().position(|h| h.id == id).unwrap();
        assert_eq!(position(copy), position(10) + 1);
        assert_eq!(v.host(copy).unwrap().label, "Termian");
        assert!(v.remove_host(copy).is_some() && v.host(copy).is_none());
        v.save_host(Host { label: "renamed".into(), ..v.host(10).unwrap().clone() });
        assert_eq!(v.host(10).unwrap().label, "renamed");
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
        let v = Vault { groups: vec![group(1, "a", Some(2)), group(2, "b", Some(1))], hosts: vec![host(5, "h", Some(1))], ..Default::default() };
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
    fn key_kind_reads_the_algorithm() {
        assert_eq!(key_kind("ssh-ed25519 AAAA comment"), "ED25519");
        assert_eq!(key_kind("ecdsa-sha2-nistp256 AAAA"), "ECDSA");
        assert_eq!(key_kind("ssh-rsa AAAA"), "RSA");
        assert_eq!(key_kind("sk-ssh-ed25519@openssh.com AAAA"), "ED25519");
        assert_eq!(key_kind(""), "Unknown");
    }

    #[test]
    fn identity_and_snippet_subtitles() {
        let identity = |has_password, has_key| Identity { id: 1, label: "root".into(), has_password, has_key };
        assert_eq!(identity(true, true).subtitle(), "Auth password and key");
        assert_eq!(identity(false, true).subtitle(), "Auth key");
        let snippet = |label: &str, script: &str| Snippet { id: 1, label: label.into(), script: script.into() };
        assert_eq!(snippet("sudo su", "sudo su").subtitle(), None);
        assert_eq!(snippet("get pods", "kubectl get pods").subtitle(), Some("kubectl get pods"));
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
        port: None,
        password: None,
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
        keys: vec![
            SshKey { id: 1, label: "kazgau".into(), kind: "ED25519".into() },
            SshKey { id: 2, label: "My".into(), kind: "ECDSA".into() },
        ],
        identities: vec![
            Identity { id: 1, label: "root".into(), has_password: false, has_key: true },
            Identity { id: 2, label: "pt".into(), has_password: true, has_key: false },
            Identity { id: 3, label: "kazgau".into(), has_password: true, has_key: true },
        ],
        snippets: vec![
            Snippet { id: 1, label: "sudo su".into(), script: "sudo su".into() },
            Snippet { id: 2, label: "cd /var/docker".into(), script: "cd /var/docker".into() },
            Snippet { id: 3, label: "get pods".into(), script: "kubectl -n default get pods".into() },
        ],
        known_hosts: ["192.168.1.180", "[95.80.105.230]:50703", "10.0.209.12", "[192.168.88.59]:2001"]
            .iter()
            .enumerate()
            .map(|(i, host)| KnownHost { id: i as u64, host: host.to_string() })
            .collect(),
    }
}
