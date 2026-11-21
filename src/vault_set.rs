use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};
use uuid::Uuid;

use crate::{cloud, language, store::Store, vault::Vault};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: Uuid,
    name: String,
}

impl VaultEntry {
    pub fn name(&self) -> &str {
        if self.id.is_nil() { language::text(&self.name) } else { &self.name }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Catalog {
    selected: Uuid,
    entries: Vec<VaultEntry>,
}

impl Default for Catalog {
    fn default() -> Self {
        let selected = Uuid::nil();
        Self { selected, entries: vec![VaultEntry { id: selected, name: "Personal".into() }] }
    }
}

pub enum Destination<'a> {
    Existing(Uuid),
    New(&'a str),
}

#[derive(Clone)]
pub struct VaultSet {
    base: Store,
    catalog: Catalog,
}

impl VaultSet {
    pub fn load(base: Store, personal: Vault) -> Result<(Self, Store, Vault), String> {
        let catalog: Catalog = base.at(base.path().with_file_name("vaults.bin"))
            .read_data()?.unwrap_or_default();
        let ids: HashSet<_> = catalog.entries.iter().map(|e| e.id).collect();
        if ids.len() != catalog.entries.len() || !ids.contains(&Uuid::nil()) || !ids.contains(&catalog.selected)
            || catalog.entries.iter().any(|entry| !valid_name(&entry.name)) {
            return Err(language::text("The Vault list is damaged.").into());
        }
        let set = Self { base, catalog };
        let store = set.store_for(set.selected());
        let vault = if set.selected().is_nil() { personal } else { load_saved(&store)? };
        Ok((set, store, vault))
    }

    pub fn entries(&self) -> &[VaultEntry] {
        &self.catalog.entries
    }

    pub fn selected(&self) -> Uuid {
        self.catalog.selected
    }

    pub fn selected_name(&self) -> &str {
        self.entries().iter().find(|e| e.id == self.selected()).map_or("", VaultEntry::name)
    }

    fn store_for(&self, id: Uuid) -> Store {
        if id.is_nil() {
            self.base.clone()
        } else {
            self.base.at(scoped_path(&self.base, id))
        }
    }

    pub fn select(&mut self, current: &Vault, dest: Destination<'_>) -> Result<(Store, Vault), String> {
        let mut catalog = self.catalog.clone();
        let (id, created) = match dest {
            Destination::New(input) => {
                let name = input.trim();
                if !valid_name(name) {
                    return Err(language::text("Enter a Vault name between 1 and 64 characters.").into());
                }
                let folded = name.to_lowercase();
                if catalog.entries.iter().any(|e| e.name.to_lowercase() == folded || e.name().to_lowercase() == folded) {
                    return Err(language::text("A Vault with this name already exists.").into());
                }
                let id = Uuid::new_v4();
                catalog.entries.push(VaultEntry { id, name: name.into() });
                (id, true)
            }
            Destination::Existing(id) if catalog.entries.iter().any(|e| e.id == id) => (id, false),
            Destination::Existing(_) => return Err(language::text("Vault not found.").into()),
        };
        if id == self.selected() {
            return Ok((self.store_for(id), current.clone()));
        }
        let next = self.store_for(id);
        let mut vault = if created { Vault { logs_disabled: current.logs_disabled, ..Vault::default() } } else { load_saved(&next)? };
        self.store_for(self.selected()).save(current)?;
        vault.language = current.language;
        vault.update_channel = current.update_channel;
        vault.update_schedule = current.update_schedule;
        vault.shortcuts = current.shortcuts.clone();
        vault.privacy_password = current.privacy_password.clone();
        next.save(&vault)?;
        catalog.selected = id;
        self.base.at(self.base.path().with_file_name("vaults.bin")).save_data(&catalog)?;
        self.catalog = catalog;
        Ok((next, vault))
    }

    pub fn check_cloud_url(&self, url: &str) -> Result<(), String> {
        for other in self.entries().iter().filter(|entry| entry.id != self.selected()) {
            let saved = load_saved(&self.store_for(other.id))?;
            if saved.cloud.as_ref().is_some_and(|c| cloud::same_snapshot_endpoint(&c.url, url)) {
                return Err(language::text("Choose a cloud address that is not used by another Vault.").into());
            }
        }
        Ok(())
    }
}

fn scoped_path(base: &Store, id: Uuid) -> PathBuf {
    base.path().with_file_name("vaults").join(id.to_string()).join("vault.bin")
}

fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.chars().count()) && !name.chars().any(char::is_control)
}

fn load_saved(store: &Store) -> Result<Vault, String> {
    store.load()?.ok_or_else(|| language::text("The selected Vault file is missing.").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_keeps_records_and_log_paths_in_their_vaults() {
        let dir = tempfile::tempdir().unwrap();
        let base = Store::new(dir.path().join("vault.bin"), [42; 32]);
        let personal = crate::vault::demo();
        let base_path = base.path().to_owned();
        let (mut set, personal_store, personal) = VaultSet::load(base.clone(), personal).unwrap();
        assert_eq!(personal_store.path(), base_path);
        assert!(set.selected().is_nil());

        let (work_store, mut work) = set.select(&personal, Destination::New(" Work ")).unwrap();
        let work_id = set.selected();
        assert_eq!(set.selected_name(), "Work");
        assert!(work.hosts.is_empty() && work.identities.is_empty() && work.keys.is_empty() && work.history.is_empty());
        assert!(work.cloud.is_none());
        assert_ne!(personal_store.logs_dir(), work_store.logs_dir());
        assert_ne!(personal_store.known_hosts_path(), work_store.known_hosts_path());
        assert!(set.select(&work, Destination::New("work")).is_err());
        assert_eq!(set.selected(), work_id);

        let mut host = personal.hosts[0].clone();
        host.label = "Work host".into();
        work.hosts.push(host);
        let (_, restored) = set.select(&work, Destination::Existing(Uuid::nil())).unwrap();
        assert_eq!(restored.hosts, personal.hosts);
        let (_, reopened) = set.select(&restored, Destination::Existing(work_id)).unwrap();
        assert_eq!(reopened.hosts[0].label, "Work host");

        assert!(!std::fs::read(base_path.with_file_name("vaults.bin")).unwrap().windows(4).any(|w| w == b"Work"));
        let (reloaded, _, reopened) = VaultSet::load(base, personal).unwrap();
        assert_eq!(reloaded.selected(), work_id);
        assert_eq!(reopened.hosts[0].label, "Work host");
    }

    #[test]
    fn rejects_reused_cloud_snapshot_and_damaged_destination() {
        let dir = tempfile::tempdir().unwrap();
        let base = Store::new(dir.path().join("vault.bin"), [42; 32]);
        let (mut set, _, mut personal) = VaultSet::load(base.clone(), Vault::default()).unwrap();
        personal.cloud = Some(cloud::CloudConfig { url: "https://sync.invalid/team".into(), ..Default::default() });
        let (work_store, work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        assert!(set.check_cloud_url("https://sync.invalid/team/v1/snapshot/").is_err());
        assert!(set.check_cloud_url("https://sync.invalid/lab").is_ok());

        let (_, personal) = set.select(&work, Destination::Existing(Uuid::nil())).unwrap();
        std::fs::write(work_store.path(), b"broken").unwrap();
        assert!(set.select(&personal, Destination::Existing(work_id)).is_err());
        assert_eq!(set.selected(), Uuid::nil());
        assert_eq!(std::fs::read(work_store.path()).unwrap(), b"broken");
        assert_eq!(VaultSet::load(base, personal).unwrap().0.selected(), Uuid::nil());
    }
}
