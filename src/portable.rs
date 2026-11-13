//! The native vault as a portable snapshot (`termius-portable-snapshot` v1,
//! see legacy/src/self-cloud-adapter.cjs) and back, for own-cloud sync.
//!
//! Records keep the JSON they last arrived with (`Vault::portable`), and the
//! fields this client models are laid over it, so settings it does not know
//! (env variables, host chains, certificates, …) survive a round trip.
//! Collections it does not model at all live in `Vault::foreign`.
//!
//! A host's credentials are stored the Termius way: the host points to an
//! SSH config, which points to an identity — a Keychain identity, or a hidden
//! (`is_visible: false`) one holding the host's own username and password.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

use crate::sync::{empty_snapshot, Device, KINDS};
use crate::vault::{ForwardingRule, Group, Host, Identity, RuleKind, Snippet, SshKey, Vault};

/// Collections kept as they are.
const FOREIGN: [&str; 4] = ["snippetPackages", "telnetConfigs", "proxies", "workspaceTemplates"];

fn key(kind: &str, id: u64) -> String {
    format!("{kind}|{id}")
}

/// The record's stored JSON (or an empty object) with `fields` laid over it.
fn overlay(vault: &Vault, stored: &str, fields: Value) -> Value {
    let mut out = vault.portable.get(stored).cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
    for (name, value) in fields.as_object().into_iter().flatten() {
        out[name] = value.clone();
    }
    out
}

fn reference(id: Option<u64>) -> Value {
    id.map_or(Value::Null, |id| json!({"local_id": id}))
}

