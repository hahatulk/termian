use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};
use uuid::Uuid;

use crate::{
    cloud::{self, CloudConfig, Condition},
    language, log_backup, self_cloud,
    store::Store,
    vault::{Host, Vault},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultEntry {
    pub id: Uuid,
    name: String,
}

impl VaultEntry {
    pub fn name(&self) -> &str {
        if self.id.is_nil() {
            language::text(&self.name)
        } else {
            &self.name
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Catalog {
    selected: Uuid,
    entries: Vec<VaultEntry>,
    #[serde(default)]
    cloud: Option<CloudConfig>,
    #[serde(default)]
    previous_connections: Option<BTreeMap<Uuid, CloudConfig>>,
}

impl Default for Catalog {
    fn default() -> Self {
        let selected = Uuid::nil();
        Self {
            selected,
            entries: vec![VaultEntry {
                id: selected,
                name: "Personal".into(),
            }],
            cloud: None,
            previous_connections: None,
        }
    }
}

mod app_settings;
mod host_move;
pub use app_settings::AppSettings;

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
        let catalog: Catalog = base
            .at(base.path().with_file_name("vaults.bin"))
            .read_data()?
            .unwrap_or_default();
        let ids: HashSet<_> = catalog.entries.iter().map(|e| e.id).collect();
        if ids.len() != catalog.entries.len()
            || !ids.contains(&Uuid::nil())
            || !ids.contains(&catalog.selected)
            || catalog.entries.iter().any(|entry| !valid_name(&entry.name))
        {
            return Err(language::text("The Vault list is damaged.").into());
        }
        let mut set = Self { base, catalog };
        let store = set.store_for(set.selected());
        let mut vault = if set.selected().is_nil() {
            personal.clone()
        } else {
            load_saved(&store)?
        };
        set.migrate_cloud(&personal, &vault)?;
        let settings = set.settings_store();
        let settings = match settings.read_data::<AppSettings>()? {
            Some(saved) => saved,
            None => {
                let initial = AppSettings::migrate(&vault);
                settings.save_data(&initial)?;
                initial
            }
        };
        settings.apply(&mut vault);
        set.attach_cloud(set.selected(), &mut vault);
        Ok((set, store, vault))
    }

    pub fn entries(&self) -> &[VaultEntry] {
        &self.catalog.entries
    }

    pub fn selected(&self) -> Uuid {
        self.catalog.selected
    }

    pub fn selected_name(&self) -> &str {
        self.entries()
            .iter()
            .find(|e| e.id == self.selected())
            .map_or("", VaultEntry::name)
    }

    fn store_for(&self, id: Uuid) -> Store {
        if id.is_nil() {
            self.base.clone()
        } else {
            self.base.at(scoped_path(&self.base, id))
        }
    }

    fn settings_store(&self) -> Store {
        self.base
            .at(self.base.path().with_file_name("app-settings.bin"))
    }

    pub fn update_settings(
        &self,
        vault: &mut Vault,
        change: impl FnOnce(&mut AppSettings),
    ) -> Result<(), String> {
        let mut next = AppSettings::from_vault(vault);
        change(&mut next);
        self.settings_store().save_data(&next)?;
        next.apply(vault);
        Ok(())
    }

    pub fn select(
        &mut self,
        current: &Vault,
        dest: Destination<'_>,
    ) -> Result<(Store, Vault), String> {
        let mut catalog = self.catalog.clone();
        let (id, created) = match dest {
            Destination::New(input) => {
                let name = input.trim();
                if !valid_name(name) {
                    return Err(
                        language::text("Enter a Vault name between 1 and 64 characters.").into(),
                    );
                }
                let folded = name.to_lowercase();
                if catalog
                    .entries
                    .iter()
                    .any(|e| e.name.to_lowercase() == folded || e.name().to_lowercase() == folded)
                {
                    return Err(language::text("A Vault with this name already exists.").into());
                }
                let id = Uuid::new_v4();
                catalog.entries.push(VaultEntry {
                    id,
                    name: name.into(),
                });
                (id, true)
            }
            Destination::Existing(id) if catalog.entries.iter().any(|e| e.id == id) => (id, false),
            Destination::Existing(_) => return Err(language::text("Vault not found.").into()),
        };
        if id == self.selected() {
            return Ok((self.store_for(id), current.clone()));
        }
        let next = self.store_for(id);
        let mut vault = if created {
            Vault::default()
        } else {
            load_saved(&next)?
        };
        self.settings_store()
            .read_data::<AppSettings>()?
            .ok_or_else(|| language::text("The app settings file is missing."))?
            .apply(&mut vault);
        self.store_for(self.selected()).save(current)?;
        self.attach_cloud(id, &mut vault);
        next.save(&vault)?;
        catalog.selected = id;
        self.save_catalog(&catalog)?;
        self.catalog = catalog;
        Ok((next, vault))
    }

    pub fn move_host(&self, current: &mut Vault, host: Host, dest: Uuid) -> Result<(), String> {
        if dest == self.selected() || !self.entries().iter().any(|entry| entry.id == dest) {
            return Err(language::text("Choose another Vault.").into());
        }
        if current.host(host.id).is_none() || host.address.trim().is_empty() {
            return Err(language::text("Save the host with an address before moving it.").into());
        }
        if current.secrets_missing {
            return Err(language::text("The source Vault is missing saved credentials.").into());
        }

        let source = self.store_for(self.selected());
        let target = self.store_for(dest);
        let mut destination = load_saved(&target)?;
        let id = host.id;
        host_move::insert(current, &mut destination, host)?;
        let mut remaining = current.clone();
        remaining.remove_host(id);

        target.save(&destination)?;
        if let Err(error) = source.save(&remaining) {
            return Err(format!(
                "{} {error}",
                language::text("The host was saved in the destination Vault, but the source could not be updated.")
            ));
        }
        *current = remaining;
        Ok(())
    }

    pub fn set_cloud(&mut self, vault: &mut Vault, config: CloudConfig) -> Result<(), String> {
        let config = config.scoped(Uuid::nil());
        config.validate()?;
        if self
            .catalog
            .previous_connections
            .as_ref()
            .is_some_and(|connections| {
                connections.iter().any(|(id, old)| {
                    !id.is_nil() && cloud::same_snapshot_endpoint(&old.url, &config.url)
                })
            })
        {
            return Err(language::text("This address stored a separate Vault. Choose the original Personal server or a new server to keep the snapshots separate.").into());
        }
        let mut catalog = self.catalog.clone();
        catalog.cloud = Some(config);
        self.save_catalog(&catalog)?;
        self.catalog = catalog;
        self.attach_cloud(self.selected(), vault);
        Ok(())
    }

    pub fn sync_all(
        &mut self,
        active: &mut Vault,
        running_logs: &HashSet<u64>,
    ) -> Result<String, String> {
        self.sync_catalog()?;
        for entry in self
            .entries()
            .iter()
            .filter(|entry| entry.id != self.selected())
        {
            let sync = || {
                let store = self.store_for(entry.id);
                let mut vault = load_saved(&store)?;
                self.attach_cloud(entry.id, &mut vault);
                sync_records(&mut vault, &store, &HashSet::new())?;
                store.save(&vault)
            };
            sync().map_err(|error| format!("{}: {error}", entry.name()))?;
        }
        self.attach_cloud(self.selected(), active);
        sync_records(active, &self.store_for(self.selected()), running_logs)
    }

    pub fn sync_catalog(&mut self) -> Result<(), String> {
        let config = self.catalog.cloud.as_ref().ok_or_else(|| {
            language::text("Connect a sync server in Account settings.").to_owned()
        })?;
        let client = config.catalog()?;
        let mut conflicts = 0;
        let remote_entries = loop {
            let (remote, can_create) = client
                .fetch_for_sync()
                .map_err(|error| config.redact(&error))?;
            let (mut directory, condition) = match remote {
                Some(remote) => {
                    let directory = CloudCatalog::read(remote.snapshot)?;
                    let revision = remote.revision.map(Condition::Revision);
                    (directory, revision)
                }
                None if can_create => (
                    CloudCatalog {
                        version: 1,
                        entries: Vec::new(),
                    },
                    Some(Condition::Create),
                ),
                None if self.entries().len() == 1 => return Ok(()),
                None => {
                    return Err(language::text(
                        "Update the sync server to use more than one Vault.",
                    )
                    .into())
                }
            };
            let known: HashSet<_> = directory.entries.iter().map(|entry| entry.id).collect();
            directory.entries.extend(
                self.entries()
                    .iter()
                    .filter(|entry| !entry.id.is_nil() && !known.contains(&entry.id))
                    .cloned(),
            );
            if directory.entries.len() == known.len() {
                break directory.entries;
            }
            let condition = condition.ok_or_else(|| {
                language::text("The server did not return the Vault list revision.").to_owned()
            })?;
            let value = serde_json::to_value(&directory).map_err(|error| error.to_string())?;
            match client.upload(&value, condition) {
                Ok(_) => break directory.entries,
                Err(error) if conflicts < 2 && error.contains(cloud::REVISION_CHANGED) => {
                    conflicts += 1
                }
                Err(error) => return Err(config.redact(&error)),
            }
        };
        self.import_entries(remote_entries)
    }

    fn import_entries(&mut self, entries: Vec<VaultEntry>) -> Result<(), String> {
        let mut catalog = self.catalog.clone();
        for entry in entries {
            if catalog.entries.iter().any(|known| known.id == entry.id) {
                continue;
            }
            let store = self.store_for(entry.id);
            if store.load()?.is_none() {
                store.save(&Vault::default())?;
            }
            catalog.entries.push(entry);
        }
        if catalog.entries.len() != self.catalog.entries.len() {
            self.save_catalog(&catalog)?;
            self.catalog = catalog;
        }
        Ok(())
    }

    fn migrate_cloud(&mut self, personal: &Vault, active: &Vault) -> Result<(), String> {
        if self.catalog.previous_connections.is_some() {
            return Ok(());
        }
        let mut previous = BTreeMap::new();
        let mut readable = true;
        if let Some(config) = &personal.cloud {
            previous.insert(Uuid::nil(), config.clone());
        }
        for entry in self.entries().iter().filter(|entry| !entry.id.is_nil()) {
            if entry.id == self.selected() {
                if let Some(config) = &active.cloud {
                    previous.insert(entry.id, config.clone());
                }
            } else {
                match load_saved(&self.store_for(entry.id)) {
                    Ok(vault) => {
                        if let Some(config) = vault.cloud {
                            previous.insert(entry.id, config);
                        }
                    }
                    Err(_) => readable = false,
                }
            }
        }
        let mut catalog = self.catalog.clone();
        if readable && previous.len() == 1 {
            catalog.cloud = previous.get(&Uuid::nil()).cloned();
        }
        catalog.previous_connections = Some(previous);
        self.save_catalog(&catalog)?;
        self.catalog = catalog;
        Ok(())
    }

    fn attach_cloud(&self, id: Uuid, vault: &mut Vault) {
        let scope = self.catalog.cloud.as_ref().map(|config| config.scoped(id));
        if let Some(config) = &scope {
            let changed = vault
                .cloud
                .as_ref()
                .is_some_and(|old| !cloud::same_snapshot_endpoint(&old.url, &config.url))
                || vault
                    .sync_state
                    .as_ref()
                    .is_some_and(|state| state.server != config.sync_key());
            if changed {
                log_backup::reset_catalog(vault);
                vault.sync_state = None;
            }
        }
        vault.cloud = scope;
    }

    fn save_catalog(&self, catalog: &Catalog) -> Result<(), String> {
        self.base
            .at(self.base.path().with_file_name("vaults.bin"))
            .save_data(catalog)
    }
}

#[derive(Serialize, Deserialize)]
struct CloudCatalog {
    version: u32,
    entries: Vec<VaultEntry>,
}

impl CloudCatalog {
    fn read(value: Value) -> Result<Self, String> {
        let bad = || language::text("The cloud Vault list is damaged.").to_owned();
        let directory: Self = serde_json::from_value(value).map_err(|_| bad())?;
        let mut ids = HashSet::new();
        if directory.version != 1
            || directory
                .entries
                .iter()
                .any(|entry| entry.id.is_nil() || !ids.insert(entry.id) || !valid_name(&entry.name))
        {
            return Err(bad());
        }
        Ok(directory)
    }
}

fn sync_records(
    vault: &mut Vault,
    store: &Store,
    running_logs: &HashSet<u64>,
) -> Result<String, String> {
    let notice = self_cloud::prepare_logs(vault, &store.logs_dir(), running_logs);
    self_cloud::sync(vault, &store.cloud_backups()).map(|message| match notice {
        Some(notice) => format!("{message}. {notice}"),
        None => message,
    })
}

fn scoped_path(base: &Store, id: Uuid) -> PathBuf {
    base.path()
        .with_file_name("vaults")
        .join(id.to_string())
        .join("vault.bin")
}

fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.chars().count()) && !name.chars().any(char::is_control)
}

