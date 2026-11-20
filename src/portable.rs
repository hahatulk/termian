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

use crate::sync::{empty_snapshot, map_ids, Device, Ids, KINDS};
use crate::vault::{EnvVar, ForwardingRule, Group, Host, HostSettings, Identity, RuleKind, Snippet, SnippetPackage, SshKey, Vault};

/// Collections kept as they are.
const FOREIGN: [&str; 3] = ["telnetConfigs", "proxies", "workspaceTemplates"];

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

/// Legacy host settings from a host and its SSH config; the startup snippet is
/// linked by the caller.
fn host_settings(host: &Value, config: &Value) -> HostSettings {
    HostSettings {
        ctrl_h_backspace: host.get("backspace").and_then(Value::as_str).is_some_and(|b| b != "default"),
        agent_forwarding: config.get("agent_forwarding").and_then(Value::as_bool).unwrap_or(false),
        charset: optional(config, "charset").filter(|c| !c.eq_ignore_ascii_case("UTF-8")),
        env: config
            .get("env_variables")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| Some(EnvVar { name: v.get("name")?.as_str()?.to_owned(), value: v.get("value").and_then(Value::as_str).unwrap_or_default().to_owned() }))
            .filter(|v| !v.name.is_empty())
            .collect(),
        startup_snippet: None,
        color_scheme: config.get("color_scheme").filter(|c| !c.is_null()).cloned(),
    }
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