fn text(value: &Value, field: &str) -> String {
    value.get(field).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn optional(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn id_of(value: &Value, field: &str) -> Option<Value> {
    value.get(field).and_then(|v| v.get("local_id")).filter(|v| !v.is_null()).cloned()
}

fn rule_type(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Local => "Local Rule",
        RuleKind::Remote => "Remote Rule",
        RuleKind::Dynamic => "Dynamic Rule",
    }
}

/// Ids of the SSH config and hidden identity behind a host, kept with the host.
struct HostLinks {
    config: u64,
    identity: u64,
}

fn host_links(vault: &Vault, host: u64, next: &mut u64) -> HostLinks {
    let mut stored = |name: &str| {
        vault.portable.get(&key(name, host)).and_then(|v| v.get("local_id")).and_then(Value::as_u64).unwrap_or_else(|| {
            *next += 1;
            *next
        })
    };
    HostLinks { config: stored("sshConfigs-of-host"), identity: stored("identities-of-host") }
}

/// The vault as a portable snapshot with native ids as `local_id`.
pub fn export(vault: &Vault, now: &str) -> Value {
    let mut out = empty_snapshot(now);
    // Ids for configs and hidden identities that have none yet: above every native id.
    let mut next = vault.next_id() + 1_000_000;

    out["sshKeys"] = vault
        .keys
        .iter()
        .filter(|k| k.private_key.is_some() || vault.portable.contains_key(&key("sshKeys", k.id)))
        .map(|k| {
            overlay(vault, &key("sshKeys", k.id), json!({
                "local_id": k.id,
                "label": k.label,
                "private_key": k.private_key.clone().unwrap_or_default(),
                "public_key": k.public_key.clone().unwrap_or_default(),
                "passphrase": k.passphrase.clone().unwrap_or_default(),
            }))
        })
        .collect();
    let exported_key = |id: Option<u64>| id.filter(|id| out["sshKeys"].as_array().is_some_and(|keys| keys.iter().any(|k| k["local_id"] == *id)));

    let mut identities: Vec<Value> = vault
        .identities
        .iter()
        .map(|i| {
            overlay(vault, &key("identities", i.id), json!({
                "local_id": i.id,
                "label": i.label,
                "username": Some(i.username.clone()).filter(|u| !u.is_empty()),
                "password": i.password,
                "ssh_key": reference(exported_key(i.key)),
                "is_visible": true,
            }))
        })
        .collect();

    let mut configs = Vec::new();
    let mut hosts = Vec::new();
    for host in &vault.hosts {
        let links = host_links(vault, host.id, &mut next);
        // The host's own credentials ride in a hidden identity, as Termius stores them.
        let own = host.username.is_some() || host.password.is_some() || host.key.is_some();
        let identity = match host.identity {
            Some(id) => Some(id),
            None if own => {
                identities.push(overlay(vault, &key("identities-of-host", host.id), json!({
                    "local_id": links.identity,
                    "label": "",
                    "username": host.username,
                    "password": host.password,
                    "ssh_key": reference(exported_key(host.key)),
                    "is_visible": false,
                })));
                Some(links.identity)
            }
            None => None,
        };
        let config = overlay(vault, &key("sshConfigs-of-host", host.id), json!({
            "local_id": links.config,
            "port": host.port.unwrap_or(22),
            "identity": reference(identity),
        }));
        configs.push(config.clone());
        hosts.push(overlay(vault, &key("hosts", host.id), json!({
            "local_id": host.id,
            "label": host.label,
            "address": host.address,
            "os_name": host.os.clone().unwrap_or_default(),
            "tags": host.tags,
            "group": reference(host.group),
            "ssh_config": config,
        })));
    }
    // Group SSH configs are not edited here; they travel as they arrived.
    for group in &vault.groups {
        if let Some(config) = vault.portable.get(&key("sshConfigs-of-group", group.id)) {
            configs.push(config.clone());
        }
    }
    out["identities"] = Value::Array(identities);
    out["sshConfigs"] = Value::Array(configs);
    out["hosts"] = Value::Array(hosts);
    out["groups"] = vault
        .groups
        .iter()
        .map(|g| {
            let stored = vault.portable.get(&key("groups", g.id));
            let config = vault.portable.get(&key("sshConfigs-of-group", g.id)).cloned();
            overlay(vault, &key("groups", g.id), json!({
                "local_id": g.id,
                "label": g.label,
                "parent_group": reference(g.parent),
                "ssh_config": config.unwrap_or_else(|| stored.and_then(|s| s.get("ssh_config")).cloned().unwrap_or(Value::Null)),
            }))
        })
        .collect();
    out["snippets"] = vault
        .snippets
        .iter()
        .map(|s| overlay(vault, &key("snippets", s.id), json!({"local_id": s.id, "label": s.label, "script": s.script})))
        .collect();
    out["portForwardingRules"] = vault
        .rules
        .iter()
        .filter(|r| r.host.is_some())
        .map(|r| {
            overlay(vault, &key("portForwardingRules", r.id), json!({
                "local_id": r.id,
                "label": r.label,
                "pf_type": rule_type(r.kind),
                "host": reference(r.host),
                "bound_address": r.bind_address,
                "local_port": r.port.unwrap_or(0),
                "hostname": r.destination,
                "remote_port": r.destination_port.unwrap_or(0),
            }))
        })
        .collect();
    for name in FOREIGN {
        out[name] = vault.foreign.get(name).cloned().filter(Value::is_array).unwrap_or_else(|| json!([]));
    }
    out["settings"] = vault.foreign.get("settings").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
    out
}

/// Replaces the portable part of the vault with a (validated, sync-form)
/// snapshot. Device-local data — history, logs, known hosts, cloud settings —
/// stays. Returns the `[snapshot id, new id]` pairs the merge needs.
pub fn import(vault: &mut Vault, snapshot: &Value) -> Result<Value, String> {
    let list = |name: &str| snapshot.get(name).and_then(Value::as_array).cloned().unwrap_or_default();
    let token = |id: &Value| id.to_string();
    let mut next = vault.history.iter().map(|e| e.id).max().unwrap_or(0).max(vault.next_id());
    let mut new_id = move || {
        next += 1;
        next
    };
    let mut pairs: BTreeMap<&str, Vec<Value>> = KINDS.iter().map(|k| (*k, Vec::new())).collect();
    let mut ids: BTreeMap<(&str, String), u64> = BTreeMap::new();
    let mut portable = BTreeMap::new();
    let mut assign = |kind: &'static str, record: &Value, pairs: &mut BTreeMap<&str, Vec<Value>>, ids: &mut BTreeMap<(&str, String), u64>| -> u64 {
        let id = new_id();
        let old = record.get("local_id").cloned().unwrap_or(Value::Null);
        ids.insert((kind, token(&old)), id);
        pairs.get_mut(kind).expect("known kind").push(json!([old, id]));
        id
    };
    let find = |ids: &BTreeMap<(&str, String), u64>, kind: &'static str, record: &Value, field: &str| -> Option<u64> {
        id_of(record, field).and_then(|id| ids.get(&(kind, token(&id))).copied())
    };

    let mut keys = Vec::new();
    for raw in list("sshKeys") {
        let id = assign("sshKeys", &raw, &mut pairs, &mut ids);
        let public_key = optional(&raw, "public_key");
        keys.push(SshKey {
            id,
            label: text(&raw, "label"),
            kind: crate::vault::key_kind(public_key.as_deref().unwrap_or("")),
            private_key: optional(&raw, "private_key"),
            public_key,
            passphrase: optional(&raw, "passphrase"),
        });
        portable.insert(key("sshKeys", id), raw);
    }
    let identities_raw = list("identities");
    let mut identities = Vec::new();
    let mut hidden: BTreeMap<String, Value> = BTreeMap::new();
    for raw in &identities_raw {
        let id = assign("identities", raw, &mut pairs, &mut ids);
        // Hidden identities are a host's own credentials; they are folded into the host.
        if raw.get("is_visible") == Some(&Value::Bool(false)) {
            hidden.insert(token(&raw["local_id"]), raw.clone());
            continue;
        }
        identities.push(Identity {
            id,
            label: text(raw, "label"),
            username: text(raw, "username"),
            password: optional(raw, "password"),
            key: find(&ids, "sshKeys", raw, "ssh_key"),
            ..Identity::default()
        });
        portable.insert(key("identities", id), raw.clone());
    }
    let configs: BTreeMap<String, Value> = list("sshConfigs").into_iter().map(|c| (token(&c["local_id"]), c)).collect();
    for raw in list("sshConfigs") {
        assign("sshConfigs", &raw, &mut pairs, &mut ids);
    }

    let mut groups_raw = list("groups");
    // Parents first, so a child's parent already has its new id.
    let depth = |g: &Value| {
        let mut depth = 0;
        let mut current = id_of(g, "parent_group");
        while let Some(parent) = current {
            depth += 1;
            current = groups_raw.iter().find(|x| x["local_id"] == parent).and_then(|x| id_of(x, "parent_group"));
            if depth > groups_raw.len() {
                break;
            }
        }
        depth
    };
    let mut order: Vec<(usize, Value)> = groups_raw.iter().map(|g| (depth(g), g.clone())).collect();
    order.sort_by_key(|(d, _)| *d);
    groups_raw = order.into_iter().map(|(_, g)| g).collect();
    let mut groups = Vec::new();
    for raw in &groups_raw {
        let id = assign("groups", raw, &mut pairs, &mut ids);
        groups.push(Group { id, label: text(raw, "label"), parent: find(&ids, "groups", raw, "parent_group") });
        if let Some(config) = id_of(raw, "ssh_config").and_then(|c| configs.get(&token(&c))) {
            portable.insert(key("sshConfigs-of-group", id), config.clone());
        }
        portable.insert(key("groups", id), raw.clone());
    }

    let mut hosts = Vec::new();
    for raw in list("hosts") {
        let id = assign("hosts", &raw, &mut pairs, &mut ids);
        let config = id_of(&raw, "ssh_config").and_then(|c| configs.get(&token(&c))).cloned();
        let identity_ref = config.as_ref().and_then(|c| id_of(c, "identity"));
        let own = identity_ref.as_ref().and_then(|i| hidden.get(&token(i)));
        let shared = identity_ref.as_ref().filter(|_| own.is_none()).and_then(|i| ids.get(&("identities", token(i))).copied());
        hosts.push(Host {
            id,
            label: text(&raw, "label"),
            address: text(&raw, "address"),
            os: optional(&raw, "os_name"),
            group: find(&ids, "groups", &raw, "group"),
            username: own.and_then(|i| optional(i, "username")),
            password: own.and_then(|i| optional(i, "password")),
            key: own.and_then(|i| find(&ids, "sshKeys", i, "ssh_key")),
            identity: shared,
            tags: raw.get("tags").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect(),
            port: config.as_ref().and_then(|c| c.get("port")).and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok()).filter(|p| *p != 22),
        });
        if let Some(config) = config {
            // The config and hidden identity keep their own ids so the next export matches.
            let config_id = ids.get(&("sshConfigs", token(&config["local_id"]))).copied().unwrap_or(0);
            let mut config = config.clone();
            config["local_id"] = json!(config_id);
            portable.insert(key("sshConfigs-of-host", id), config);
        }
        if let Some(own) = own {
            let identity_id = ids.get(&("identities", token(&own["local_id"]))).copied().unwrap_or(0);
            let mut own = own.clone();
            own["local_id"] = json!(identity_id);
            portable.insert(key("identities-of-host", id), own);
        }
        portable.insert(key("hosts", id), raw);
    }

    let mut snippets = Vec::new();
    for raw in list("snippets") {
        let id = assign("snippets", &raw, &mut pairs, &mut ids);
        snippets.push(Snippet { id, label: text(&raw, "label"), script: text(&raw, "script") });
        portable.insert(key("snippets", id), raw);
    }
    let mut rules = Vec::new();
    for raw in list("portForwardingRules") {
        let id = assign("portForwardingRules", &raw, &mut pairs, &mut ids);
        let kind = match raw.get("pf_type").and_then(Value::as_str) {
            Some("Remote Rule") => RuleKind::Remote,
            Some("Dynamic Rule") => RuleKind::Dynamic,
            _ => RuleKind::Local,
        };
        let port = |field: &str| raw.get(field).and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok());
        rules.push(ForwardingRule {
            id,
            label: text(&raw, "label"),
            kind,
            host: find(&ids, "hosts", &raw, "host"),
            bind_address: optional(&raw, "bound_address").unwrap_or_else(|| "127.0.0.1".into()),
            port: port("local_port"),
            destination: text(&raw, "hostname"),
            destination_port: port("remote_port"),
        });
        portable.insert(key("portForwardingRules", id), raw);
    }
    // Collections kept as they are use their snapshot ids as their own.
    let mut foreign = json!({"settings": snapshot.get("settings").cloned().unwrap_or_else(|| json!({}))});
    for name in FOREIGN {
        let records = list(name);
        for record in &records {
            pairs.get_mut(name).expect("known kind").push(json!([record["local_id"], record["local_id"]]));
        }
        foreign[name] = Value::Array(records);
    }

    vault.keys = keys;
    vault.identities = identities;
    vault.groups = groups;
    vault.hosts = hosts;
    vault.snippets = snippets;
    vault.rules = rules;
    vault.portable = portable;
    vault.foreign = foreign;
    vault.secrets_missing = false;
    Ok(Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), Value::Array(v))).collect::<Map<_, _>>()))
}