fn load_saved(store: &Store) -> Result<Vault, String> {
    store
        .load()?
        .ok_or_else(|| language::text("The selected Vault file is missing.").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_reassigns_host_and_dependencies_without_switching_vaults() {
        use crate::vault::{
            ForwardingRule, Group, HistoryEntry, Identity, RuleKind, Snippet, SshKey,
        };

        let directory = tempfile::tempdir().unwrap();
        let base = Store::new(directory.path().join("vault.bin"), [42; 32]);
        let mut personal = Vault::default();
        personal.groups.push(Group {
            id: 2,
            label: "Source group".into(),
            parent: None,
        });
        personal.save_key(SshKey {
            id: 3,
            label: "Fixture key".into(),
            certificate: Some("fixture certificate".into()),
            ..SshKey::default()
        });
        personal.save_identity(Identity {
            id: 4,
            label: "Saved identity".into(),
            username: "fixture".into(),
            key: Some(3),
            ..Identity::default()
        });
        personal.save_snippet(Snippet {
            id: 5,
            label: "Startup".into(),
            script: "printf ready".into(),
            targets: vec![9],
            ..Snippet::default()
        });
        let mut host = crate::vault::tests::host(9, "Moved host", Some(2));
        host.identity = Some(4);
        host.key = Some(3);
        host.settings.startup_snippet = Some(5);
        host.settings.ctrl_h_backspace = true;
        personal.save_host(host.clone());
        let mut forwarding = ForwardingRule::new(10, RuleKind::Dynamic);
        forwarding.host = Some(host.id);
        personal.save_rule(forwarding);
        personal.history.push(HistoryEntry {
            id: 1,
            host: Some(host.id),
            label: host.label.clone(),
            ..HistoryEntry::default()
        });
        let mut another = crate::vault::tests::host(11, "Second host", Some(2));
        another.identity = host.identity;
        personal.save_host(another.clone());

        let (mut set, _, _) = VaultSet::load(base.clone(), personal.clone()).unwrap();
        let (work_store, mut work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        work.groups.push(Group {
            id: 2,
            label: "Destination group".into(),
            parent: None,
        });
        work.save_key(SshKey {
            id: 3,
            label: "Fixture key".into(),
            ..SshKey::default()
        });
        work.save_identity(Identity {
            id: 4,
            label: "Saved identity".into(),
            username: "different".into(),
            ..Identity::default()
        });
        let other = crate::vault::tests::host(9, "Unrelated host", Some(2));
        work.save_host(other.clone());
        let (_, mut personal) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();

        host.label = "Updated before move".into();
        set.move_host(&mut personal, host.clone(), work_id).unwrap();

        assert_eq!(set.selected(), Uuid::nil());
        assert!(personal.host(9).is_none());
        assert_eq!(personal.host(another.id), Some(&another));
        assert_eq!(personal.history[0].host, Some(9));
        assert_eq!(personal.rule(10).unwrap().host, None);
        assert!(personal.snippet(5).unwrap().targets.is_empty());
        assert!(personal.identity(4).is_some() && personal.key(3).is_some());
        assert_eq!(load_saved(&base).unwrap(), personal);

        let work = load_saved(&work_store).unwrap();
        assert_eq!(work.host(9), Some(&other));
        let moved = work.hosts.iter().find(|h| h.label == host.label).unwrap();
        assert_ne!(moved.id, host.id);
        assert_eq!(moved.group, None);
        let key = work.key(moved.key.unwrap()).unwrap();
        assert_ne!(key.id, 3);
        assert_eq!(key.certificate, personal.key(3).unwrap().certificate);
        let identity = work.identity(moved.identity.unwrap()).unwrap();
        assert_ne!(identity.id, 4);
        assert_eq!(identity.username, "fixture");
        assert_eq!(identity.key, Some(key.id));
        let startup = work
            .snippet(moved.settings.startup_snippet.unwrap())
            .unwrap();
        assert_eq!(startup.script, "printf ready");
        assert!(startup.targets.is_empty());
        assert!(moved.settings.ctrl_h_backspace);
        assert!(work.history.is_empty());

        let (mut set, _, mut work) = VaultSet::load(base.clone(), personal.clone())
            .and_then(|(mut set, _, personal)| {
                let (store, work) = set.select(&personal, Destination::Existing(work_id))?;
                Ok((set, store, work))
            })
            .unwrap();
        let moved = work.host(moved.id).unwrap().clone();
        set.move_host(&mut work, moved, Uuid::nil()).unwrap();
        let (_, back) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        let returned = back.hosts.iter().find(|h| h.label == host.label).unwrap();
        assert!(returned.id > another.id);
        assert_eq!(returned.key, Some(3));
        assert_eq!(returned.identity, Some(4));
        assert_eq!(returned.settings.startup_snippet, Some(5));
        assert_eq!(back.keys.len(), 1);
        assert_eq!(back.identities.len(), 1);
        assert_eq!(back.snippets.len(), 1);
    }

    #[test]
    fn move_preserves_a_typed_startup_command() {
        let directory = tempfile::tempdir().unwrap();
        let base = Store::new(directory.path().join("vault.bin"), [42; 32]);
        let mut personal = Vault::default();
        let mut host = crate::vault::tests::host(9, "Moved host", None);
        let command = "printf ready\npwd";
        host.settings.set_startup_command(command.into());
        personal.save_host(host.clone());
        let (mut set, _, _) = VaultSet::load(base, personal.clone()).unwrap();
        let (destination, work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        let (_, mut personal) = set.select(&work, Destination::Existing(Uuid::nil())).unwrap();

        set.move_host(&mut personal, host, work_id).unwrap();

        let moved = load_saved(&destination).unwrap();
        assert!(moved.snippets.is_empty());
        assert_eq!(moved.hosts[0].settings.ssh_startup(&moved, None).as_deref(), Some(command));
        assert!(personal.hosts.is_empty());
    }

    #[test]
    fn move_keeps_source_when_destination_cannot_be_loaded() {
        let directory = tempfile::tempdir().unwrap();
        let base = Store::new(directory.path().join("vault.bin"), [42; 32]);
        let host = crate::vault::tests::host(9, "Retained", None);
        let personal = Vault {
            hosts: vec![host.clone()],
            ..Vault::default()
        };
        let (mut set, _, _) = VaultSet::load(base.clone(), personal.clone()).unwrap();
        let (work_store, work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        let (_, mut personal) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        std::fs::write(work_store.path(), b"damaged fixture").unwrap();

        assert!(set.move_host(&mut personal, host.clone(), work_id).is_err());
        assert_eq!(personal.host(9), Some(&host));
        assert_eq!(load_saved(&base).unwrap(), personal);
        assert_eq!(
            std::fs::read(work_store.path()).unwrap(),
            b"damaged fixture"
        );
    }

    #[test]
    fn move_keeps_destination_copy_if_source_save_fails() {
        let directory = tempfile::tempdir().unwrap();
        let base = Store::new(directory.path().join("vault.bin"), [42; 32]);
        let host = crate::vault::tests::host(9, "Retained", None);
        let personal = Vault {
            hosts: vec![host.clone()],
            ..Vault::default()
        };
        let (mut set, _, _) = VaultSet::load(base.clone(), personal.clone()).unwrap();
        let (work_store, work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        let (_, mut personal) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        let preserved = base.path().with_file_name("preserved.bin");
        std::fs::rename(base.path(), &preserved).unwrap();
        std::fs::create_dir(base.path()).unwrap();

        assert!(set.move_host(&mut personal, host.clone(), work_id).is_err());
        assert_eq!(personal.host(9), Some(&host));
        assert_eq!(load_saved(&base.at(preserved)).unwrap(), personal);
        let work = load_saved(&work_store).unwrap();
        assert_eq!(work.hosts.len(), 1);
        assert_eq!(work.hosts[0].address, host.address);
    }

    #[test]
    fn settings_are_shared_across_vaults_and_restarts() {
        use crate::{
            listing::{Listing, SortOrder, ViewMode},
            vault::{HistoryEntry, Identity},
        };

        let dir = tempfile::tempdir().unwrap();
        let base = Store::new(dir.path().join("vault.bin"), [42; 32]);
        let work_id = Uuid::new_v4();
        let mut catalog = Catalog::default();
        catalog.entries.push(VaultEntry {
            id: work_id,
            name: "Work".into(),
        });
        catalog.selected = work_id;
        base.at(base.path().with_file_name("vaults.bin"))
            .save_data(&catalog)
            .unwrap();

        let mut personal = one_host("Personal host");
        personal.terminal.color_scheme = "dracula".into();
        personal.identities.push(Identity {
            id: 2,
            label: "Personal identity".into(),
            ..Identity::default()
        });
        personal.history.push(HistoryEntry {
            id: 1,
            label: "Personal session".into(),
            ..HistoryEntry::default()
        });
        base.save(&personal).unwrap();

        let mut work = one_host("Work host");
        work.terminal.color_scheme = "nord".into();
        work.terminal.font_size = 17;
        work.foreign = serde_json::json!({"settings": {"keep_alive_interval": "45"}});
        let work_store = base.at(scoped_path(&base, work_id));
        work_store.save(&work).unwrap();

        let (mut set, _, mut work) = VaultSet::load(base.clone(), personal.clone()).unwrap();
        assert_eq!(
            (
                work.terminal.color_scheme.as_str(),
                work.terminal.font_size,
                work.terminal.keep_alive
            ),
            ("nord", 17, 45)
        );
        let initial_vault_file = std::fs::read(work_store.path()).unwrap();
        let layout = Listing {
            view: ViewMode::List,
            sort: SortOrder::AZ,
        };
        set.update_settings(&mut work, |vault| {
            vault.terminal.color_scheme = "monokai".into();
            vault.terminal.font_family = "Fira Code".into();
            vault.terminal.font_size = 21;
            vault.terminal.keep_alive = 0;
            vault.listing.insert("hosts".into(), layout);
            vault.language = crate::language::Language::Russian;
            vault.logs_disabled = true;
            vault.shortcuts.current = crate::shortcuts::Current::Disabled;
        })
        .unwrap();
        assert_eq!(
            std::fs::read(work_store.path()).unwrap(),
            initial_vault_file
        );
        let (_, _, running) = VaultSet::load(base.clone(), personal.clone()).unwrap();
        assert_eq!(
            running.terminal, work.terminal,
            "editing settings persists even before switching"
        );

        let (_, restored) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        assert_eq!(restored.hosts, personal.hosts);
        assert_eq!(restored.history, personal.history);
        assert_eq!(restored.identities, personal.identities);
        assert_eq!(restored.terminal, work.terminal);
        assert_eq!(restored.listing.get("hosts"), Some(&layout));
        assert_eq!(restored.language, crate::language::Language::Russian);
        assert!(restored.logs_disabled);
        assert_eq!(restored.shortcuts, work.shortcuts);

        let (_, reopened) = set
            .select(&restored, Destination::Existing(work_id))
            .unwrap();
        assert_eq!(reopened.terminal.keep_alive, 0);
        assert_eq!(
            crate::session::SshOptions::from_vault(&reopened).keepalive,
            0
        );
        assert_eq!(reopened.foreign["settings"]["keep_alive_interval"], "45");
        assert_eq!(reopened.hosts, work.hosts);
        assert!(reopened.history.is_empty() && reopened.identities.is_empty());
        let (_, blank) = set.select(&reopened, Destination::New("New")).unwrap();
        assert!(blank.hosts.is_empty() && blank.history.is_empty() && blank.identities.is_empty());
        assert_eq!(blank.terminal, work.terminal);
        assert_eq!(blank.listing.get("hosts"), Some(&layout));
    }

    #[test]
    fn failed_settings_write_leaves_vault_and_active_settings_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let base = Store::new(dir.path().join("vault.bin"), [42; 32]);
        let personal = one_host("Only host");
        base.save(&personal).unwrap();
        let (set, _, mut active) = VaultSet::load(base.clone(), personal.clone()).unwrap();
        let settings = set.settings_store();
        let preserved = settings.path().with_file_name("preserved-settings.bin");
        let original = std::fs::read(base.path()).unwrap();
        std::fs::rename(settings.path(), &preserved).unwrap();
        std::fs::create_dir(settings.path()).unwrap();
        assert!(set
            .update_settings(&mut active, |vault| vault.terminal.font_size = 21)
            .is_err());
        assert_eq!(active.terminal, personal.terminal);
        assert_eq!(std::fs::read(base.path()).unwrap(), original);
        assert!(VaultSet::load(base, personal).is_err());
        assert!(settings.path().is_dir());
    }

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
        assert!(
            work.hosts.is_empty()
                && work.identities.is_empty()
                && work.keys.is_empty()
                && work.history.is_empty()
        );
        assert!(work.cloud.is_none());
        assert_ne!(personal_store.logs_dir(), work_store.logs_dir());
        assert_ne!(
            personal_store.known_hosts_path(),
            work_store.known_hosts_path()
        );
        assert!(set.select(&work, Destination::New("work")).is_err());
        assert_eq!(set.selected(), work_id);

        let mut host = personal.hosts[0].clone();
        host.label = "Work host".into();
        work.hosts.push(host);
        let (_, restored) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        assert_eq!(restored.hosts, personal.hosts);
        let (_, reopened) = set
            .select(&restored, Destination::Existing(work_id))
            .unwrap();
        assert_eq!(reopened.hosts[0].label, "Work host");

        assert!(!std::fs::read(base_path.with_file_name("vaults.bin"))
            .unwrap()
            .windows(4)
            .any(|w| w == b"Work"));
        let (reloaded, _, reopened) = VaultSet::load(base, personal).unwrap();
        assert_eq!(reloaded.selected(), work_id);
        assert_eq!(reopened.hosts[0].label, "Work host");
    }

    #[test]
    fn rejects_a_damaged_destination_without_replacing_it() {
        let dir = tempfile::tempdir().unwrap();
        let base = Store::new(dir.path().join("vault.bin"), [42; 32]);
        let (mut set, _, personal) = VaultSet::load(base.clone(), Vault::default()).unwrap();
        let (work_store, work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        let (_, personal) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        std::fs::write(work_store.path(), b"broken").unwrap();
        assert!(set
            .select(&personal, Destination::Existing(work_id))
            .is_err());
        assert_eq!(set.selected(), Uuid::nil());
        assert_eq!(std::fs::read(work_store.path()).unwrap(), b"broken");
        assert_eq!(
            VaultSet::load(base, personal).unwrap().0.selected(),
            Uuid::nil()
        );
    }

    #[test]
    fn migration_keeps_old_configs_and_requires_one_server_when_they_differ() {
        let dir = tempfile::tempdir().unwrap();
        let base = Store::new(dir.path().join("vault.bin"), [42; 32]);
        let work = Uuid::new_v4();
        let mut catalog = Catalog::default();
        catalog.entries.push(VaultEntry {
            id: work,
            name: "Work".into(),
        });
        base.at(base.path().with_file_name("vaults.bin"))
            .save_data(&catalog)
            .unwrap();
        let personal_config = cloud::tests::config("https://sync.invalid/personal");
        let work_config = cloud::tests::config("https://sync.invalid/work");
        let personal = Vault {
            cloud: Some(personal_config.clone()),
            ..Vault::default()
        };
        base.at(scoped_path(&base, work))
            .save(&Vault {
                cloud: Some(work_config.clone()),
                ..Vault::default()
            })
            .unwrap();
        let (mut set, _, mut personal) = VaultSet::load(base, personal).unwrap();
        assert!(personal.cloud.is_none());
        assert_eq!(set.catalog.previous_connections.as_ref().unwrap().len(), 2);
        assert!(set.set_cloud(&mut personal, work_config).is_err());
        set.set_cloud(&mut personal, personal_config.clone())
            .unwrap();
        let (store, work_vault) = set.select(&personal, Destination::Existing(work)).unwrap();
        assert_eq!(work_vault.cloud, Some(personal_config.scoped(work)));
        assert!(store.load().unwrap().unwrap().cloud.is_none());
    }

    fn one_host(label: &str) -> Vault {
        let mut vault = Vault::default();
        let mut host = crate::vault::demo().hosts.remove(0);
        host.label = label.into();
        host.group = None;
        host.key = None;
        host.identity = None;
        vault.hosts.push(host);
        vault
    }

    #[test]
    fn one_server_syncs_active_and_inactive_vaults_and_discovers_them_on_another_device() {
        let (url, _) = cloud::tests::server();
        let config = cloud::tests::config(&url);
        let first = tempfile::tempdir().unwrap();
        let base = Store::new(first.path().join("vault.bin"), [42; 32]);
        let mut personal = one_host("Personal host");
        personal.cloud = Some(config.clone());
        self_cloud::sync(&mut personal, &base.cloud_backups()).unwrap();
        let (mut set, _, personal) = VaultSet::load(base, personal).unwrap();
        let (_, mut work) = set.select(&personal, Destination::New("Work")).unwrap();
        let work_id = set.selected();
        work.hosts = one_host("Work host").hosts;
        let key = work.next_id();
        work.save_key(crate::vault::SshKey {
            id: key,
            label: "Fixture key".into(),
            private_key: Some("fixture private material".into()),
            public_key: Some("ssh-ed25519 fixture".into()),
            ..Default::default()
        });
        let identity = work.next_id();
        work.save_identity(crate::vault::Identity {
            id: identity,
            label: "Deployment".into(),
            username: "service".into(),
            password: Some("fixture password".into()),
            key: Some(key),
            ..Default::default()
        });
        work.hosts[0].identity = Some(identity);
        let (_, mut personal) = set
            .select(&work, Destination::Existing(Uuid::nil()))
            .unwrap();
        set.sync_all(&mut personal, &HashSet::new()).unwrap();

        let second = tempfile::tempdir().unwrap();
        let base = Store::new(second.path().join("vault.bin"), [17; 32]);
        let (mut other, _, mut personal) = VaultSet::load(base, Vault::default()).unwrap();
        other.set_cloud(&mut personal, config.clone()).unwrap();
        other.sync_all(&mut personal, &HashSet::new()).unwrap();
        assert_eq!(other.selected(), Uuid::nil());
        assert_eq!(other.entries().len(), 2);
        assert_eq!(
            personal
                .hosts
                .iter()
                .map(|host| host.label.as_str())
                .collect::<Vec<_>>(),
            ["Personal host"]
        );
        let ticket = self_cloud::checked_preview(&personal).unwrap().1.unwrap();
        let (work_store, mut work) = other
            .select(&personal, Destination::Existing(work_id))
            .unwrap();
        let identity = work.hosts[0].identity.unwrap();
        for _ in 0..3 {
            other.sync_all(&mut work, &HashSet::new()).unwrap();
            assert_eq!(self_cloud::checked_preview(&work).unwrap().0, self_cloud::Preview::Synced);
            work_store.save(&work).unwrap();
            work = load_saved(&work_store).unwrap();
            other.attach_cloud(work_id, &mut work);
            assert_eq!((work.identities.len(), work.keys.len()), (1, 1));
            assert_eq!(work.hosts[0].identity, Some(identity));
            let (username, password, key) = work.credentials(&work.hosts[0]);
            assert_eq!(username.as_deref(), Some("service"));
            assert!(password.as_deref() == Some("fixture password"));
            assert!(key.unwrap().private_key.as_deref() == Some("fixture private material"));
        }
        assert_eq!(
            work.hosts
                .iter()
                .map(|host| host.label.as_str())
                .collect::<Vec<_>>(),
            ["Work host"]
        );
        assert_eq!(work.cloud, Some(config.scoped(work_id)));
        assert!(self_cloud::upload_checked(&mut work, ticket)
            .unwrap_err()
            .contains(cloud::REVISION_CHANGED));
    }
}