/// The key's certificate list: the imported first entry with the current text, or none.
fn certificates(vault: &Vault, k: &SshKey) -> Value {
    let Some(certificate) = &k.certificate else { return json!([]) };
    let mut first = vault.portable.get(&key("sshKeys", k.id)).map(|raw| raw["certificates"][0].clone()).filter(Value::is_object).unwrap_or_else(|| json!({}));
    first["certificate"] = json!(certificate);
    json!([first])
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
                "certificates": certificates(vault, k),
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
        let settings = &host.settings;
        let config = overlay(vault, &key("sshConfigs-of-host", host.id), json!({
            "local_id": links.config,
            "port": host.port.unwrap_or(22),
            "identity": reference(identity),
            "agent_forwarding": settings.agent_forwarding,
            "charset": settings.charset,
            "env_variables": settings.env.iter().map(|v| json!({"name": v.name, "value": v.value})).collect::<Vec<_>>(),
            "startup_snippet": reference(settings.startup_snippet.filter(|id| vault.snippets.iter().any(|s| s.id == *id))),
            "color_scheme": settings.color_scheme,
        }));
        configs.push(config.clone());
        hosts.push(overlay(vault, &key("hosts", host.id), json!({
            "local_id": host.id,
            "label": host.label,
            "address": host.address,
            "os_name": host.os.clone().unwrap_or_default(),
            "tags": host.tags,
            "group": reference(host.group),
            // Ctrl+H keeps the value it arrived with ("legacy" when set here).
            "backspace": if host.settings.ctrl_h_backspace {
                vault.portable.get(&key("hosts", host.id)).and_then(|h| h.get("backspace")).and_then(Value::as_str).filter(|b| *b != "default").unwrap_or("legacy")
            } else {
                "default"
            },
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
    out["snippetPackages"] = vault
        .packages
        .iter()
        .map(|p| overlay(vault, &key("snippetPackages", p.id), json!({"local_id": p.id, "label": p.label})))
        .collect();
    out["snippets"] = vault
        .snippets
        .iter()
        .map(|s| {
            let package = s.package.filter(|p| vault.package(*p).is_some());
            let targets: Vec<Value> = s.targets.iter().filter(|h| vault.host(**h).is_some()).map(|h| json!({"local_id": h})).collect();
            overlay(vault, &key("snippets", s.id), json!({
                "local_id": s.id,
                "label": s.label,
                "script": s.script,
                "package": reference(package),
                "targetHosts": if targets.is_empty() { Value::Null } else { Value::Array(targets) },
            }))
        })
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
    out["settings"] = crate::log_backup::export_settings(vault);
    out
}

/// Replaces the portable part of the vault with a (validated, sync-form)
/// snapshot. Returns the `[snapshot id, new id]` pairs the merge needs.
pub fn import(vault: &mut Vault, snapshot: &Value, matched: &Ids) -> Result<Value, String> {
    let list = |name: &str| snapshot.get(name).and_then(Value::as_array).cloned().unwrap_or_default();
    let token = |id: &Value| id.to_string();
    let reused: BTreeMap<_, _> = matched.iter().flat_map(|(kind, entries)| {
        entries.iter().filter_map(move |(local, remote)| {
            Some(((kind.as_str(), token(remote)), local.strip_prefix("number:")?.parse::<u64>().ok()?))
        })
    }).collect();
    let mut next = vault.history.iter().filter_map(|e| e.host)
        .chain(vault.shell_history.iter().filter_map(|e| e.host))
        .chain(vault.foreign["workspaceTemplates"].as_array().into_iter().flatten().filter_map(|v| v["local_id"].as_u64()))
        .chain(reused.values().copied())
        .max().unwrap_or(0).max(vault.next_id());
    let mut new_id = move || {
        next += 1;
        next
    };
    let mut pairs: BTreeMap<&str, Vec<Value>> = KINDS.iter().map(|k| (*k, Vec::new())).collect();
    let mut ids: BTreeMap<(&str, String), u64> = BTreeMap::new();
    let mut portable = BTreeMap::new();
    let mut assign = |kind: &'static str, record: &Value, pairs: &mut BTreeMap<&str, Vec<Value>>, ids: &mut BTreeMap<(&str, String), u64>| -> u64 {
        let old = record.get("local_id").cloned().unwrap_or(Value::Null);
        let old_token = token(&old);
        let id = reused.get(&(kind, old_token.clone())).copied().unwrap_or_else(&mut new_id);
        ids.insert((kind, old_token), id);
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
            certificate: raw["certificates"][0]["certificate"].as_str().filter(|c| !c.is_empty()).map(str::to_owned),
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
    let mut startup_snippets = Vec::new();
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
            settings: config.as_ref().map(|c| host_settings(&raw, c)).unwrap_or_else(|| host_settings(&raw, &Value::Null)),
        });
        // Snippets come later; the startup snippet is linked once they have ids.
        if let Some(snippet) = config.as_ref().and_then(|c| id_of(c, "startup_snippet")) {
            startup_snippets.push((hosts.len() - 1, snippet));
        }
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

    let mut packages = Vec::new();
    for raw in list("snippetPackages") {
        let id = assign("snippetPackages", &raw, &mut pairs, &mut ids);
        packages.push(SnippetPackage { id, label: text(&raw, "label") });
        portable.insert(key("snippetPackages", id), raw);
    }
    let mut snippets = Vec::new();
    for raw in list("snippets") {
        let id = assign("snippets", &raw, &mut pairs, &mut ids);
        let targets = raw.get("targetHosts").and_then(Value::as_array).into_iter().flatten();
        snippets.push(Snippet {
            id,
            label: text(&raw, "label"),
            script: text(&raw, "script"),
            package: find(&ids, "snippetPackages", &raw, "package"),
            targets: targets.filter_map(|h| h.get("local_id")).filter_map(|h| ids.get(&("hosts", token(h))).copied()).collect(),
        });
        portable.insert(key("snippets", id), raw);
    }
    for (host, snippet) in startup_snippets {
        hosts[host].settings.startup_snippet = ids.get(&("snippets", token(&snippet))).copied();
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
    let templates = list("workspaceTemplates");
    for raw in &templates {
        assign("workspaceTemplates", raw, &mut pairs, &mut ids);
    }
    let mut foreign = json!({"settings": snapshot.get("settings").cloned().unwrap_or_else(|| json!({}))});
    let translate = |kind: &str, id: &Value| {
        ids.get(&(kind, token(id))).map_or_else(|| id.clone(), |id| json!(id))
    };
    for name in FOREIGN {
        let records = if name == "workspaceTemplates" {
            templates.iter().map(|record| map_ids(record, name, &mut |kind, id| translate(kind, id))).collect()
        } else {
            let records = list(name);
            for record in &records {
                pairs.get_mut(name).expect("known kind").push(json!([record["local_id"], record["local_id"]]));
            }
            records.iter().map(|record| map_ids(record, name, &mut |kind, id| translate(kind, id))).collect()
        };
        foreign[name] = Value::Array(records);
    }
    for raw in portable.values_mut() {
        *raw = map_ids(raw, "", &mut |kind, id| translate(kind, id));
    }

    crate::log_backup::apply_catalog(vault, &foreign["settings"]);
    vault.keys = keys;
    vault.identities = identities;
    vault.groups = groups;
    if let Some(highest) = vault.hosts.iter().map(|host| host.id).max() {
        vault.retired_host_id = vault.retired_host_id.max(highest);
    }
    vault.hosts = hosts;
    vault.snippets = snippets;
    vault.packages = packages;
    vault.rules = rules;
    vault.portable = portable;
    vault.foreign = foreign;
    scrub_cached_secrets(vault);
    vault.secrets_missing = false;
    Ok(Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), Value::Array(v))).collect::<Map<_, _>>()))
}