/// The native vault as the device side of a sync.
pub struct VaultDevice<'a> {
    pub vault: &'a mut Vault,
    pub now: String,
    /// Takes the encrypted backup before an import replaces the data.
    pub backup: Box<dyn FnMut(&Value) -> Result<(), String> + 'a>,
}

impl Device for VaultDevice<'_> {
    fn export(&mut self) -> Result<Value, String> {
        crate::sync::validate_snapshot(&export(self.vault, &self.now))
    }

    fn import(&mut self, snapshot: &Value) -> Result<Value, String> {
        import(self.vault, snapshot)
    }

    fn backup(&mut self, snapshot: &Value) -> Result<(), String> {
        (self.backup)(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::validate_snapshot;

    const NOW: &str = "2026-11-13T00:00:00.000Z";

    fn sample() -> Vault {
        let mut vault = Vault::default();
        vault.groups.push(Group { id: 1, label: "work".into(), parent: None });
        vault.groups.push(Group { id: 2, label: "db".into(), parent: Some(1) });
        vault.keys.push(SshKey { id: 3, label: "deploy".into(), kind: "ED25519".into(), private_key: Some("PRIVATE".into()), public_key: Some("ssh-ed25519 AAAA".into()), passphrase: None });
        vault.identities.push(Identity { id: 4, label: "ops".into(), username: "ops".into(), password: Some("pw".into()), key: Some(3), ..Identity::default() });
        let host = |id, label: &str| Host { id, label: label.into(), address: format!("{label}.lan"), os: None, group: Some(2), username: None, tags: vec![], port: None, password: None, key: None, identity: None };
        vault.hosts.push(Host { username: Some("root".into()), password: Some("secret".into()), port: Some(2222), tags: vec!["prod".into()], ..host(5, "web") });
        vault.hosts.push(Host { identity: Some(4), ..host(6, "api") });
        vault.snippets.push(Snippet { id: 7, label: "top".into(), script: "top".into() });
        vault.rules.push(ForwardingRule { id: 8, label: "pg".into(), kind: RuleKind::Local, host: Some(5), bind_address: "127.0.0.1".into(), port: Some(15432), destination: "127.0.0.1".into(), destination_port: Some(5432) });
        vault
    }

    /// The parts of a vault a snapshot carries, ids aside.
    fn shape(vault: &Vault) -> Value {
        let label_of_group = |id: Option<u64>| id.and_then(|g| vault.group(g)).map(|g| g.label.clone());
        let label_of_host = |id: Option<u64>| id.and_then(|h| vault.host(h)).map(|h| h.label.clone());
        let mut hosts: Vec<Value> = vault
            .hosts
            .iter()
            .map(|h| json!([h.label, h.address, label_of_group(h.group), h.username, h.password, h.port, h.tags,
                h.key.and_then(|k| vault.key(k)).map(|k| k.label.clone()), h.identity.and_then(|i| vault.identity(i)).map(|i| i.label.clone())]))
            .collect();
        hosts.sort_by_key(|v| v.to_string());
        json!({
            "hosts": hosts,
            "groups": vault.groups.iter().map(|g| json!([g.label, label_of_group(g.parent)])).collect::<Vec<_>>(),
            "keys": vault.keys.iter().map(|k| json!([k.label, k.private_key, k.public_key])).collect::<Vec<_>>(),
            "identities": vault.identities.iter().map(|i| json!([i.label, i.username, i.password, i.key.and_then(|k| vault.key(k)).map(|k| k.label.clone())])).collect::<Vec<_>>(),
            "snippets": vault.snippets.iter().map(|s| json!([s.label, s.script])).collect::<Vec<_>>(),
            "rules": vault.rules.iter().map(|r| json!([r.label, label_of_host(r.host), r.port, r.destination, r.destination_port])).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn a_vault_survives_export_and_import() {
        let vault = sample();
        let snapshot = validate_snapshot(&export(&vault, NOW)).unwrap();
        assert_eq!(snapshot["identities"].as_array().unwrap().len(), 2, "ops and web's own hidden identity");
        let web = snapshot["hosts"].as_array().unwrap().iter().find(|h| h["label"] == "web").unwrap();
        assert_eq!(web["ssh_config"]["port"], 2222);
        let mut copy = Vault { history: vec![crate::vault::HistoryEntry::default()], ..Vault::default() };
        let pairs = import(&mut copy, &snapshot).unwrap();
        assert_eq!(shape(&copy), shape(&vault));
        assert_eq!(copy.history.len(), 1, "device-local history stays");
        assert_eq!(pairs["hosts"].as_array().unwrap().len(), 2);
        assert!(!copy.secrets_missing);
    }

    #[test]
    fn fields_and_collections_this_client_does_not_model_survive() {
        let mut snapshot = validate_snapshot(&export(&sample(), NOW)).unwrap();
        let config_id = snapshot["hosts"][0]["ssh_config"]["local_id"].clone();
        for config in snapshot["sshConfigs"].as_array_mut().unwrap() {
            if config["local_id"] == config_id {
                config["env_variables"] = json!([{"name": "LANG", "value": "C"}]);
            }
        }
        snapshot["hosts"][0]["backspace"] = json!("ctrl-h");
        snapshot["sshKeys"][0]["certificates"] = json!([{"local_id": 99, "certificate": "cert"}]);
        snapshot["snippetPackages"] = json!([{"local_id": "pkg", "label": "tools"}]);
        snapshot["settings"] = json!({"settings_font_size": 14});
        let snapshot = validate_snapshot(&snapshot).unwrap();
        let mut vault = Vault::default();
        import(&mut vault, &snapshot).unwrap();
        vault.hosts[0].label = "renamed".into();
        let again = validate_snapshot(&export(&vault, NOW)).unwrap();
        let host = again["hosts"].as_array().unwrap().iter().find(|h| h["label"] == "renamed").unwrap();
        assert_eq!(host["backspace"], "ctrl-h");
        let config_id = host["ssh_config"]["local_id"].clone();
        let config = again["sshConfigs"].as_array().unwrap().iter().find(|c| c["local_id"] == config_id).unwrap();
        assert_eq!(config["env_variables"][0]["name"], "LANG");
        assert_eq!(again["sshKeys"][0]["certificates"][0]["certificate"], "cert");
        assert_eq!(again["snippetPackages"][0]["label"], "tools");
        assert_eq!(again["settings"]["settings_font_size"], 14);
    }

    #[test]
    fn exporting_twice_after_an_import_gives_the_same_digests() {
        let snapshot = validate_snapshot(&export(&sample(), NOW)).unwrap();
        let mut vault = Vault::default();
        import(&mut vault, &snapshot).unwrap();
        let first = validate_snapshot(&export(&vault, NOW)).unwrap();
        let second = validate_snapshot(&export(&vault, NOW)).unwrap();
        assert_eq!(crate::sync::digests_of(Some(&first)), crate::sync::digests_of(Some(&second)));
    }

    #[test]
    fn two_native_vaults_converge_through_the_cloud() {
        let (url, _) = crate::cloud::tests::server();
        let cloud = crate::cloud::tests::config(&url).validate().unwrap();
        let mut a = sample();
        let mut b = Vault::default();
        b.hosts.push(Host { id: 1, label: "laptop".into(), address: "10.0.0.9".into(), os: None, group: None, username: Some("me".into()), tags: vec![], port: None, password: None, key: None, identity: None });
        let round = |vault: &mut Vault| {
            let saved = vault.sync_state.take();
            let outcome = {
                let mut device = VaultDevice { vault: &mut *vault, now: NOW.into(), backup: Box::new(|_| Ok(())) };
                crate::sync::sync_once(saved, "server", None, &mut device, &cloud, NOW).unwrap()
            };
            assert!(outcome.conflicts.is_empty(), "{:?}", outcome.conflicts);
            vault.sync_state = outcome.state;
        };
        round(&mut a);
        round(&mut b);
        round(&mut a);
        let labels = |v: &Vault| {
            let mut l: Vec<String> = v.hosts.iter().map(|h| h.label.clone()).collect();
            l.sort();
            l
        };
        assert_eq!(labels(&a), ["api", "laptop", "web"]);
        assert_eq!(labels(&b), ["api", "laptop", "web"]);
        let web = b.hosts.iter().find(|h| h.label == "web").unwrap();
        assert_eq!((web.username.as_deref(), web.password.as_deref(), web.port), (Some("root"), Some("secret"), Some(2222)));
        // An edit on one device reaches the other; nothing else moves.
        b.hosts.iter_mut().find(|h| h.label == "laptop").unwrap().address = "10.0.0.10".into();
        round(&mut b);
        round(&mut a);
        assert_eq!(a.hosts.iter().find(|h| h.label == "laptop").unwrap().address, "10.0.0.10");
        assert_eq!(shape(&a), shape(&b));
    }

    #[test]
    fn keys_from_the_legacy_profile_without_a_private_part_are_not_exported() {
        let mut vault = sample();
        vault.keys.push(SshKey { id: 50, label: "legacy".into(), kind: "RSA".into(), ..SshKey::default() });
        let snapshot = export(&vault, NOW);
        assert!(!snapshot["sshKeys"].as_array().unwrap().iter().any(|k| k["label"] == "legacy"));
    }
}
