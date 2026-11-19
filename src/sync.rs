//! Git-like three-way merge of portable snapshots, ported from
//! legacy/src/bridge/sync-merge.js (and the reference checks of
//! legacy/src/self-cloud-adapter.cjs `validateSnapshot`).
//!
//! The cloud snapshot uses stable sync IDs as `local_id`; each device keeps a
//! map local_id → sync ID per collection, plus the record digests both sides
//! had after the last sync. A record changed on one side is applied to the
//! other; the same record changed on both sides is a conflict that is never
//! resolved silently — only by `prefer`, which the user chooses.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::cloud::{Condition, Remote};

pub const FORMAT: &str = "termius-portable-snapshot";
pub const VERSION: u64 = 1;

pub const KINDS: [&str; 11] = [
    "groups", "hosts", "identities", "sshKeys", "snippets", "snippetPackages",
    "portForwardingRules", "sshConfigs", "telnetConfigs", "proxies", "workspaceTemplates",
];
/// Every array a snapshot carries (knownHosts stays empty: device-local trust).
const COLLECTIONS: [&str; 12] = [
    "groups", "hosts", "identities", "sshKeys", "snippets", "snippetPackages",
    "portForwardingRules", "knownHosts", "sshConfigs", "telnetConfigs", "proxies", "workspaceTemplates",
];

fn ref_field(key: &str) -> Option<&'static str> {
    Some(match key {
        "ssh_key" => "sshKeys",
        "identity" => "identities",
        "proxy" => "proxies",
        "startup_snippet" => "snippets",
        "parent_group" | "group" => "groups",
        "ssh_config" => "sshConfigs",
        "telnet_config" => "telnetConfigs",
        "package" => "snippetPackages",
        "host" => "hosts",
        _ => return None,
    })
}

fn ref_list(key: &str) -> Option<&'static str> {
    matches!(key, "targetHosts" | "hosts_chain").then_some("hosts")
}

const VOLATILE: [&str; 8] = ["id", "global_id", "updated_at", "local_updated_at", "created_at", "changedFields", "is_shared", "encrypted_with"];

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "groups" => "Группа",
        "hosts" => "Хост",
        "identities" => "Идентификатор",
        "sshKeys" => "SSH-ключ",
        "snippets" => "Сниппет",
        "snippetPackages" => "Пакет сниппетов",
        "portForwardingRules" => "Проброс портов",
        "sshConfigs" => "SSH-конфигурация",
        "telnetConfigs" => "Telnet-конфигурация",
        "proxies" => "Прокси",
        "workspaceTemplates" => "Шаблон рабочей области",
        "settings" => "Настройка",
        _ => "Запись",
    }
}

/// `number:5` / `string:abc`, the legacy id token.
pub(crate) fn token(id: &Value) -> String {
    match id {
        Value::String(s) => format!("string:{s}"),
        Value::Number(n) => format!("number:{n}"),
        other => format!("other:{other}"),
    }
}

fn ref_id(value: &Value) -> Option<&Value> {
    value.as_object()?.get("local_id").filter(|id| !id.is_null())
}

fn key_of(kind: &str, id: &Value) -> String {
    format!("{kind}|{}", token(id))
}