pub(crate) fn scrub_cached_secrets(vault: &mut Vault) {
    fn clean(record: &mut Value, kind: &str) {
        let Some(fields) = record.as_object_mut() else { return };
        let children: &[(&str, &str)] = match kind {
            "identities" | "identities-of-host" => {
                fields.remove("password");
                &[("ssh_key", "sshKeys")]
            }
            "sshKeys" => {
                fields.remove("private_key");
                fields.remove("passphrase");
                &[]
            }
            "hosts" | "groups" => &[("ssh_config", "config"), ("telnet_config", "config")],
            "config" | "telnetConfigs" | "sshConfigs-of-host" | "sshConfigs-of-group" => &[("identity", "identities"), ("proxy", "proxies")],
            "proxies" => &[("identity", "identities")],
            _ => &[],
        };
        for &(field, kind) in children {
            if let Some(child) = fields.get_mut(field) { clean(child, kind); }
        }
    }
    for (name, value) in &mut vault.portable {
        clean(value, name.split('|').next().unwrap_or(""));
    }
    for name in FOREIGN {
        for record in vault.foreign.get_mut(name).and_then(Value::as_array_mut).into_iter().flatten() {
            clean(record, name);
        }
    }
}

pub struct VaultDevice<'a> {
    pub vault: &'a mut Vault,
    pub now: String,
    /// Takes the encrypted backup before an import replaces the data.
    pub backup: Box<dyn FnMut(&Vault) -> Result<(), String> + 'a>,
}

