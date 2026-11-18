//! Snippets screen rules from the legacy client: the remove confirmation,
//! the Package field's choices and the «Add targets» host picker.

use crate::vault::{GroupId, HostId, Vault};

/// «Run» is enabled for 1 to 100 targets, as in legacy.
pub const MAX_TARGETS: usize = 100;

/// «Action description» is at most 50 characters.
pub const LABEL_LIMIT: usize = 50;

/// Legacy `formatEntitiesCounts` / `…WithNumbers`: «package and snippets»,
/// «1 package and 3 snippets».
fn counts(parts: &[(usize, &str, &str)], numbers: bool) -> String {
    parts
        .iter()
        .filter(|(count, ..)| *count != 0)
        .map(|(count, singular, plural)| {
            let noun = if *count > 1 { plural } else { singular };
            if numbers { format!("{count} {noun}") } else { (*noun).to_owned() }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// Title and text of «Remove …» for packages and snippets; a package's own
/// snippets count with it.
pub fn remove_texts(vault: &Vault, packages: &[u64], snippets: &[u64]) -> (String, String) {
    let inside: usize = packages.iter().map(|p| vault.package_snippet_count(*p)).sum();
    let outside = snippets.iter().filter(|s| vault.snippet(**s).is_none_or(|s| s.package.is_none_or(|p| !packages.contains(&p)))).count();
    let parts = [(packages.len(), "package", "packages"), (outside + inside, "snippet", "snippets")];
    (format!("Remove {}", counts(&parts, false)), format!("Are you sure you want to remove {}?", counts(&parts, true)))
}

/// A row of the Package list.
#[derive(Clone, Debug, PartialEq)]
pub enum PackageChoice {
    Existing(u64, String),
    /// `Create "name"`: typing a new name creates the package.
    Create(String),
}

/// Packages whose name contains `query`, then «Create» for a new name.
pub fn package_choices(vault: &Vault, query: &str) -> Vec<PackageChoice> {
    let query = query.trim();
    let needle = query.to_lowercase();
    let mut out: Vec<PackageChoice> = vault
        .packages
        .iter()
        .filter(|p| p.label.to_lowercase().contains(&needle))
        .map(|p| PackageChoice::Existing(p.id, p.label.clone()))
        .collect();
    if !query.is_empty() && !vault.packages.iter().any(|p| p.label == query) {
        out.push(PackageChoice::Create(query.to_owned()));
    }
    out
}

/// How much of a group the picked hosts cover: the legacy group checkbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    None,
    Some,
    All,
}

pub fn coverage(vault: &Vault, group: GroupId, picked: &[HostId]) -> Coverage {
    let hosts = vault.hosts_under(Some(group));
    let count = hosts.iter().filter(|h| picked.contains(&h.id)).count();
    match count {
        0 => Coverage::None,
        n if n == hosts.len() => Coverage::All,
        _ => Coverage::Some,
    }
}

/// Clicking a group's checkbox: a group with any picked host is cleared,
/// otherwise every host in it (and its subgroups) is picked.
pub fn toggle_group(vault: &Vault, group: GroupId, picked: &mut Vec<HostId>) {
    let hosts: Vec<HostId> = vault.hosts_under(Some(group)).iter().map(|h| h.id).collect();
    if coverage(vault, group, picked) == Coverage::None {
        picked.extend(hosts.into_iter().filter(|h| !picked.contains(h)).collect::<Vec<_>>());
    } else {
        picked.retain(|h| !hosts.contains(h));
    }
}

pub fn toggle_host(host: HostId, picked: &mut Vec<HostId>) {
    match picked.iter().position(|h| *h == host) {
        Some(index) => {
            picked.remove(index);
        }
        None => picked.push(host),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::demo;

    #[test]
    fn remove_texts_count_a_packages_snippets() {
        let vault = demo();
        assert_eq!(remove_texts(&vault, &[], &[1]), ("Remove snippet".into(), "Are you sure you want to remove 1 snippet?".into()));
        assert_eq!(remove_texts(&vault, &[], &[1, 2]).1, "Are you sure you want to remove 2 snippets?");
        assert_eq!(
            remove_texts(&vault, &[30], &[]),
            ("Remove package and snippet".into(), "Are you sure you want to remove 1 package and 1 snippet?".into())
        );
        assert_eq!(remove_texts(&vault, &[30], &[3]).1, "Are you sure you want to remove 1 package and 1 snippet?", "counted once");
    }

    #[test]
    fn package_choices_offer_to_create_a_new_name() {
        let vault = demo();
        assert_eq!(package_choices(&vault, ""), [PackageChoice::Existing(30, "Kubernetes".into())]);
        assert_eq!(package_choices(&vault, "kube"), [PackageChoice::Existing(30, "Kubernetes".into()), PackageChoice::Create("kube".into())]);
        assert_eq!(package_choices(&vault, "Kubernetes"), [PackageChoice::Existing(30, "Kubernetes".into())]);
        assert_eq!(package_choices(&vault, " tools "), [PackageChoice::Create("tools".into())]);
    }

    #[test]
    fn group_checkboxes_pick_and_clear_every_host_inside() {
        let vault = demo();
        let ugoloc: Vec<HostId> = vault.hosts_under(Some(4)).iter().map(|h| h.id).collect();
        let mut picked = vec![];
        toggle_group(&vault, 4, &mut picked);
        assert_eq!(picked, ugoloc);
        assert_eq!(coverage(&vault, 4, &picked), Coverage::All);
        toggle_host(ugoloc[0], &mut picked);
        assert_eq!(coverage(&vault, 4, &picked), Coverage::Some);
        toggle_group(&vault, 4, &mut picked);
        assert!(picked.is_empty(), "a partly picked group is cleared");
        assert_eq!(coverage(&vault, 4, &picked), Coverage::None);
    }
}