/// Replaces the record's own local_id and every reference with `id_of`.
pub(crate) fn map_ids(value: &Value, kind: &str, id_of: &mut dyn FnMut(&str, &Value) -> Value) -> Value {
    fn walk(item: &Value, item_kind: Option<&str>, id_of: &mut dyn FnMut(&str, &Value) -> Value) -> Value {
        match item {
            Value::Array(items) => Value::Array(items.iter().map(|child| walk(child, None, id_of)).collect()),
            Value::Object(map) => {
                let mut out = Map::new();
                for (key, child) in map {
                    let value = if key == "local_id" && item_kind.is_some() {
                        if child.is_null() { Value::Null } else { id_of(item_kind.unwrap(), child) }
                    } else if let (Some(kind), true) = (ref_field(key), child.is_object()) {
                        walk(child, Some(kind), id_of)
                    } else if let (Some(kind), Value::Array(list)) = (ref_list(key), child) {
                        Value::Array(list.iter().map(|x| if x.is_object() { walk(x, Some(kind), id_of) } else { x.clone() }).collect())
                    } else {
                        walk(child, None, id_of)
                    };
                    out.insert(key.clone(), value);
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }
    walk(value, Some(kind), id_of)
}

/// Stable form for digests: references reduced to their IDs, bookkeeping dropped.
fn canonical(value: &Value, kind: Option<&str>) -> Value {
    fn walk(item: &Value, item_kind: Option<&str>, top: bool) -> Value {
        match item {
            Value::Array(items) => Value::Array(items.iter().map(|c| walk(c, None, false)).collect()),
            Value::Object(map) => {
                if item_kind.is_some() && !top {
                    return json!({"local_id": map.get("local_id").cloned().unwrap_or(Value::Null)});
                }
                let mut out = Map::new();
                for (key, child) in map {
                    if VOLATILE.contains(&key.as_str()) || (top && key == "status") {
                        continue;
                    }
                    if key == "local_id" {
                        if item_kind.is_some() {
                            out.insert(key.clone(), child.clone());
                        }
                        continue;
                    }
                    let value = if let (Some(kind), true) = (ref_field(key), child.is_object()) {
                        walk(child, Some(kind), false)
                    } else if let (Some(kind), Value::Array(list)) = (ref_list(key), child) {
                        Value::Array(list.iter().map(|x| if x.is_object() { walk(x, Some(kind), false) } else { x.clone() }).collect())
                    } else {
                        walk(child, None, false)
                    };
                    out.insert(key.clone(), value);
                }
                Value::Object(out)
            }
            other => other.clone(),
        }
    }
    walk(value, kind, true)
}

fn references_of(value: &Value, kind: &str) -> Vec<String> {
    fn walk(item: &Value, item_kind: Option<&str>, top: bool, refs: &mut Vec<String>) {
        match item {
            Value::Array(items) => items.iter().for_each(|c| walk(c, None, false, refs)),
            Value::Object(map) => {
                if let (Some(kind), false, Some(id)) = (item_kind, top, ref_id(item)) {
                    refs.push(key_of(kind, id));
                }
                for (key, child) in map {
                    if let (Some(kind), true) = (ref_field(key), child.is_object()) {
                        walk(child, Some(kind), false, refs);
                    } else if let (Some(kind), Value::Array(list)) = (ref_list(key), child) {
                        list.iter().for_each(|x| walk(x, Some(kind), false, refs));
                    } else if key != "local_id" {
                        walk(child, None, false, refs);
                    }
                }
            }
            _ => {}
        }
    }
    let mut refs = Vec::new();
    walk(value, Some(kind), true, &mut refs);
    refs
}

/// serde_json maps are sorted, so this is already a stable serialisation.
fn sha(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_default();
    Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn label_of(kind: &str, value: &Value) -> String {
    let label = if kind == "settings" {
        value.as_str().map(str::to_owned)
    } else {
        ["label", "name"]
            .iter()
            .find_map(|k| value.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()))
            .or_else(|| (kind == "proxies").then(|| value.get("hostname").and_then(Value::as_str)).flatten())
            .map(str::to_owned)
    };
    let label: String = label.unwrap_or_default().chars().map(|c| if (c as u32) < 0x20 { ' ' } else { c }).collect();
    label.trim().chars().take(100).collect()
}

pub fn new_sync_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// local_id token → sync ID, per collection.
pub type Ids = BTreeMap<String, BTreeMap<String, Value>>;

const LABELED: [&str; 9] = ["groups", "hosts", "identities", "sshKeys", "snippets", "snippetPackages", "proxies", "portForwardingRules", "workspaceTemplates"];
const STRUCTURE: [(&str, &str, &str); 14] = [
    ("hosts", "ssh_config", "sshConfigs"), ("hosts", "telnet_config", "telnetConfigs"),
    ("groups", "ssh_config", "sshConfigs"), ("groups", "telnet_config", "telnetConfigs"),
    ("groups", "parent_group", "groups"), ("hosts", "group", "groups"),
    ("sshConfigs", "identity", "identities"), ("telnetConfigs", "identity", "identities"),
    ("sshConfigs", "proxy", "proxies"), ("sshConfigs", "startup_snippet", "snippets"),
    ("telnetConfigs", "startup_snippet", "snippets"), ("identities", "ssh_key", "sshKeys"),
    ("proxies", "identity", "identities"), ("snippets", "package", "snippetPackages"),
];

fn records<'a>(snapshot: &'a Value, kind: &str) -> impl Iterator<Item = &'a Value> {
    snapshot.get(kind).and_then(Value::as_array).into_iter().flatten()
}

/// Binds local records without a sync ID to unclaimed remote records: by
/// name first, then through references of already-bound records.
pub fn match_ids(local: &Value, remote: Option<&Value>, ids: &mut Ids) {
    let Some(remote) = remote else { return };
    let mut claimed: HashSet<String> = HashSet::new();
    for kind in KINDS {
        for sync_id in ids.get(kind).into_iter().flat_map(|m| m.values()) {
            claimed.insert(key_of(kind, sync_id));
        }
    }
    let mut bind = |ids: &mut Ids, kind: &str, local_id: Option<&Value>, sync_id: Option<&Value>| -> bool {
        let (Some(local_id), Some(sync_id)) = (local_id, sync_id) else { return false };
        if ids.get(kind).is_some_and(|m| m.contains_key(&token(local_id))) || claimed.contains(&key_of(kind, sync_id)) {
            return false;
        }
        ids.entry(kind.into()).or_default().insert(token(local_id), sync_id.clone());
        claimed.insert(key_of(kind, sync_id));
        true
    };
    let by_name = |kind: &str, snapshot: &Value| -> HashMap<String, Value> {
        let mut names = HashMap::new();
        let mut seen: HashMap<String, usize> = HashMap::new();
        for record in records(snapshot, kind) {
            let label = label_of(kind, record).to_lowercase();
            if label.is_empty() {
                continue;
            }
            let count = seen.entry(label.clone()).or_default();
            names.insert(format!("{label}#{count}"), record.clone());
            *count += 1;
        }
        names
    };
    for kind in LABELED {
        let theirs = by_name(kind, remote);
        for (name, record) in by_name(kind, local) {
            if let Some(other) = theirs.get(&name) {
                bind(ids, kind, ref_id(&record), ref_id(other));
            }
        }
    }
    let remote_by_id: HashMap<&str, HashMap<String, &Value>> =
        KINDS.iter().map(|kind| (*kind, records(remote, kind).filter_map(|r| Some((token(ref_id(r)?), r))).collect())).collect();
    loop {
        let mut changed = false;
        for (kind, field, target) in STRUCTURE {
            for record in records(local, kind) {
                let Some(local_id) = ref_id(record) else { continue };
                let Some(sync_id) = ids.get(kind).and_then(|m| m.get(&token(local_id))).cloned() else { continue };
                let Some(other) = remote_by_id[kind].get(&token(&sync_id)) else { continue };
                let mine = record.get(field).and_then(ref_id);
                let theirs = other.get(field).and_then(ref_id);
                if bind(ids, target, mine, theirs) {
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// The local export in sync form: records without a sync ID get a new one;
/// map entries of records that no longer exist are dropped.
pub fn to_sync_form(snapshot: &Value, ids: &mut Ids) -> Value {
    let mut used: Ids = KINDS.iter().map(|k| (k.to_string(), BTreeMap::new())).collect();
    let mut out = snapshot.clone();
    out["knownHosts"] = json!([]);
    for kind in KINDS {
        let mapped: Vec<Value> = records(snapshot, kind)
            .map(|record| {
                map_ids(record, kind, &mut |kind, id| {
                    let t = token(id);
                    let sync_id = ids.get(kind).and_then(|m| m.get(&t)).cloned().unwrap_or_else(|| Value::String(new_sync_id()));
                    ids.entry(kind.into()).or_default().insert(t.clone(), sync_id.clone());
                    used.entry(kind.into()).or_default().insert(t, sync_id.clone());
                    sync_id
                })
            })
            .collect();
        out[kind] = Value::Array(mapped);
    }
    for kind in KINDS {
        ids.insert(kind.into(), used.remove(kind).unwrap_or_default());
    }
    out
}

/// The map after importing a sync-form snapshot: the importer reports
/// `[sync ID, new local_id]` pairs per collection.
pub fn ids_from_import(imported: &Value) -> Ids {
    let mut ids = Ids::new();
    for kind in KINDS {
        let entry = ids.entry(kind.into()).or_default();
        for pair in imported.get(kind).and_then(Value::as_array).into_iter().flatten() {
            if let (Some(sync_id), Some(local_id)) = (pair.get(0).filter(|v| !v.is_null()), pair.get(1).filter(|v| !v.is_null())) {
                entry.insert(token(local_id), sync_id.clone());
            }
        }
    }
    ids
}

#[derive(Clone, Debug)]
struct Record {
    kind: String,
    name: Option<String>,
    value: Value,
    label: String,
    digest: String,
}

fn records_of(snapshot: Option<&Value>) -> BTreeMap<String, Record> {
    let mut out = BTreeMap::new();
    let Some(snapshot) = snapshot else { return out };
    for kind in KINDS {
        for value in records(snapshot, kind) {
            let Some(id) = ref_id(value) else { continue };
            out.insert(key_of(kind, id), Record { kind: kind.into(), name: None, value: value.clone(), label: label_of(kind, value), digest: sha(&canonical(value, Some(kind))) });
        }
    }
    for (name, value) in snapshot.get("settings").and_then(Value::as_object).into_iter().flatten() {
        let label = crate::log_backup::catalog_label(name, value);
        out.insert(format!("settings|{name}"), Record { kind: "settings".into(), name: Some(name.clone()), value: value.clone(), label: label.unwrap_or_else(|| name.clone()), digest: sha(value) });
    }
    out
}

pub type Digests = BTreeMap<String, String>;

pub fn digests_of(snapshot: Option<&Value>) -> Digests {
    records_of(snapshot).into_iter().map(|(key, record)| (key, record.digest)).collect()
}

fn assemble(template: &Value, records: &BTreeMap<String, Record>) -> Value {
    let mut out = template.clone();
    out["knownHosts"] = json!([]);
    out["settings"] = json!({});
    for kind in KINDS {
        out[kind] = json!([]);
    }
    for record in records.values() {
        if record.kind == "settings" {
            out["settings"][record.name.as_deref().unwrap_or_default()] = record.value.clone();
        } else if let Some(list) = out[record.kind.as_str()].as_array_mut() {
            list.push(record.value.clone());
        }
    }
    out
}

/// Keeps records still referenced by a kept record, though the other side deleted them.
fn restore_references(records: &mut BTreeMap<String, Record>, sources: &[&BTreeMap<String, Record>]) {
    loop {
        let mut changed = false;
        let wanted: Vec<String> = records
            .values()
            .filter(|r| r.kind != "settings")
            .flat_map(|r| references_of(&r.value, &r.kind))
            .filter(|key| !records.contains_key(key))
            .collect();
        for key in wanted {
            if let Some(found) = sources.iter().find_map(|s| s.get(&key)) {
                records.insert(key, found.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn owner_labels(sources: &[&BTreeMap<String, Record>]) -> HashMap<String, String> {
    let mut owners = HashMap::new();
    for records in sources {
        for record in records.values() {
            if !matches!(record.kind.as_str(), "hosts" | "groups") || record.label.is_empty() {
                continue;
            }
            for (field, kind) in [("ssh_config", "sshConfigs"), ("telnet_config", "telnetConfigs")] {
                if let Some(id) = record.value.get(field).and_then(ref_id) {
                    owners.entry(key_of(kind, id)).or_insert_with(|| record.label.clone());
                }
            }
        }
    }
    owners
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictPolicy {
    Preview,
    Automatic,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConflictInfo {
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub text: String,
}

/// One record that a sync would move, for the preview.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Change {
    /// The record goes to the cloud (`true`) or comes to this device.
    pub upload: bool,
    pub kind: String,
    pub label: String,
    /// `added`, `modified` or `removed` on the side it comes from.
    pub action: &'static str,
}

impl Change {
    pub fn text(&self) -> String {
        let action = match self.action {
            "added" => "добавлено",
            "removed" => "удалено",
            _ => "изменено",
        };
        format!("{} «{}»: {}", kind_label(&self.kind), self.label, action)
    }
}

#[derive(Debug)]
pub struct Merged {
    pub changes: Vec<Change>,
    pub conflicts: Vec<ConflictInfo>,
    pub from_local: usize,
    pub from_remote: usize,
    pub local: Value,
    pub remote: Value,
    pub import_needed: bool,
    pub upload_needed: bool,
}

/// What both sides held after the last sync.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Base {
    pub local: Digests,
    pub remote: Digests,
}

fn same_digests(a: &BTreeMap<String, Record>, b: &BTreeMap<String, Record>) -> bool {
    a.len() == b.len() && a.iter().all(|(key, r)| b.get(key).is_some_and(|o| o.digest == r.digest))
}

pub fn merge_snapshots(local: &Value, remote: Option<&Value>, base: Option<&Base>, policy: ConflictPolicy, now: &str) -> Merged {
    let l_records = records_of(Some(local));
    let r_records = records_of(remote);
    let empty = Base::default();
    let base = base.unwrap_or(&empty);
    let (mut to_local, mut to_remote, mut conflicts) = (BTreeMap::new(), BTreeMap::new(), Vec::new());
    let (mut from_local, mut from_remote) = (0, 0);
    let mut changes = Vec::new();
    let owners = owner_labels(&[&l_records, &r_records]);
    let keys: std::collections::BTreeSet<&String> = l_records.keys().chain(r_records.keys()).collect();
    for key in keys {
        let (l, r) = (l_records.get(key), r_records.get(key));
        let choice = if l.map(|x| &x.digest) == r.map(|x| &x.digest) {
            "same"
        } else {
            let local_changed = l.map(|x| &x.digest) != base.local.get(key);
            let remote_changed = r.map(|x| &x.digest) != base.remote.get(key);
            match (local_changed, remote_changed) {
                (true, false) => "local",
                (false, true) => "remote",
                // Neither side changed it since the last sync: an importer difference.
                (false, false) => "same",
                (true, true) => {
                    let item = l.or(r).expect("a key comes from one side");
                    let label = if item.label.is_empty() { owners.get(key).cloned().unwrap_or_else(|| "запись".into()) } else { item.label.clone() };
                    let detail = if l.is_none() {
                        "удалено на устройстве, изменено в облаке"
                    } else if r.is_none() {
                        "изменено на устройстве, удалено в облаке"
                    } else {
                        "изменено на обеих сторонах"
                    };
                    conflicts.push(ConflictInfo {
                        kind: item.kind.clone(),
                        text: format!("Конфликт: {} «{label}»", kind_label(&item.kind)),
                        label,
                        detail: detail.into(),
                    });
                    match policy {
                        ConflictPolicy::Preview => continue,
                        ConflictPolicy::Automatic if r.is_some() => "remote",
                        ConflictPolicy::Automatic => "local",
                    }
                }
            }
        };
        let for_local = if choice == "remote" { r } else { l };
        let for_remote = if choice == "local" { l } else { r };
        match choice {
            "local" => from_local += 1,
            "remote" => from_remote += 1,
            _ => {}
        }
        if choice == "local" || choice == "remote" {
            let (from, to) = if choice == "local" { (l, r) } else { (r, l) };
            let item = from.or(to).expect("a key comes from one side");
            let label = if item.label.is_empty() { owners.get(key).cloned().unwrap_or_else(|| "запись".into()) } else { item.label.clone() };
            let action = match (from, to) {
                (None, _) => "removed",
                (Some(_), None) => "added",
                _ => "modified",
            };
            changes.push(Change { upload: choice == "local", kind: item.kind.clone(), label, action });
        }
        if let Some(record) = for_local {
            to_local.insert(key.clone(), record.clone());
        }
        if let Some(record) = for_remote {
            to_remote.insert(key.clone(), record.clone());
        }
    }
    if policy == ConflictPolicy::Preview && !conflicts.is_empty() {
        return Merged { changes, conflicts, from_local, from_remote, local: Value::Null, remote: Value::Null, import_needed: false, upload_needed: false };
    }
    restore_references(&mut to_local, &[&l_records, &r_records]);
    restore_references(&mut to_remote, &[&r_records, &l_records]);
    let mut template = remote.cloned().unwrap_or_else(|| json!({}));
    for (key, value) in local.as_object().into_iter().flatten() {
        template[key] = value.clone();
    }
    template["createdAt"] = json!(now);
    Merged {
        changes,
        conflicts,
        from_local,
        from_remote,
        local: assemble(&template, &to_local),
        remote: assemble(&template, &to_remote),
        import_needed: !same_digests(&to_local, &l_records),
        upload_needed: remote.is_none() || !same_digests(&to_remote, &r_records),
    }
}

/// Merge state kept on the device between syncs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// SHA-256 of the server URL: a state is only used with its own server.
    pub server: String,
    pub ids: Ids,
    pub local: Digests,
    pub remote: Digests,
}

pub fn server_key(url: &str) -> String {
    Sha256::digest(url.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// The device side of a sync: its vault.
pub trait Device {
    fn export(&mut self) -> Result<Value, String>;
    /// Replaces the portable data; returns `[sync ID, new local_id]` pairs per collection.
    fn import(&mut self, snapshot: &Value, ids: &Ids) -> Result<Value, String>;
    /// Encrypted copy of the local data, taken before an import replaces it.
    fn backup(&mut self, snapshot: &Value) -> Result<(), String>;
}

/// The server side: fetch the decrypted snapshot, upload a new one.
pub trait Server {
    fn fetch_for_sync(&self) -> Result<(Option<Remote>, bool), String>;
    fn upload(&self, snapshot: &Value, condition: Condition) -> Result<Option<String>, String>;
}

impl Server for crate::cloud::Cloud {
    fn fetch_for_sync(&self) -> Result<(Option<Remote>, bool), String> {
        crate::cloud::Cloud::fetch_for_sync(self)
    }

    fn upload(&self, snapshot: &Value, condition: Condition) -> Result<Option<String>, String> {
        crate::cloud::Cloud::upload(self, snapshot, condition)
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub resolved_conflicts: usize,
    pub state: Option<State>,
    pub uploaded: bool,
    pub imported: bool,
    pub from_local: usize,
    pub from_remote: usize,
}

/// One sync round, as `syncOnce` in sync-merge.js.
pub fn sync_once(saved: Option<State>, server_id: &str, device: &mut dyn Device, server: &dyn Server, now: &str) -> Result<Outcome, String> {
    let saved = saved.filter(|s| s.server == server_id);
    let mut ids = saved.as_ref().map(|s| s.ids.clone()).unwrap_or_default();
    let (fetched, can_create) = server.fetch_for_sync()?;
    let remote = match &fetched {
        Some(found) => Some(validate_snapshot(&found.snapshot)?),
        None => None,
    };
    // A snapshot deleted on the server is re-created from this device, not
    // read as "every record was deleted".
    let base = match (&saved, &remote) {
        (Some(saved), Some(_)) => Some(Base { local: saved.local.clone(), remote: saved.remote.clone() }),
        _ => None,
    };
    let exported = device.export()?;
    match_ids(&exported, remote.as_ref(), &mut ids);
    let local = to_sync_form(&exported, &mut ids);
    let merged = merge_snapshots(&local, remote.as_ref(), base.as_ref(), ConflictPolicy::Automatic, now);
    let outgoing = if merged.upload_needed {
        let condition = match (&fetched, &remote) {
            (Some(Remote { revision: Some(revision), .. }), Some(_)) => Condition::Revision(revision.clone()),
            (_, None) if can_create => Condition::Create,
            (_, None) => return Err("Сервер не подтвердил безопасное создание снимка.".into()),
            _ => return Err("Сервер не вернул ревизию снимка. Запись отменена.".into()),
        };
        Some((validate_snapshot(&merged.remote)?, condition))
    } else {
        None
    };
    let incoming = merged.import_needed.then(|| validate_snapshot(&merged.local)).transpose()?;
    if incoming.is_some() {
        device.backup(&exported)?;
    }
    let cloud = if let Some((upload, condition)) = outgoing {
        server.upload(&upload, condition)?;
        Some(upload)
    } else {
        remote
    };
    let mut settled_local = local;
    if let Some(incoming) = incoming {
        let pairs = device.import(&incoming, &ids)?;
        ids = ids_from_import(&pairs);
        let exported = device.export()?;
        match_ids(&exported, Some(&incoming), &mut ids);
        settled_local = to_sync_form(&exported, &mut ids);
    }
    Ok(Outcome {
        resolved_conflicts: merged.conflicts.len(),
        state: Some(State { server: server_id.into(), local: digests_of(Some(&settled_local)), remote: digests_of(cloud.as_ref()), ids }),
        uploaded: merged.upload_needed,
        imported: merged.import_needed,
        from_local: merged.from_local,
        from_remote: merged.from_remote,
    })
}

/// The importer's checks: format, unique ids, references that resolve, no
/// group cycles; embedded copies are replaced with the canonical records.
pub fn validate_snapshot(snapshot: &Value) -> Result<Value, String> {
    let fail = |message: String| Err(format!("Self-cloud snapshot: {message}"));
    let Some(object) = snapshot.as_object() else { return fail("snapshot must be an object".into()) };
    if object.get("format").and_then(Value::as_str) != Some(FORMAT) || object.get("version").and_then(Value::as_u64) != Some(VERSION) {
        return fail(format!("unsupported format/version (expected {FORMAT} v{VERSION})"));
    }
    if !object.get("settings").is_some_and(Value::is_object) {
        return fail("snapshot.settings must be an object".into());
    }
    let mut indexes: HashMap<&str, HashMap<String, Value>> = HashMap::new();
    for name in COLLECTIONS {
        let Some(list) = object.get(name).and_then(Value::as_array) else { return fail(format!("snapshot.{name} must be an array")) };
        let list: &[Value] = if name == "knownHosts" { &[] } else { list };
        let mut ids = HashMap::new();
        for (index, value) in list.iter().enumerate() {
            if !value.is_object() {
                return fail(format!("{name}[{index}] must be an object"));
            }
            let valid = match ref_id(value) {
                Some(Value::Number(n)) => n.as_i64().is_some_and(|n| n.unsigned_abs() < 1 << 53),
                Some(Value::String(s)) => !s.is_empty(),
                _ => false,
            };
            if !valid {
                return fail(format!("{name}[{index}].local_id must be a safe integer or non-empty string"));
            }
            if let Some(field) = ["id", "global_id", "status"].iter().find(|f| value.get(**f).is_some()) {
                return fail(format!("{name}[{index}] contains official-sync field {field}"));
            }
            let t = token(ref_id(value).expect("checked"));
            if ids.insert(t, value.clone()).is_some() {
                return fail(format!("{name} contains duplicate id {}", ref_id(value).expect("checked")));
            }
        }
        indexes.insert(name, ids);
    }
    let has = |kind: &str, value: Option<&Value>, path: &str, optional: bool| -> Result<(), String> {
        let value = value.filter(|v| !v.is_null());
        match value.and_then(ref_id) {
            None if value.is_some() || !optional => Err(format!("Self-cloud snapshot: {path} is missing a valid local_id reference")),
            None => Ok(()),
            Some(id) if !indexes[kind].contains_key(&token(id)) => Err(format!("Self-cloud snapshot: {path} references missing {kind} id {id}")),
            Some(_) => Ok(()),
        }
    };
    let config_refs = |config: Option<&Value>, path: &str| -> Result<(), String> {
        let Some(config) = config.filter(|c| !c.is_null()) else { return Ok(()) };
        if !config.is_object() {
            return Err(format!("Self-cloud snapshot: {path} must be an object"));
        }
        has("identities", config.get("identity"), &format!("{path}.identity"), true)?;
        has("proxies", config.get("proxy"), &format!("{path}.proxy"), true)?;
        has("snippets", config.get("startup_snippet"), &format!("{path}.startup_snippet"), true)?;
        if let Some(chain) = config.get("host_chain").filter(|c| !c.is_null()) {
            let Some(hosts) = chain.get("hosts_chain").and_then(Value::as_array) else {
                return Err(format!("Self-cloud snapshot: {path}.host_chain.hosts_chain must be an array"));
            };
            for (i, host) in hosts.iter().enumerate() {
                has("hosts", Some(host), &format!("{path}.host_chain.hosts_chain[{i}]"), false)?;
            }
        }
        Ok(())
    };
    let list = |name: &str| object.get(name).and_then(Value::as_array).cloned().unwrap_or_default();
    for (i, x) in list("identities").iter().enumerate() {
        has("sshKeys", x.get("ssh_key"), &format!("identities[{i}].ssh_key"), true)?;
    }
    for (i, x) in list("proxies").iter().enumerate() {
        has("identities", x.get("identity"), &format!("proxies[{i}].identity"), true)?;
    }
    for name in ["sshConfigs", "telnetConfigs"] {
        for (i, x) in list(name).iter().enumerate() {
            config_refs(Some(x), &format!("{name}[{i}]"))?;
        }
    }
    for (name, parent) in [("groups", "parent_group"), ("hosts", "group")] {
        for (i, x) in list(name).iter().enumerate() {
            has("groups", x.get(parent), &format!("{name}[{i}].{parent}"), true)?;
            has("sshConfigs", x.get("ssh_config"), &format!("{name}[{i}].ssh_config"), true)?;
            has("telnetConfigs", x.get("telnet_config"), &format!("{name}[{i}].telnet_config"), true)?;
            config_refs(x.get("ssh_config"), &format!("{name}[{i}].ssh_config"))?;
            config_refs(x.get("telnet_config"), &format!("{name}[{i}].telnet_config"))?;
        }
    }
    for (i, x) in list("snippets").iter().enumerate() {
        has("snippetPackages", x.get("package"), &format!("snippets[{i}].package"), true)?;
        if let Some(targets) = x.get("targetHosts").filter(|t| !t.is_null()) {
            let Some(targets) = targets.as_array() else { return fail(format!("snippets[{i}].targetHosts must be an array")) };
            for (j, host) in targets.iter().enumerate() {
                has("hosts", Some(host), &format!("snippets[{i}].targetHosts[{j}]"), false)?;
            }
        }
    }
    for (i, x) in list("portForwardingRules").iter().enumerate() {
        has("hosts", x.get("host"), &format!("portForwardingRules[{i}].host"), false)?;
    }
    for (i, x) in list("workspaceTemplates").iter().enumerate() {
        let Some(terminals) = x.get("workspaceTerminals").and_then(Value::as_array) else {
            return fail(format!("workspaceTemplates[{i}].workspaceTerminals must be an array"));
        };
        for (j, terminal) in terminals.iter().enumerate() {
            has("hosts", terminal.get("host"), &format!("workspaceTemplates[{i}].workspaceTerminals[{j}].host"), false)?;
        }
    }
    // Parent cycles would make import ordering unsafe.
    for group in list("groups") {
        let mut seen = HashSet::new();
        let mut current = Some(group);
        while let Some(g) = current {
            let id = token(ref_id(&g).expect("checked"));
            if !seen.insert(id.clone()) {
                return fail(format!("group parent cycle at id {}", ref_id(&g).expect("checked")));
            }
            current = g.get("parent_group").and_then(ref_id).and_then(|p| indexes["groups"].get(&token(p)).cloned());
        }
    }

    // One canonical representation per relationship.
    let canonical_of = |kind: &str, value: &Value, minimal: bool| -> Value {
        let Some(id) = ref_id(value) else { return Value::Null };
        let found = indexes[kind].get(&token(id)).cloned().unwrap_or(Value::Null);
        if minimal { json!({"local_id": found.get("local_id").cloned().unwrap_or(Value::Null)}) } else { found }
    };
    let normalize_config = |config: &mut Value| {
        for (field, kind, minimal) in [("identity", "identities", false), ("proxy", "proxies", false), ("startup_snippet", "snippets", true)] {
            if let Some(value) = config.get(field).filter(|v| !v.is_null()).cloned() {
                config[field] = canonical_of(kind, &value, minimal);
            }
        }
        if let Some(Value::Array(chain)) = config.get("host_chain").and_then(|c| c.get("hosts_chain")).cloned() {
            config["host_chain"]["hosts_chain"] = Value::Array(chain.iter().map(|h| canonical_of("hosts", h, true)).collect());
        }
    };
    let mut out = snapshot.clone();
    out["knownHosts"] = json!([]);
    for x in out["identities"].as_array_mut().into_iter().flatten() {
        if let Some(key) = x.get("ssh_key").filter(|v| !v.is_null()).cloned() {
            x["ssh_key"] = canonical_of("sshKeys", &key, true);
        }
    }
    for x in out["proxies"].as_array_mut().into_iter().flatten() {
        if let Some(identity) = x.get("identity").filter(|v| !v.is_null()).cloned() {
            x["identity"] = canonical_of("identities", &identity, false);
        }
    }
    for name in ["sshConfigs", "telnetConfigs"] {
        for x in out[name].as_array_mut().into_iter().flatten() {
            normalize_config(x);
        }
    }
    let linked = |kind: &str, config: &Value| -> Value {
        let mut result = canonical_of(kind, config, false);
        // A config's own protocol status is semantic and is kept.
        if let (Some(status), true) = (config.get("status"), result.is_object()) {
            result["status"] = status.clone();
        }
        result
    };
    for (name, parent) in [("groups", "parent_group"), ("hosts", "group")] {
        for x in out[name].as_array_mut().into_iter().flatten() {
            if let Some(p) = x.get(parent).filter(|v| !v.is_null()).cloned() {
                x[parent] = canonical_of("groups", &p, true);
            }
            for (field, kind) in [("ssh_config", "sshConfigs"), ("telnet_config", "telnetConfigs")] {
                if let Some(config) = x.get(field).filter(|v| !v.is_null()).cloned() {
                    x[field] = linked(kind, &config);
                }
            }
        }
    }
    for x in out["snippets"].as_array_mut().into_iter().flatten() {
        if let Some(package) = x.get("package").filter(|v| !v.is_null()).cloned() {
            x["package"] = canonical_of("snippetPackages", &package, true);
        }
        if let Some(Value::Array(targets)) = x.get("targetHosts").cloned() {
            x["targetHosts"] = Value::Array(targets.iter().map(|h| canonical_of("hosts", h, true)).collect());
        }
    }
    for x in out["portForwardingRules"].as_array_mut().into_iter().flatten() {
        let host = x.get("host").cloned().unwrap_or(Value::Null);
        x["host"] = canonical_of("hosts", &host, true);
    }
    for x in out["workspaceTemplates"].as_array_mut().into_iter().flatten() {
        for terminal in x["workspaceTerminals"].as_array_mut().into_iter().flatten() {
            let host = terminal.get("host").cloned().unwrap_or(Value::Null);
            terminal["host"] = canonical_of("hosts", &host, true);
        }
    }
    Ok(out)
}

/// A snapshot with nothing in it.
pub fn empty_snapshot(now: &str) -> Value {
    let mut out = json!({"format": FORMAT, "version": VERSION, "createdAt": now, "settings": {}});
    for name in COLLECTIONS {
        out[name] = json!([]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const NOW: &str = "2026-11-13T00:00:00.000Z";

    /// A device like the legacy test's: numeric local IDs that change on every import.
    struct TestDevice {
        name: &'static str,
        next: i64,
        data: Value,
        saved: Option<State>,
        backups: usize,
    }

    impl TestDevice {
        fn new(name: &'static str, start: i64) -> Self {
            Self { name, next: start, data: empty_snapshot(NOW), saved: None, backups: 0 }
        }
        fn id(&mut self) -> i64 {
            self.next += 1;
            self.next - 1
        }
        fn add_host(&mut self, label: &str, port: i64, group: Option<Value>) {
            let config = json!({"local_id": self.id(), "port": port, "identity": null, "proxy": null, "startup_snippet": null, "host_chain": null});
            self.data["sshConfigs"].as_array_mut().unwrap().push(config.clone());
            let host = json!({"local_id": self.id(), "label": label, "address": format!("{label}.lan"),
                "group": group.map(|g| json!({"local_id": g["local_id"]})), "ssh_config": config, "telnet_config": null});
            self.data["hosts"].as_array_mut().unwrap().push(host);
        }
        fn add_group(&mut self, label: &str) -> Value {
            let group = json!({"local_id": self.id(), "label": label, "parent_group": null, "ssh_config": null, "telnet_config": null});
            self.data["groups"].as_array_mut().unwrap().push(group.clone());
            group
        }
        fn host_mut(&mut self, label: &str) -> &mut Value {
            self.data["hosts"].as_array_mut().unwrap().iter_mut().find(|h| h["label"] == label).unwrap()
        }
        fn labels(&self, kind: &str) -> Vec<String> {
            let mut out: Vec<String> = records(&self.data, kind).map(|r| r["label"].as_str().unwrap().to_owned()).collect();
            out.sort();
            out
        }
    }

    impl Device for TestDevice {
        fn export(&mut self) -> Result<Value, String> {
            validate_snapshot(&self.data)
        }
        fn import(&mut self, snapshot: &Value, _: &Ids) -> Result<Value, String> {
            let mut pairs = json!({});
            let mut table = Ids::new();
            for kind in KINDS {
                let mut list = Vec::new();
                for record in records(snapshot, kind) {
                    let id = json!(self.id());
                    list.push(json!([record["local_id"], id]));
                    table.entry(kind.into()).or_default().insert(token(&record["local_id"]), id);
                }
                pairs[kind] = Value::Array(list);
            }
            self.data = validate_snapshot(&to_sync_form(snapshot, &mut table))?;
            Ok(pairs)
        }
        fn backup(&mut self, _: &Value) -> Result<(), String> {
            self.backups += 1;
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestServer {
        snapshot: RefCell<Option<Value>>,
        revision: RefCell<u64>,
        uploads: RefCell<usize>,
        /// Another device uploads right before ours.
        race: bool,
    }

    impl Server for TestServer {
        fn fetch_for_sync(&self) -> Result<(Option<Remote>, bool), String> {
            Ok((self.snapshot.borrow().clone().map(|snapshot| Remote { snapshot, revision: Some(format!("r{}", self.revision.borrow())) }), true))
        }
        fn upload(&self, snapshot: &Value, condition: Condition) -> Result<Option<String>, String> {
            if self.race {
                *self.revision.borrow_mut() += 1;
            }
            let current = format!("r{}", self.revision.borrow());
            let refused = match &condition {
                Condition::Create => self.snapshot.borrow().is_some(),
                Condition::Revision(revision) => *revision != current,
                Condition::Any => false,
            };
            if refused {
                return Err(format!("{} (HTTP 412)", crate::cloud::REVISION_CHANGED));
            }
            *self.snapshot.borrow_mut() = Some(snapshot.clone());
            *self.revision.borrow_mut() += 1;
            *self.uploads.borrow_mut() += 1;
            Ok(Some(format!("r{}", self.revision.borrow())))
        }
    }

    fn sync(device: &mut TestDevice, server: &TestServer) -> Outcome {
        let saved = device.saved.clone();
        let outcome = sync_once(saved, "server", device, server, NOW).unwrap();
        device.saved = outcome.state.clone();
        outcome
    }

    #[test]
    fn devices_creating_different_hosts_are_merged_without_duplicates() {
        let cloud = TestServer::default();
        let (mut a, mut b) = (TestDevice::new("A", 1), TestDevice::new("B", 500));
        a.add_host("web", 22, None);
        sync(&mut a, &cloud);
        let snapshot = cloud.snapshot.borrow().clone().unwrap();
        assert_eq!(snapshot["hosts"].as_array().unwrap().len(), 1);
        assert!(snapshot["hosts"][0]["local_id"].is_string(), "the cloud keeps stable sync IDs");

        sync(&mut b, &cloud);
        assert_eq!(b.labels("hosts"), ["web"]);

        a.add_host("api", 22, None);
        b.add_host("db", 22, None);
        assert!(sync(&mut a, &cloud).uploaded);
        let from_b = sync(&mut b, &cloud);
        assert_eq!((from_b.from_local, from_b.from_remote), (2, 2), "db (+config) up, api (+config) down");
        sync(&mut a, &cloud);
        assert_eq!(a.labels("hosts"), ["api", "db", "web"]);
        assert_eq!(b.labels("hosts"), ["api", "db", "web"]);
        let snapshot = cloud.snapshot.borrow().clone().unwrap();
        assert_eq!((snapshot["hosts"].as_array().unwrap().len(), snapshot["sshConfigs"].as_array().unwrap().len()), (3, 3));

        let uploads = *cloud.uploads.borrow();
        for device in [&mut a, &mut b] {
            let again = sync(device, &cloud);
            assert_eq!((again.uploaded, again.imported), (false, false), "{}", device.name);
        }
        assert_eq!(*cloud.uploads.borrow(), uploads, "converged: nothing more to send");
    }

    #[test]
    fn sync_uses_cloud_versions_only_for_records_changed_on_both_sides() {
        let cloud = TestServer::default();
        let (mut a, mut b) = (TestDevice::new("A", 1), TestDevice::new("B", 500));
        a.add_host("web", 22, None);
        a.add_host("db", 22, None);
        sync(&mut a, &cloud);
        sync(&mut b, &cloud);

        // A changes web's port (its SSH config), B renames db.
        let config_id = a.host_mut("web")["ssh_config"]["local_id"].clone();
        for config in a.data["sshConfigs"].as_array_mut().unwrap() {
            if config["local_id"] == config_id {
                config["port"] = json!(2222);
            }
        }
        a.host_mut("web")["ssh_config"]["port"] = json!(2222);
        b.host_mut("db")["label"] = json!("database");
        sync(&mut a, &cloud);
        assert_eq!(sync(&mut b, &cloud).resolved_conflicts, 0);
        let web_config = b.host_mut("web")["ssh_config"]["local_id"].clone();
        let port = records(&b.data, "sshConfigs").find(|c| c["local_id"] == web_config).unwrap()["port"].clone();
        assert_eq!(port, 2222);
        sync(&mut a, &cloud);
        assert_eq!(a.labels("hosts"), ["database", "web"]);

        a.host_mut("web")["label"] = json!("web-a");
        b.host_mut("web")["label"] = json!("web-b");
        sync(&mut a, &cloud);
        b.host_mut("database")["label"] = json!("db-b");
        let backups = b.backups;
        assert_eq!(sync(&mut b, &cloud).resolved_conflicts, 1);
        assert_eq!(b.labels("hosts"), ["db-b", "web-a"]);
        assert_eq!(b.backups, backups + 1);
        sync(&mut a, &cloud);
        assert_eq!(a.labels("hosts"), ["db-b", "web-a"]);
        assert_eq!(sync(&mut b, &cloud).resolved_conflicts, 0);
    }

    #[test]
    fn automatic_sync_keeps_an_edit_over_a_concurrent_deletion() {
        let mut previous = empty_snapshot(NOW);
        previous["settings"]["value"] = json!(1);
        let digests = digests_of(Some(&previous));
        let base = Base { local: digests.clone(), remote: digests };
        let mut edited = previous;
        edited["settings"]["value"] = json!(2);
        let deleted = empty_snapshot(NOW);

        for (local, remote) in [(&edited, &deleted), (&deleted, &edited)] {
            let merged = merge_snapshots(local, Some(remote), Some(&base), ConflictPolicy::Automatic, NOW);
            assert_eq!(merged.conflicts.len(), 1);
            assert_eq!(merged.local["settings"]["value"], 2);
            assert_eq!(merged.remote["settings"]["value"], 2);
        }
    }

    #[test]
    fn deletions_propagate_and_a_record_still_referenced_is_kept() {
        let cloud = TestServer::default();
        let (mut a, mut b) = (TestDevice::new("A", 1), TestDevice::new("B", 500));
        a.add_group("prod");
        a.add_host("web", 22, None);
        a.add_host("old", 22, None);
        sync(&mut a, &cloud);
        sync(&mut b, &cloud);

        a.data["hosts"].as_array_mut().unwrap().retain(|h| h["label"] != "old");
        a.data["groups"] = json!([]);
        let prod = records(&b.data, "groups").find(|g| g["label"] == "prod").cloned();
        b.add_host("db", 22, prod);
        sync(&mut a, &cloud);
        assert_eq!(sync(&mut b, &cloud).resolved_conflicts, 0);
        assert_eq!(b.labels("hosts"), ["db", "web"]);
        assert_eq!(b.labels("groups"), ["prod"], "the group db needs survives the deletion");
        sync(&mut a, &cloud);
        assert_eq!(a.labels("hosts"), ["db", "web"]);
        assert_eq!(a.labels("groups"), ["prod"]);
    }

    #[test]
    fn first_merge_sync_matches_records_both_devices_already_have_by_name() {
        let cloud = TestServer::default();
        let (mut a, mut b) = (TestDevice::new("A", 1), TestDevice::new("B", 500));
        a.add_host("web", 22, None);
        b.add_host("web", 22, None);
        b.add_host("extra", 22, None);
        // An older client uploaded A's data with plain numeric IDs.
        *cloud.snapshot.borrow_mut() = Some(validate_snapshot(&a.data).unwrap());
        assert_eq!(sync(&mut b, &cloud).resolved_conflicts, 0);
        let snapshot = cloud.snapshot.borrow().clone().unwrap();
        let mut hosts: Vec<_> = records(&snapshot, "hosts").map(|h| h["label"].as_str().unwrap().to_owned()).collect();
        hosts.sort();
        assert_eq!(hosts, ["extra", "web"]);
        assert_eq!(snapshot["sshConfigs"].as_array().unwrap().len(), 2, "web's SSH config is matched through the host");

        let mut c = TestDevice::new("C", 900);
        c.add_host("web", 2200, None);
        assert_eq!(sync(&mut c, &cloud).resolved_conflicts, 1);
        assert_eq!(c.host_mut("web")["ssh_config"]["port"], 22);
    }

    #[test]
    fn a_snapshot_changed_during_the_sync_is_not_overwritten() {
        let mut cloud = TestServer::default();
        let mut a = TestDevice::new("A", 1);
        a.add_host("web", 22, None);
        sync(&mut a, &cloud);
        a.add_host("api", 22, None);
        cloud.race = true;
        let saved = a.saved.clone();
        let error = sync_once(saved, "server", &mut a, &cloud, NOW).unwrap_err();
        assert!(error.contains("изменился во время синхронизации"), "{error}");
        assert_eq!(cloud.snapshot.borrow().as_ref().unwrap()["hosts"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_deleted_server_snapshot_is_recreated_instead_of_deleting_local_data() {
        let cloud = TestServer::default();
        let mut a = TestDevice::new("A", 1);
        a.add_host("web", 22, None);
        sync(&mut a, &cloud);
        *cloud.snapshot.borrow_mut() = None;
        sync(&mut a, &cloud);
        assert_eq!(a.labels("hosts"), ["web"]);
        assert_eq!(cloud.snapshot.borrow().as_ref().unwrap()["hosts"].as_array().unwrap().len(), 1);
        assert_eq!(a.backups, 0, "nothing was imported, so no backup was needed");
    }

    #[test]
    fn settings_merge_per_key() {
        let local = json!({"format": FORMAT, "version": 1, "settings": {"font": "A", "theme": "dark"}});
        let remote = json!({"format": FORMAT, "version": 1, "settings": {"font": "A", "theme": "light", "size": 14}});
        assert!(!merge_snapshots(&local, Some(&local), None, ConflictPolicy::Preview, NOW).upload_needed);
        let base = Base { local: digests_of(Some(&local)), remote: digests_of(Some(&local)) };
        let changed = json!({"format": FORMAT, "version": 1, "settings": {"font": "B", "theme": "dark"}});
        let merged = merge_snapshots(&changed, Some(&remote), Some(&base), ConflictPolicy::Preview, NOW);
        assert!(merged.conflicts.is_empty());
        assert_eq!(merged.local["settings"], json!({"font": "B", "theme": "light", "size": 14}));
        assert_eq!(merged.remote["settings"], json!({"font": "B", "theme": "light", "size": 14}));
    }

    #[test]
    fn two_devices_sync_through_the_encrypted_http_transport() {
        let (url, stored) = crate::cloud::tests::server();
        let cloud = crate::cloud::tests::config(&url).validate().unwrap();
        let (mut a, mut b) = (TestDevice::new("A", 1), TestDevice::new("B", 500));
        a.add_host("web", 22, None);
        sync_with(&mut a, &cloud);
        b.add_host("db", 22, None);
        sync_with(&mut b, &cloud);
        sync_with(&mut a, &cloud);
        assert_eq!(a.labels("hosts"), ["db", "web"]);
        assert_eq!(b.labels("hosts"), ["db", "web"]);
        let raw = stored.lock().unwrap().clone().unwrap().0;
        assert!(!String::from_utf8_lossy(&raw).contains("web.lan"), "the server only sees ciphertext");
    }

    fn sync_with(device: &mut TestDevice, cloud: &crate::cloud::Cloud) {
        let saved = device.saved.clone();
        let outcome = sync_once(saved, "server", device, cloud, NOW).unwrap();
        assert_eq!(outcome.resolved_conflicts, 0);
        device.saved = outcome.state;
    }

    #[test]
    fn broken_snapshots_are_refused() {
        let mut snapshot = empty_snapshot(NOW);
        snapshot["hosts"] = json!([{"local_id": "h", "label": "x", "group": {"local_id": "missing"}}]);
        assert!(validate_snapshot(&snapshot).unwrap_err().contains("references missing groups id"));
        snapshot["hosts"] = json!([]);
        snapshot["groups"] = json!([{"local_id": 1, "parent_group": {"local_id": 2}}, {"local_id": 2, "parent_group": {"local_id": 1}}]);
        assert!(validate_snapshot(&snapshot).unwrap_err().contains("group parent cycle"));
        assert!(validate_snapshot(&json!({"format": "other"})).is_err());
        let mut status = empty_snapshot(NOW);
        status["hosts"] = json!([{"local_id": 1, "status": "x"}]);
        assert!(validate_snapshot(&status).unwrap_err().contains("official-sync field status"));
    }

    #[test]
    fn embedded_copies_are_replaced_with_the_canonical_record() {
        let mut snapshot = empty_snapshot(NOW);
        snapshot["sshConfigs"] = json!([{"local_id": "c", "port": 2222}]);
        snapshot["hosts"] = json!([{"local_id": "h", "label": "web", "ssh_config": {"local_id": "c", "port": 1}}]);
        let normalized = validate_snapshot(&snapshot).unwrap();
        assert_eq!(normalized["hosts"][0]["ssh_config"]["port"], 2222);
    }
}