impl Device for VaultDevice<'_> {
    fn export(&mut self) -> Result<Value, String> {
        crate::sync::validate_snapshot(&export(self.vault, &self.now))
    }

    fn import(&mut self, snapshot: &Value, ids: &Ids) -> Result<Value, String> {
        import(self.vault, snapshot, ids)
    }

    fn backup(&mut self, _: &Value) -> Result<(), String> {
        (self.backup)(self.vault)
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
        vault.keys.push(SshKey { id: 3, label: "deploy".into(), kind: "ED25519".into(), private_key: Some("PRIVATE".into()), public_key: Some("ssh-ed25519 AAAA".into()), passphrase: None, certificate: None });
        vault.identities.push(Identity { id: 4, label: "ops".into(), username: "ops".into(), password: Some("pw".into()), key: Some(3), ..Identity::default() });
        let host = |id, label: &str| Host { id, label: label.into(), address: format!("{label}.lan"), os: None, group: Some(2), username: None, tags: vec![], port: None, password: None, key: None, identity: None, settings: HostSettings::default() };
        vault.hosts.push(Host { username: Some("root".into()), password: Some("secret".into()), port: Some(2222), tags: vec!["prod".into()], ..host(5, "web") });
        vault.hosts.push(Host { identity: Some(4), ..host(6, "api") });
        vault.packages.push(SnippetPackage { id: 9, label: "tools".into() });
        vault.snippets.push(Snippet { id: 7, label: "top".into(), script: "top".into(), package: Some(9), targets: vec![5, 6] });
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
            "snippets": vault.snippets.iter().map(|s| json!([s.label, s.script, s.package.and_then(|p| vault.package(p)).map(|p| p.label.clone()), s.targets.iter().map(|h| label_of_host(Some(*h))).collect::<Vec<_>>()])).collect::<Vec<_>>(),
            "packages": vault.packages.iter().map(|p| p.label.clone()).collect::<Vec<_>>(),
            "rules": vault.rules.iter().map(|r| json!([r.label, label_of_host(r.host), r.port, r.destination, r.destination_port])).collect::<Vec<_>>(),
        })
    }

    #[test]
    fn group_removal_unlinks_hosts_and_local_panes_still_export() {
        let mut vault = sample();
        vault.foreign = json!({"workspaceTemplates": [{"local_id": 50, "label": "mixed", "workspaceTerminals": [
            {"protocol": "local"}, {"protocol": "ssh", "host": {"local_id": 5}}
        ]}]});
        let mut invalid = export(&vault, NOW);
        invalid["workspaceTemplates"][0]["workspaceTerminals"][0]["protocol"] = json!("ssh");
        assert!(validate_snapshot(&invalid).is_err());
        validate_snapshot(&export(&vault, NOW)).unwrap();
        assert_eq!(vault.remove_group(1).len(), 2);
        assert!(vault.rules[0].host.is_none());
        assert!(vault.snippets[0].targets.is_empty());
        let snapshot = validate_snapshot(&export(&vault, NOW)).unwrap();
        assert_eq!(snapshot["workspaceTemplates"][0]["workspaceTerminals"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["workspaceTemplates"][0]["workspaceTerminals"][0]["protocol"], "local");
    }

    #[test]
    fn imported_deletion_retires_the_old_host_id() {
        let mut vault = sample();
        vault.hosts[0].id = 900;
        import(&mut vault, &empty_snapshot(NOW), &Ids::new()).unwrap();
        assert!(vault.hosts.is_empty());
        let persisted: Vault = serde_json::from_value(serde_json::to_value(&vault).unwrap()).unwrap();
        assert!(persisted.next_id() > 900);
    }

    #[test]
    fn old_cache_loses_duplicate_secrets_and_keeps_extension_fields() {
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::new(dir.path().join("vault.bin"), [7; 32]);
        let snapshot = validate_snapshot(&export(&sample(), NOW)).unwrap();
        let mut vault = Vault::default();
        import(&mut vault, &snapshot, &Ids::new()).unwrap();
        let key_id = vault.keys[0].id;
        let identity = vault.identities[0].id;
        let host = vault.hosts[0].id;
        vault.portable.get_mut(&key("identities", identity)).unwrap()["password"] = json!("old-pw");
        vault.portable.get_mut(&key("sshKeys", key_id)).unwrap()["private_key"] = json!("old-private");
        vault.portable.get_mut(&key("hosts", host)).unwrap()["extension"] = json!({"identity": {"password": "extension value"}});
        store.save(&vault).unwrap();
        let mut loaded = store.load().unwrap().unwrap();
        let cached = serde_json::to_string(&loaded.portable).unwrap();
        for duplicate in ["old-pw", "old-private", "PRIVATE", "\"secret\"", "\"pw\""] { assert!(!cached.contains(duplicate)); }
        store.save(&loaded).unwrap();
        loaded = store.load().unwrap().unwrap();
        let snapshot = validate_snapshot(&export(&loaded, NOW)).unwrap();
        assert_eq!(snapshot["hosts"][0]["extension"]["identity"]["password"], "extension value");
        assert_eq!(shape(&loaded), shape(&sample()));
        assert_eq!(snapshot["sshKeys"][0]["private_key"], "PRIVATE");
        assert!(snapshot["identities"].as_array().unwrap().iter().any(|i| i["password"] == "pw"));
        loaded.remove_key(key_id);
        assert!(loaded.identities[0].key.is_none());
        loaded.remove_identity(identity);
        assert!(!loaded.portable.contains_key(&key("sshKeys", key_id)));
        assert!(!loaded.portable.contains_key(&key("identities", identity)));
        validate_snapshot(&export(&loaded, NOW)).unwrap();
    }

    #[test]
    fn a_vault_survives_export_and_import() {
        let vault = sample();
        let snapshot = validate_snapshot(&export(&vault, NOW)).unwrap();
        assert_eq!(snapshot["identities"].as_array().unwrap().len(), 2, "ops and web's own hidden identity");
        let web = snapshot["hosts"].as_array().unwrap().iter().find(|h| h["label"] == "web").unwrap();
        assert_eq!(web["ssh_config"]["port"], 2222);
        let mut copy = Vault { history: vec![crate::vault::HistoryEntry::default()], ..Vault::default() };
        let pairs = import(&mut copy, &snapshot, &Ids::new()).unwrap();
        assert_eq!(shape(&copy), shape(&vault));
        assert_eq!(copy.history.len(), 1, "device-local history stays");
        assert_eq!(pairs["hosts"].as_array().unwrap().len(), 2);
        assert!(!copy.secrets_missing);
    }

    #[test]
    fn sync_import_preserves_host_ids_and_translates_workspace_references() {
        let mut vault = sample();
        vault.history.push(crate::vault::HistoryEntry { host: Some(5), ..Default::default() });
        vault.shell_history.push(crate::vault::ShellCommand { host: Some(5), ..Default::default() });
        vault.foreign = json!({"workspaceTemplates": [{
            "local_id": 11, "label": "web", "workspaceTerminals": [
                {"id": "terminal", "protocol": "ssh", "host": { "local_id": 5 }}
            ]
        }]});
        let local = validate_snapshot(&export(&vault, NOW)).unwrap();
        let mut matched = Ids::new();
        let mut remote = crate::sync::to_sync_form(&local, &mut matched);
        remote["hosts"][0]["label"] = json!("updated");
        let remote = validate_snapshot(&remote).unwrap();

        let mut fresh = Vault::default();
        import(&mut fresh, &remote, &Ids::new()).unwrap();
        let templates = crate::workspace_template::templates(&fresh);
        let id = fresh.hosts.iter().find(|host| host.label == "updated").unwrap().id;
        assert_eq!(templates[0].workspace_terminals[0].target(&fresh), Some(crate::workspace_template::Target::Host(id)));

        import(&mut vault, &remote, &matched).unwrap();
        assert_eq!(vault.history[0].host, Some(5));
        assert_eq!(vault.shell_history[0].host, Some(5));
        assert_eq!(vault.host(5).unwrap().label, "updated");
        let templates = crate::workspace_template::templates(&vault);
        assert_eq!(templates[0].id, 11);
        assert_eq!(templates[0].workspace_terminals[0].target(&vault), Some(crate::workspace_template::Target::Host(5)));
        validate_snapshot(&export(&vault, NOW)).unwrap();
    }

    #[test]
    fn host_settings_travel_in_the_legacy_fields() {
        let mut vault = sample();
        vault.hosts[0].settings = HostSettings {
            ctrl_h_backspace: true,
            agent_forwarding: true,
            charset: Some("KOI8-R".into()),
            env: vec![EnvVar { name: "LANG".into(), value: "C".into() }],
            startup_snippet: Some(7),
            color_scheme: Some(json!("Night Owl")),
        };
        let snapshot = validate_snapshot(&export(&vault, NOW)).unwrap();
        let web = snapshot["hosts"].as_array().unwrap().iter().find(|h| h["label"] == "web").unwrap();
        assert_eq!(web["backspace"], "legacy");
        let config = &web["ssh_config"];
        assert_eq!((config["agent_forwarding"].clone(), config["charset"].clone()), (json!(true), json!("KOI8-R")));
        assert_eq!(config["env_variables"], json!([{"name": "LANG", "value": "C"}]));
        assert_eq!(config["color_scheme"], "Night Owl");
        let mut copy = Vault::default();
        import(&mut copy, &snapshot, &Ids::new()).unwrap();
        let web = copy.hosts.iter().find(|h| h.label == "web").unwrap();
        let snippet = copy.snippets.iter().find(|s| s.label == "top").unwrap().id;
        assert_eq!(web.settings, HostSettings { startup_snippet: Some(snippet), ..vault.hosts[0].settings.clone() });
        assert_eq!(copy.hosts.iter().find(|h| h.label == "api").unwrap().settings, HostSettings::default(), "defaults stay empty");
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
        snapshot["snippetPackages"][0]["analytics_id"] = json!("pkg");
        snapshot["settings"] = json!({"settings_font_size": 14});
        let snapshot = validate_snapshot(&snapshot).unwrap();
        let mut vault = Vault::default();
        import(&mut vault, &snapshot, &Ids::new()).unwrap();
        vault.hosts[0].label = "renamed".into();
        let again = validate_snapshot(&export(&vault, NOW)).unwrap();
        let host = again["hosts"].as_array().unwrap().iter().find(|h| h["label"] == "renamed").unwrap();
        assert_eq!(host["backspace"], "ctrl-h");
        let config_id = host["ssh_config"]["local_id"].clone();
        let config = again["sshConfigs"].as_array().unwrap().iter().find(|c| c["local_id"] == config_id).unwrap();
        assert_eq!(config["env_variables"][0]["name"], "LANG");
        assert_eq!(again["sshKeys"][0]["certificates"][0]["certificate"], "cert");
        assert_eq!(again["sshKeys"][0]["certificates"][0]["local_id"], 99, "the certificate keeps its id");

        // Edited in the key form: the text changes, clearing it drops the certificate.
        let mut edited = vault.clone();
        edited.keys[0].certificate = Some("new cert".into());
        assert_eq!(export(&edited, NOW)["sshKeys"][0]["certificates"], json!([{"local_id": 99, "certificate": "new cert"}]));
        edited.keys[0].certificate = None;
        assert_eq!(export(&edited, NOW)["sshKeys"][0]["certificates"], json!([]));
        assert_eq!((again["snippetPackages"][0]["label"].as_str(), again["snippetPackages"][0]["analytics_id"].as_str()), (Some("tools"), Some("pkg")));
        assert_eq!(again["settings"]["settings_font_size"], 14);
    }

    #[test]
    fn exporting_twice_after_an_import_gives_the_same_digests() {
        let snapshot = validate_snapshot(&export(&sample(), NOW)).unwrap();
        let mut vault = Vault::default();
        import(&mut vault, &snapshot, &Ids::new()).unwrap();
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
        b.hosts.push(Host { id: 1, label: "laptop".into(), address: "10.0.0.9".into(), os: None, group: None, username: Some("me".into()), tags: vec![], port: None, password: None, key: None, identity: None, settings: HostSettings::default() });
        let round = |vault: &mut Vault| {
            let saved = vault.sync_state.take();
            let outcome = {
                let mut device = VaultDevice { vault: &mut *vault, now: NOW.into(), backup: Box::new(|_| Ok(())) };
                crate::sync::sync_once(saved, "server", &mut device, &cloud, NOW).unwrap()
            };
            assert_eq!(outcome.resolved_conflicts, 0);
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
