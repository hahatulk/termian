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
    /// The host's own password (in legacy, a hidden identity).
    #[serde(default)]
    pub password: Option<String>,
    /// Key from the Keychain to log in with.
    #[serde(default)]
    pub key: Option<u64>,
    /// Identity from the Keychain: its username, password and key replace the host's own.
    #[serde(default)]
    pub identity: Option<u64>,
    /// The legacy host form's other SSH settings.
    #[serde(default)]
    pub settings: HostSettings,
}

/// Legacy host settings: `backspace` on the host, the rest on its SSH config.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HostSettings {
    /// «Backspace: Ctrl+H» (legacy `"legacy"`); otherwise DEL is sent.
    #[serde(default)]
    pub ctrl_h_backspace: bool,
    #[serde(default)]
    pub agent_forwarding: bool,
    /// `None` is UTF-8.
    #[serde(default)]
    pub charset: Option<String>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Snippet run once the shell is ready («Startup Command»).
    #[serde(default)]
    pub startup_snippet: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_command: Option<String>,
    /// The host's terminal theme as legacy stores it (a name or an object with `name`).
    #[serde(default)]
    pub color_scheme: Option<serde_json::Value>,
}

impl HostSettings {
    pub fn set_startup_command(&mut self, command: String) {
        self.startup_snippet = None;
        self.startup_command = (!command.is_empty()).then_some(command);
    }

    pub fn startup_text<'a>(&'a self, vault: &'a Vault) -> &'a str {
        self.startup_command.as_deref()
            .or_else(|| self.startup_snippet.and_then(|id| vault.snippet(id)).map(|s| s.script.as_str()))
            .unwrap_or_default()
    }

    pub fn ssh_startup(&self, vault: &Vault, explicit: Option<String>) -> Option<String> {
        explicit.or_else(|| {
            let text = self.startup_text(vault);
            (!text.trim().is_empty()).then(|| text.to_owned())
        })
    }

    /// The theme name, when the host has its own.
    pub fn theme_name(&self) -> Option<&str> {
        match self.color_scheme.as_ref()? {
            serde_json::Value::String(name) => Some(name.as_str()),
            value => value.get("name")?.as_str(),
        }
        .filter(|n| !n.is_empty())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SshKey {
    pub id: u64,
    pub label: String,
    /// Algorithm as shown on the card: `ED25519`, `ECDSA`, `RSA`.
    pub kind: String,
    /// Decrypted from the legacy profile on import, or set in this client.
    #[serde(default)]
    pub private_key: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
    /// OpenSSH user certificate for this key (legacy `ssh_certificates`, the first one).
    #[serde(default)]
    pub certificate: Option<String>,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Identity {
    pub id: u64,
    pub label: String,
    pub has_password: bool,
    pub has_key: bool,
    #[serde(default)]
    pub username: String,
    /// Decrypted from the legacy profile on import, or set in this client.
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub key: Option<u64>,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    pub id: u64,
    /// «Action description»; the first line of the script stands in when empty.
    pub label: String,
    pub script: String,
    #[serde(default)]
    pub package: Option<u64>,
    /// «Targets»: hosts «Run» opens the snippet on without asking.
    #[serde(default)]
    pub targets: Vec<HostId>,
}

impl Snippet {
    /// The legacy `displayLabel`: the label, or the script's first non-empty line.
    pub fn display_label(&self) -> &str {
        if !self.label.is_empty() {
            return &self.label;
        }
        self.script.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("")
    }

    /// The card shows the script under the label only when they differ.
    pub fn subtitle(&self) -> Option<&str> {
        let script = self.script.trim();
        (!script.is_empty() && script != self.display_label().trim()).then_some(script)
    }
}

/// A command from Shell History (legacy `HistoryCommand`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShellCommand {
    pub script: String,
    /// Unix seconds; 0 when unknown.
    pub completed_at: i64,
    pub rating: i64,
    /// The saved host it ran on (`None`: local, unsaved or imported).
    #[serde(default)]
    pub host: Option<HostId>,
}

/// Shell History keeps this many commands.
const SHELL_HISTORY_LIMIT: usize = 2000;

/// A snippet package: a folder of snippets on the Snippets screen.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SnippetPackage {
    pub id: u64,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnownHost {
    pub id: u64,
    /// `host` or `[host]:port`, as in OpenSSH known_hosts.
    pub host: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleKind {
    Local,
    Remote,
    Dynamic,
}

/// A port forwarding rule, with the legacy field meanings: `port` is the
/// local port (Local, Dynamic) or the port opened on the server (Remote).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForwardingRule {
    pub id: u64,
    pub label: String,
    pub kind: RuleKind,
    pub host: Option<HostId>,
    pub bind_address: String,
    pub port: Option<u16>,
    pub destination: String,
    pub destination_port: Option<u16>,
}

impl ForwardingRule {
    pub fn new(id: u64, kind: RuleKind) -> Self {
        let label = match kind {
            RuleKind::Local => "Local Rule",
            RuleKind::Remote => "Remote Rule",
            RuleKind::Dynamic => "Dynamic Rule",
        };
        Self { id, label: label.into(), kind, host: None, bind_address: "127.0.0.1".into(), port: None, destination: String::new(), destination_port: None }
    }

    /// Everything the rule needs to run, as the legacy `isValid`.
    pub fn is_valid(&self) -> bool {
        let base = self.host.is_some() && self.port.is_some();
        match self.kind {
            RuleKind::Dynamic => base,
            RuleKind::Local | RuleKind::Remote => base && !self.destination.trim().is_empty() && self.destination_port.is_some(),
        }
    }

    /// `From home lab:8080 to 10.0.0.5:80` (Dynamic: just the From part).
    pub fn rule_text(&self, vault: &Vault) -> String {
        let host = self.host.and_then(|h| vault.host(h)).map(|h| h.display_label().to_owned()).unwrap_or_default();
        let port = self.port.map(|p| p.to_string()).unwrap_or_default();
        let from = format!("From {host}:{port}");
        match self.kind {
            RuleKind::Dynamic => from,
            _ => format!("{from} to {}:{}", self.destination, self.destination_port.map(|p| p.to_string()).unwrap_or_default()),
        }
    }

    /// Card title: the label, or the rule when the label is empty.
    pub fn title(&self, vault: &Vault) -> String {
        if self.label.trim().is_empty() && self.is_valid() { self.rule_text(vault) } else { self.label.clone() }
    }
}

/// Where a session's encrypted log is and its key (base64), as the legacy
/// `session_log_data`. Kept only inside the encrypted vault.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogRef {
    pub name: String,
    pub key: String,
}

/// One connection: listed by New Tab (Recent connections) and Logs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    #[serde(default)]
    pub id: u64,
    /// `None` for a local terminal.
    pub host: Option<HostId>,
    /// Seconds since the Unix epoch.
    pub connected_at: i64,
    #[serde(default)]
    pub disconnected_at: Option<i64>,
    /// Name shown in Logs, kept even if the host is removed later.
    #[serde(default)]
    pub label: String,
    /// `ssh, root` or `local`.
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub log_os: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub remote_log: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub connected: bool,
    #[serde(default)]
    pub log: Option<LogRef>,
    /// The Saved (bookmark) mark in Logs.
    #[serde(default)]
    pub saved: bool,
    #[serde(default)]
    pub comment: String,
}

/// The legacy Saved-comment limit, measured in UTF-16 code units.
pub const LOG_COMMENT_LIMIT: usize = 2000;

/// Bounds a comment without splitting a Unicode scalar at the limit.
pub fn bounded_log_comment(comment: &str) -> &str {
    let mut units = 0;
    let end = comment.char_indices().find_map(|(offset, ch)| {
        units += ch.len_utf16();
        (units > LOG_COMMENT_LIMIT).then_some(offset)
    }).unwrap_or(comment.len());
    &comment[..end]
}

/// How many connections are remembered.
const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default)]
    pub retired_host_id: u64,
    pub groups: Vec<Group>,
    pub hosts: Vec<Host>,
    pub keys: Vec<SshKey>,
    pub identities: Vec<Identity>,
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub packages: Vec<SnippetPackage>,
    pub known_hosts: Vec<KnownHost>,
    /// Newest first.
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    #[serde(default)]
    pub rules: Vec<ForwardingRule>,
    /// Portable JSON of each record as last received from a snapshot, keyed
    /// `kind|id`: fields this client does not model survive a sync.
    #[serde(default)]
    pub portable: std::collections::BTreeMap<String, serde_json::Value>,
    /// Snapshot collections and settings this client does not model, kept as they are.
    #[serde(default)]
    pub foreign: serde_json::Value,
    /// Passwords and private keys were not loaded (read from the legacy
    /// profile): uploading this data would erase them in the cloud.
    #[serde(default)]
    pub secrets_missing: bool,
    /// Settings → Logs → «Disabled»: sessions are not recorded. Device-local,
    /// as legacy keeps log settings out of the snapshot.
    #[serde(default)]
    pub logs_disabled: bool,
    #[serde(default)]
    pub update_channel: crate::updater::Channel,
    #[serde(default)]
    pub language: crate::language::Language,
    #[serde(default)]
    pub update_schedule: crate::updater::Schedule,
    #[serde(default)]
    pub terminal: crate::terminal_settings::TerminalSettings,
    /// Connections and their logs were read from the legacy profile.
    #[serde(default)]
    pub legacy_history_imported: bool,
    /// Shell History, newest first; device-local like legacy.
    #[serde(default)]
    pub shell_history: Vec<ShellCommand>,
    #[serde(default)]
    pub legacy_shell_history_imported: bool,
    #[serde(default)]
    pub legacy_templates_imported: bool,
    /// Saved marks and comments were read for history imported before they were.
    #[serde(default)]
    pub legacy_bookmarks_imported: bool,
    /// Hosts' own credentials imported as Keychain identities were folded back.
    #[serde(default)]
    pub legacy_hidden_identities_folded: bool,
    /// Hidden legacy identities no host uses were removed from the Keychain.
    #[serde(default)]
    pub legacy_unused_hidden_identities_dropped: bool,
    /// Read from the legacy profile only: ids of its hosts' own (hidden) identities.
    #[serde(skip)]
    pub hidden_identity_ids: Vec<u64>,
    /// Vaults toolbar view and sorting per section; device-local like legacy.
    #[serde(default)]
    pub listing: crate::listing::ListingSettings,
    /// Settings → Shortcuts; device-local like legacy.
    #[serde(default)]
    pub shortcuts: crate::shortcuts::ShortcutSettings,
    #[serde(default)]
    #[serde(skip_serializing_if = "crate::cloud::CloudConfig::workspace_config")]
    pub cloud: Option<crate::cloud::CloudConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_password: Option<crate::privacy_password::PrivacyPassword>,
    #[serde(default)]
    pub sync_state: Option<crate::sync::State>,
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

    /// Adds a connection to the top of the history; returns its id.
    pub fn record_connection(&mut self, mut entry: HistoryEntry) -> u64 {
        entry.id = self.history.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        let id = entry.id;
        self.history.insert(0, entry);
        let catalog = &self.foreign["settings"];
        let is_local = |entry: &HistoryEntry| !crate::log_backup::catalogued(catalog, entry);
        let excess = self.history.iter().filter(|entry| is_local(entry)).count().saturating_sub(HISTORY_LIMIT);
        for _ in 0..excess {
            let Some(oldest) = self.history.iter().rposition(|e| !e.saved && is_local(e)) else { break };
            if oldest == 0 { break; }
            self.history.remove(oldest);
        }
        id
    }

    /// Passwords and keys from the legacy profile for a vault imported before
    /// they were read. Records match by id and name; a value already set
    /// here is kept.
    pub fn import_legacy_secrets(&mut self, legacy: &Vault) {
        for key in &mut self.keys {
            if let Some(from) = legacy.keys.iter().find(|k| k.id == key.id && k.label == key.label) {
                key.private_key = key.private_key.take().or_else(|| from.private_key.clone());
                key.public_key = key.public_key.take().or_else(|| from.public_key.clone());
                key.passphrase = key.passphrase.take().or_else(|| from.passphrase.clone());
            }
        }
        for identity in &mut self.identities {
            if let Some(from) = legacy.identities.iter().find(|i| i.id == identity.id && i.label == identity.label) {
                identity.password = identity.password.take().or_else(|| from.password.clone());
                identity.key = identity.key.or(from.key);
            }
        }
        for host in &mut self.hosts {
            if let Some(from) = legacy.hosts.iter().find(|h| h.id == host.id && h.label == host.label) {
                host.password = host.password.take().or_else(|| from.password.clone());
                host.key = host.key.or(from.key);
                host.identity = host.identity.or(from.identity);
            }
        }
        self.secrets_missing = legacy.secrets_missing;
    }

    /// A host's own credentials (a hidden legacy identity) imported before
    /// they were told apart ended up as a Keychain identity linked to the host.
    /// An identity is folded back into its hosts only when the fresh legacy
    /// read `legacy` has no such identity and gives the same host no identity;
    /// a folded identity is removed once nothing links to it.
    pub fn fold_hidden_identities(&mut self, legacy: &Vault) {
        self.legacy_hidden_identities_folded = true;
        let stale: Vec<u64> = self
            .identities
            .iter()
            .filter(|i| !legacy.identities.iter().any(|l| l.id == i.id))
            .map(|i| i.id)
            .collect();
        let mut folded = Vec::new();
        for host in &mut self.hosts {
            let Some(id) = host.identity.filter(|id| stale.contains(id)) else { continue };
            let Some(from) = legacy.hosts.iter().find(|h| h.id == host.id && h.label == host.label && h.identity.is_none()) else { continue };
            let identity = self.identities.iter().find(|i| i.id == id);
            host.username = host.username.take().or_else(|| from.username.clone()).or_else(|| identity.map(|i| i.username.clone()).filter(|u| !u.is_empty()));
            host.password = host.password.take().or_else(|| from.password.clone()).or_else(|| identity.and_then(|i| i.password.clone()));
            host.key = host.key.or(from.key).or_else(|| identity.and_then(|i| i.key));
            host.identity = None;
            folded.push(id);
        }
        let hosts = &self.hosts;
        self.identities.retain(|i| !folded.contains(&i.id) || hosts.iter().any(|h| h.identity == Some(i.id)));
    }

    /// Removes hosts' own legacy credentials that the old import showed as
    /// Keychain identities and that no host uses any more; returns their labels.
    /// Identities made here are never hidden in legacy, so they stay.
    pub fn drop_unused_hidden_identities(&mut self, legacy: &Vault) -> Vec<String> {
        self.legacy_unused_hidden_identities_dropped = true;
        let hosts = &self.hosts;
        let unused = |i: &Identity| legacy.hidden_identity_ids.contains(&i.id) && !hosts.iter().any(|h| h.identity == Some(i.id));
        let dropped = self.identities.iter().filter(|i| unused(i)).map(|i| i.label.clone()).collect();
        self.identities.retain(|i| !unused(i));
        dropped
    }

    /// Legacy Saved marks and comments for connections imported before they
    /// were read: matched by start time and name; a comment written here is kept.
    pub fn import_legacy_bookmarks(&mut self, legacy: &[HistoryEntry]) {
        self.legacy_bookmarks_imported = true;
        for from in legacy.iter().filter(|e| e.saved) {
            if let Some(entry) = self.history.iter_mut().find(|e| e.connected_at == from.connected_at && e.label == from.label) {
                entry.saved = true;
                if entry.comment.is_empty() {
                    entry.comment = from.comment.clone();
                }
            }
        }
    }

    /// Adds connections read from the legacy profile, once: the same start
    /// time and name are not added twice, the newest 100 stay (Saved ones
    /// always), newest first.
    pub fn import_history(&mut self, entries: Vec<HistoryEntry>) -> usize {
        let mut next = self.history.iter().map(|e| e.id).max().unwrap_or(0).max(self.next_id());
        let mut added = 0;
        for mut entry in entries {
            if self.history.iter().any(|e| e.connected_at == entry.connected_at && e.label == entry.label) {
                continue;
            }
            next += 1;
            entry.id = next;
            self.history.push(entry);
            added += 1;
        }
        self.history.sort_by_key(|e| std::cmp::Reverse(e.connected_at));
        while self.history.len() > HISTORY_LIMIT {
            match self.history.iter().rposition(|e| !e.saved) {
                Some(oldest) => {
                    self.history.remove(oldest);
                }
                None => break,
            }
        }
        self.legacy_history_imported = true;
        added
    }

    /// Adds legacy Shell History once, skipping commands already here, in
    /// the legacy order: newest first, then by rating.
    pub fn import_shell_history(&mut self, commands: Vec<ShellCommand>) {
        for command in commands {
            if !self.shell_history.iter().any(|c| c.script == command.script && c.completed_at == command.completed_at) {
                self.shell_history.push(command);
            }
        }
        self.shell_history.sort_by(|a, b| b.completed_at.cmp(&a.completed_at).then(b.rating.cmp(&a.rating)));
        self.legacy_shell_history_imported = true;
    }

    /// Existing history from a shell, oldest first. Execution times are not
    /// known; keep live records ahead of it and never count a reconnect as a
    /// new execution. The legacy-profile migration has its own separate flag.
    pub fn merge_shell_history(&mut self, commands: &[String], host: Option<HostId>) -> bool {
        use std::collections::{HashMap, HashSet};
        let mut counts = HashMap::<&str, i64>::new();
        for command in commands {
            let script = command.trim();
            if !script.is_empty() { *counts.entry(script).or_default() += 1; }
        }
        let mut seen: HashSet<String> = self.shell_history.iter().filter(|c| c.host == host).map(|c| c.script.clone()).collect();
        let before = self.shell_history.len();
        for command in commands.iter().rev() {
            if self.shell_history.len() >= SHELL_HISTORY_LIMIT { break; }
            let script = command.trim();
            if script.is_empty() || !seen.insert(script.to_owned()) { continue; }
            self.shell_history.push(ShellCommand { script: script.to_owned(), completed_at: 0, rating: counts[script], host });
        }
        self.shell_history.len() != before
    }

    /// A command the shell integration saw finish: on top, a repeat moving
    /// up with a higher rating instead of being listed twice.
    pub fn record_command(&mut self, script: &str, host: Option<HostId>, completed_at: i64) {
        let script = script.trim();
        if script.is_empty() {
            return;
        }
        let rating = match self.shell_history.iter().position(|c| c.script == script && c.host == host) {
            Some(index) => self.shell_history.remove(index).rating + 1,
            None => 1,
        };
        self.shell_history.insert(0, ShellCommand { script: script.to_owned(), completed_at, rating, host });
        self.shell_history.truncate(SHELL_HISTORY_LIMIT);
    }

    pub fn history_entry_mut(&mut self, id: u64) -> Option<&mut HistoryEntry> {
        self.history.iter_mut().find(|e| e.id == id)
    }

    /// Saving a comment also bookmarks the log; an empty comment is allowed.
    /// Returns whether the encrypted vault needs saving.
    pub fn save_log_comment(&mut self, id: u64, comment: &str) -> bool {
        let Some(entry) = self.history_entry_mut(id).filter(|e| e.log.is_some()) else { return false };
        let comment = bounded_log_comment(comment).trim().to_owned();
        if entry.saved && entry.comment == comment { return false; }
        entry.saved = true;
        entry.comment = comment;
        true
    }

    /// Removing the Saved mark discards its comment too, as in legacy.
    pub fn remove_log_bookmark(&mut self, id: u64) -> bool {
        let Some(entry) = self.history_entry_mut(id) else { return false };
        let changed = entry.saved || !entry.comment.is_empty();
        entry.saved = false;
        entry.comment.clear();
        changed
    }

    /// Sessions with a log, for the Logs screen: newest first, or Saved first.
    pub fn logs(&self, saved_first: bool) -> Vec<&HistoryEntry> {
        let mut logs: Vec<&HistoryEntry> = self.history.iter().filter(|e| e.log.is_some()).collect();
        logs.sort_by(|a, b| (saved_first && b.saved).cmp(&(saved_first && a.saved)).then(b.connected_at.cmp(&a.connected_at)));
        logs
    }

    /// "Recent connections": newest first, one row per host that still exists.
    pub fn recent_hosts(&self) -> Vec<&Host> {
        let mut seen = Vec::new();
        self.history
            .iter()
            .filter_map(|entry| entry.host.and_then(|h| self.host(h)))
            .filter(|host| {
                let new = !seen.contains(&host.id);
                seen.push(host.id);
                new
            })
            .collect()
    }

    /// `Personal / work / Cenix`, the right-hand text of a recent connection.
    pub fn host_location(&self, host: &Host) -> String {
        std::iter::once("Personal".to_owned()).chain(self.breadcrumbs(host.group).iter().map(|g| g.label.clone())).collect::<Vec<_>>().join(" / ")
    }

    /// A free id for a new record of any kind.
    pub fn next_id(&self) -> u64 {
        let ids = self.groups.iter().map(|g| g.id).chain(self.hosts.iter().map(|h| h.id)).chain(self.rules.iter().map(|r| r.id));
        let ids = ids.chain(self.history.iter().filter_map(|entry| entry.host)).chain(self.shell_history.iter().filter_map(|entry| entry.host));
        ids.chain(self.keys.iter().map(|k| k.id)).chain(self.identities.iter().map(|i| i.id)).chain(self.snippets.iter().map(|s| s.id)).chain(self.packages.iter().map(|p| p.id)).max().unwrap_or(0).max(self.retired_host_id) + 1
    }

    /// When each saved host was last connected (legacy `hostInteractions`).
    pub fn last_connections(&self) -> std::collections::HashMap<HostId, i64> {
        let mut last = std::collections::HashMap::new();
        for entry in &self.history {
            if let Some(host) = entry.host {
                let at = last.entry(host).or_insert(entry.connected_at);
                *at = (*at).max(entry.connected_at);
            }
        }
        last
    }

    /// The host connected most recently, outlined in the legacy Hosts grid.
    pub fn last_connected_host(&self) -> Option<HostId> {
        self.last_connections().into_iter().max_by_key(|(id, at)| (*at, *id)).map(|(id, _)| id)
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

    /// Adds the group, or replaces the one with the same id.
    pub fn save_group(&mut self, group: Group) {
        match self.groups.iter_mut().find(|g| g.id == group.id) {
            Some(existing) => *existing = group,
            None => self.groups.push(group),
        }
    }

    /// The group and every group inside it.
    pub fn subtree(&self, id: GroupId) -> Vec<GroupId> {
        let mut out = vec![id];
        let mut index = 0;
        while index < out.len() {
            let parent = out[index];
            let children: Vec<GroupId> = self.groups.iter().filter(|g| g.parent == Some(parent) && !out.contains(&g.id)).map(|g| g.id).collect();
            out.extend(children);
            index += 1;
        }
        out
    }

    /// Groups a group may move into: not itself and not one of its own subgroups.
    pub fn parent_choices(&self, id: GroupId) -> Vec<&Group> {
        let excluded = self.subtree(id);
        self.groups.iter().filter(|g| !excluded.contains(&g.id)).collect()
    }

    /// `work / Cenix`.
    pub fn group_path(&self, id: GroupId) -> String {
        self.breadcrumbs(Some(id)).iter().map(|g| g.label.as_str()).collect::<Vec<_>>().join(" / ")
    }

    /// Removes a group with its subgroups and their hosts, as the legacy
    /// "Remove a group … And these hosts" does. Returns the removed hosts.
    pub fn remove_group(&mut self, id: GroupId) -> Vec<Host> {
        let groups = self.subtree(id);
        let (removed, kept): (Vec<Host>, Vec<Host>) =
            std::mem::take(&mut self.hosts).into_iter().partition(|h| h.group.is_some_and(|g| groups.contains(&g)));
        self.hosts = kept;
        self.unlink_hosts(&removed.iter().map(|h| h.id).collect());
        for id in &groups {
            self.portable.remove(&format!("groups|{id}"));
            self.portable.remove(&format!("sshConfigs-of-group|{id}"));
        }
        self.groups.retain(|g| !groups.contains(&g.id));
        removed
    }

    pub fn rule(&self, id: u64) -> Option<&ForwardingRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    pub fn save_rule(&mut self, rule: ForwardingRule) {
        match self.rules.iter_mut().find(|r| r.id == rule.id) {
            Some(existing) => *existing = rule,
            None => self.rules.push(rule),
        }
    }

    pub fn remove_rule(&mut self, id: u64) {
        self.rules.retain(|r| r.id != id);
    }

    pub fn identity(&self, id: u64) -> Option<&Identity> {
        self.identities.iter().find(|i| i.id == id)
    }

    /// Adds or replaces an identity, keeping its "Auth …" subtitle in step.
    pub fn save_identity(&mut self, mut identity: Identity) {
        identity.has_password = identity.password.is_some();
        identity.has_key = identity.key.is_some();
        match self.identities.iter_mut().find(|i| i.id == identity.id) {
            Some(existing) => *existing = identity,
            None => self.identities.push(identity),
        }
    }

    /// Removes an identity and unlinks it from hosts.
    pub fn remove_identity(&mut self, id: u64) {
        self.identities.retain(|i| i.id != id);
        self.portable.remove(&format!("identities|{id}"));
        for host in &mut self.hosts {
            if host.identity == Some(id) {
                host.identity = None;
            }
        }
    }

    /// Username, password and key a host logs in with: its identity's when it has one.
    pub fn credentials(&self, host: &Host) -> (Option<String>, Option<String>, Option<&SshKey>) {
        match host.identity.and_then(|id| self.identity(id)) {
            Some(identity) => (
                Some(identity.username.clone()).filter(|u| !u.is_empty()),
                identity.password.clone(),
                identity.key.and_then(|k| self.key(k)),
            ),
            None => (host.username.clone(), host.password.clone(), host.key.and_then(|k| self.key(k))),
        }
    }

    pub fn key(&self, id: u64) -> Option<&SshKey> {
        self.keys.iter().find(|k| k.id == id)
    }

    /// Adds the key, or replaces the one with the same id.
    pub fn save_key(&mut self, key: SshKey) {
        match self.keys.iter_mut().find(|k| k.id == key.id) {
            Some(existing) => *existing = key,
            None => self.keys.push(key),
        }
    }

    /// Removes a key and unlinks it from hosts.
    pub fn remove_key(&mut self, id: u64) {
        self.keys.retain(|k| k.id != id);
        self.portable.remove(&format!("sshKeys|{id}"));
        for identity in &mut self.identities { if identity.key == Some(id) { identity.key = None; } }
        for host in &mut self.hosts {
            if host.key == Some(id) {
                host.key = None;
            }
        }
    }

    pub fn remove_host(&mut self, id: HostId) -> Option<Host> {
        let index = self.hosts.iter().position(|h| h.id == id)?;
        self.unlink_hosts(&[id].into_iter().collect());
        Some(self.hosts.remove(index))
    }

    fn unlink_hosts(&mut self, ids: &std::collections::HashSet<u64>) {
        let Some(highest) = ids.iter().max() else { return };
        self.retired_host_id = self.retired_host_id.max(*highest);
        for rule in &mut self.rules {
            if rule.host.is_some_and(|id| ids.contains(&id)) { rule.host = None; }
        }
        for snippet in &mut self.snippets { snippet.targets.retain(|id| !ids.contains(id)); }
        for id in ids {
            for kind in ["hosts", "identities-of-host", "sshConfigs-of-host"] {
                self.portable.remove(&format!("{kind}|{id}"));
            }
        }
        let keep = |host: &serde_json::Value| !host["local_id"].as_u64().is_some_and(|id| ids.contains(&id));
        if let Some(templates) = self.foreign.get_mut("workspaceTemplates").and_then(|v| v.as_array_mut()) {
            for template in templates {
                if let Some(panes) = template.get_mut("workspaceTerminals").and_then(|v| v.as_array_mut()) {
                    panes.retain(|pane| keep(&pane["host"]));
                }
            }
        }
        let clear_chain = |config: &mut serde_json::Value| {
            if let Some(hosts) = config.pointer_mut("/host_chain/hosts_chain").and_then(|v| v.as_array_mut()) {
                hosts.retain(keep);
            }
        };
        for (name, value) in &mut self.portable {
            match name.split('|').next() {
                Some("sshConfigs-of-host" | "sshConfigs-of-group") => clear_chain(value),
                Some("hosts" | "groups") => {
                    for field in ["ssh_config", "telnet_config"] {
                        if let Some(config) = value.get_mut(field) { clear_chain(config); }
                    }
                }
                _ => {}
            }
        }
        for config in self.foreign.get_mut("telnetConfigs").and_then(|v| v.as_array_mut()).into_iter().flatten() {
            clear_chain(config);
        }
    }

    pub fn snippet(&self, id: u64) -> Option<&Snippet> {
        self.snippets.iter().find(|s| s.id == id)
    }

    /// Adds the snippet, or replaces the one with the same id.
    pub fn save_snippet(&mut self, snippet: Snippet) {
        match self.snippets.iter_mut().find(|s| s.id == snippet.id) {
            Some(existing) => *existing = snippet,
            None => self.snippets.push(snippet),
        }
    }

    pub fn remove_snippet(&mut self, id: u64) {
        self.snippets.retain(|s| s.id != id);
    }

    /// Duplicate in the legacy snippet menu: a copy under a new id, right after the original.
    pub fn duplicate_snippet(&mut self, id: u64) -> Option<u64> {
        let index = self.snippets.iter().position(|s| s.id == id)?;
        let copy = Snippet { id: self.next_id(), ..self.snippets[index].clone() };
        let new_id = copy.id;
        self.snippets.insert(index + 1, copy);
        Some(new_id)
    }

    pub fn package(&self, id: u64) -> Option<&SnippetPackage> {
        self.packages.iter().find(|p| p.id == id)
    }

    pub fn save_package(&mut self, package: SnippetPackage) {
        match self.packages.iter_mut().find(|p| p.id == package.id) {
            Some(existing) => *existing = package,
            None => self.packages.push(package),
        }
    }

    /// The package with this name, created when there is none: the legacy
    /// Package field «Create a new package by typing the name».
    pub fn package_named(&mut self, label: &str) -> u64 {
        if let Some(existing) = self.packages.iter().find(|p| p.label == label) {
            return existing.id;
        }
        let id = self.next_id();
        self.packages.push(SnippetPackage { id, label: label.to_owned() });
        id
    }

    pub fn package_snippet_count(&self, id: u64) -> usize {
        self.snippets.iter().filter(|s| s.package == Some(id)).count()
    }

    /// Removes a package with its snippets, as legacy «Remove … package and N snippets» does.
    pub fn remove_package(&mut self, id: u64) {
        self.packages.retain(|p| p.id != id);
        self.snippets.retain(|s| s.package != Some(id));
    }

    /// Snippets on the screen: a package's own, or — at the top — those
    /// outside packages (all of them while searching), as legacy filters them.
    pub fn snippets_in(&self, package: Option<u64>, searching: bool) -> Vec<&Snippet> {
        self.snippets.iter().filter(|s| package.map_or(searching || s.package.is_none(), |p| s.package == Some(p))).collect()
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

    #[test]
    fn legacy_bookmarks_mark_connections_imported_earlier() {
        let entry = |id, at, label: &str| HistoryEntry { id, connected_at: at, label: label.into(), ..HistoryEntry::default() };
        let mut v = Vault { history: vec![entry(1, 100, "a"), HistoryEntry { comment: "mine".into(), ..entry(2, 200, "b") }, entry(3, 300, "c")], ..Vault::default() };
        let legacy = [
            HistoryEntry { saved: true, comment: "theirs".into(), ..entry(0, 100, "a") },
            HistoryEntry { saved: true, comment: "theirs".into(), ..entry(0, 200, "b") },
            HistoryEntry { saved: true, ..entry(0, 300, "renamed") },
        ];
        v.import_legacy_bookmarks(&legacy);
        assert!(v.legacy_bookmarks_imported);
        let got: Vec<_> = v.history.iter().map(|e| (e.saved, e.comment.as_str())).collect();
        assert_eq!(got, [(true, "theirs"), (true, "mine"), (false, "")]);
    }

    #[test]
    fn hidden_identities_imported_as_keychain_entries_fold_back_into_hosts() {
        let identity = |id, label: &str| Identity { id, label: label.into(), username: label.into(), password: Some(format!("{label}-pw")), ..Identity::default() };
        let mut vault = Vault {
            hosts: vec![
                Host { identity: Some(21), ..host(10, "web", None) },
                Host { identity: Some(20), ..host(11, "api", None) },
                Host { identity: Some(22), ..host(12, "renamed", None) },
            ],
            identities: vec![identity(20, "ops"), identity(21, "pt"), identity(22, "db"), identity(23, "mine")],
            ..Vault::default()
        };
        let legacy = Vault {
            hosts: vec![
                Host { username: Some("pt".into()), password: Some("secret".into()), key: Some(1), ..host(10, "web", None) },
                Host { identity: Some(20), ..host(11, "api", None) },
                host(12, "old name", None),
            ],
            identities: vec![identity(20, "ops")],
            ..Vault::default()
        };
        vault.fold_hidden_identities(&legacy);
        assert!(vault.legacy_hidden_identities_folded);
        let web = &vault.hosts[0];
        assert_eq!((web.identity, web.username.as_deref(), web.password.as_deref(), web.key), (None, Some("pt"), Some("secret"), Some(1)));
        assert_eq!(vault.hosts[1].identity, Some(20), "a real Keychain identity stays linked");
        assert_eq!(vault.hosts[2].identity, Some(22), "a host that no longer matches is left alone");
        // 23 was made here (not in legacy) and nothing links it, yet it is the user's own.
        assert_eq!(vault.identities.iter().map(|i| i.id).collect::<Vec<_>>(), [20, 22, 23]);
    }

    #[test]
    fn unused_hidden_legacy_identities_are_dropped_and_others_stay() {
        let identity = |id, label: &str| Identity { id, label: label.into(), ..Identity::default() };
        let mut vault = Vault {
            hosts: vec![Host { identity: Some(31), ..host(10, "web", None) }],
            identities: vec![identity(30, "orphan"), identity(31, "used"), identity(32, "visible"), identity(33, "mine")],
            ..Vault::default()
        };
        let legacy = Vault { hidden_identity_ids: vec![30, 31], identities: vec![identity(32, "visible")], ..Vault::default() };
        assert_eq!(vault.drop_unused_hidden_identities(&legacy), ["orphan"]);
        assert!(vault.legacy_unused_hidden_identities_dropped);
        assert_eq!(vault.identities.iter().map(|i| i.id).collect::<Vec<_>>(), [31, 32, 33]);
    }

    #[test]
    fn legacy_secrets_fill_a_vault_imported_without_them() {
        let mut vault = Vault {
            hosts: vec![Host { username: Some("root".into()), ..host(10, "prod", None) }, host(11, "renamed", None), Host { password: Some("mine".into()), ..host(12, "own", None) }],
            keys: vec![SshKey { id: 1, label: "k".into(), ..SshKey::default() }],
            identities: vec![Identity { id: 20, label: "root".into(), has_password: true, username: "root".into(), ..Identity::default() }],
            secrets_missing: true,
            ..Vault::default()
        };
        let legacy = Vault {
            hosts: vec![
                Host { identity: Some(20), username: Some("root".into()), ..host(10, "prod", None) },
                Host { password: Some("x".into()), ..host(11, "old name", None) },
                Host { password: Some("theirs".into()), key: Some(1), ..host(12, "own", None) },
            ],
            keys: vec![SshKey { id: 1, label: "k".into(), private_key: Some("PRIVATE".into()), passphrase: Some("p".into()), ..SshKey::default() }],
            identities: vec![Identity { id: 20, label: "root".into(), has_password: true, username: "root".into(), password: Some("pw".into()), ..Identity::default() }],
            ..Vault::default()
        };

        vault.import_legacy_secrets(&legacy);

        assert_eq!(vault.credentials(vault.host(10).unwrap()), (Some("root".into()), Some("pw".into()), None));
        assert_eq!(vault.host(11).unwrap().password, None, "a host changed since the import is left alone");
        let own = vault.host(12).unwrap();
        assert_eq!((own.password.as_deref(), own.key), (Some("mine"), Some(1)), "values set in this client are kept");
        assert_eq!(vault.key(1).unwrap().private_key.as_deref(), Some("PRIVATE"));
        assert!(!vault.secrets_missing);
    }

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
            key: None,
            identity: None,
            settings: HostSettings::default(),
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
    fn forwarding_rules_read_like_the_legacy_cards() {
        let v = vault();
        let mut rule = ForwardingRule::new(50, RuleKind::Local);
        assert!(!rule.is_valid());
        assert_eq!(rule.title(&v), "Local Rule");
        rule.host = Some(11);
        rule.port = Some(8080);
        rule.destination = "10.0.0.5".into();
        rule.destination_port = Some(80);
        assert!(rule.is_valid());
        assert_eq!(rule.rule_text(&v), "From Office VPN:8080 to 10.0.0.5:80");
        rule.label.clear();
        assert_eq!(rule.title(&v), "From Office VPN:8080 to 10.0.0.5:80");
        let dynamic = ForwardingRule { kind: RuleKind::Dynamic, destination: String::new(), destination_port: None, ..rule };
        assert!(dynamic.is_valid());
        assert_eq!(dynamic.rule_text(&v), "From Office VPN:8080");
    }

    #[test]
    fn an_identity_supplies_the_host_credentials() {
        let mut v = vault();
        v.keys.push(SshKey { id: 70, label: "deploy".into(), kind: "ED25519".into(), private_key: Some("k".into()), ..SshKey::default() });
        let id = v.next_id();
        v.save_identity(Identity { id, label: "deploy".into(), username: "deploy".into(), password: Some("pw".into()), key: Some(70), ..Identity::default() });
        assert_eq!(v.identity(id).unwrap().subtitle(), "Auth password and key");
        let mut host = v.host(10).unwrap().clone();
        host.username = Some("root".into());
        assert_eq!(v.credentials(&host).0.as_deref(), Some("root"));
        host.identity = Some(id);
        let (user, password, key) = v.credentials(&host);
        assert_eq!((user.as_deref(), password.as_deref(), key.map(|k| k.id)), (Some("deploy"), Some("pw"), Some(70)));
        v.save_host(host);
        v.remove_identity(id);
        assert_eq!(v.host(10).unwrap().identity, None);
    }

    #[test]
    fn logs_list_sessions_with_a_log_and_saved_ones_survive_the_limit() {
        let mut v = vault();
        let log = |n: &str| Some(LogRef { name: n.into(), key: "k".into() });
        let old = v.record_connection(HistoryEntry { host: Some(10), connected_at: 1, log: log("a"), ..HistoryEntry::default() });
        assert!(v.save_log_comment(old, "Keep this recording"));
        v.record_connection(HistoryEntry { host: Some(11), connected_at: 2, log: log("b"), ..HistoryEntry::default() });
        v.record_connection(HistoryEntry { host: Some(12), connected_at: 3, ..HistoryEntry::default() });
        let names = |v: &Vault, saved_first| v.logs(saved_first).iter().map(|e| e.log.as_ref().unwrap().name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&v, false), ["b", "a"]);
        assert_eq!(names(&v, true), ["a", "b"]);
        for i in 0..200 {
            v.record_connection(HistoryEntry { host: Some(10), connected_at: 10 + i, ..HistoryEntry::default() });
        }
        assert_eq!(v.history.len(), 100);
        assert!(v.history.iter().any(|e| e.id == old), "the saved entry is kept");
        v.history_entry_mut(old).unwrap().disconnected_at = Some(9);
        assert_eq!(v.history.iter().find(|e| e.id == old).unwrap().disconnected_at, Some(9));
    }

    #[test]
    fn commenting_saves_a_log_and_removing_the_bookmark_clears_its_comment() {
        let mut v = vault();
        let id = v.record_connection(HistoryEntry { log: Some(LogRef { name: "a".into(), key: "k".into() }), ..HistoryEntry::default() });
        assert!(v.save_log_comment(id, "  Выпуск\nпроверен ✅  "));
        let entry = v.logs(true)[0];
        assert!(entry.saved);
        assert_eq!(entry.comment, "Выпуск\nпроверен ✅");
        assert!(!v.save_log_comment(id, "Выпуск\nпроверен ✅"));
        assert!(v.save_log_comment(id, ""));
        assert!(v.logs(false)[0].saved, "clearing text alone keeps the bookmark");
        v.save_log_comment(id, "note");
        assert!(v.remove_log_bookmark(id));
        assert!(!v.logs(false)[0].saved);
        assert_eq!(v.logs(false)[0].comment, "");
        assert!(!v.remove_log_bookmark(id));
        assert!(!v.save_log_comment(999, "missing"));
        let without_log = v.record_connection(HistoryEntry::default());
        assert!(!v.save_log_comment(without_log, "not a log"));
    }

    #[test]
    fn log_comments_are_bounded_and_old_vault_entries_still_load() {
        let old: HistoryEntry = serde_json::from_value(serde_json::json!({
            "id": 1, "host": null, "connected_at": 123, "saved": true,
            "log": {"name": "old", "key": "k"}
        })).unwrap();
        assert_eq!(old.comment, "");
        let mut v = Vault { history: vec![old], ..Vault::default() };
        let text = format!("{}✅x", "я".repeat(1999));
        assert!(v.save_log_comment(1, &text));
        assert_eq!(v.logs(false)[0].comment, format!("{}✅", "я".repeat(1999)));
        v.save_log_comment(1, &format!("{}😀", "a".repeat(1999)));
        assert_eq!(v.logs(false)[0].comment, "a".repeat(1999), "never split a UTF-16 surrogate pair at the legacy 2000-unit limit");
        let round_trip: Vault = serde_json::from_slice(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert_eq!(round_trip, v);
    }

    #[test]
    fn recent_hosts_are_newest_first_once_each() {
        let mut v = vault();
        let at = |host, connected_at| HistoryEntry { host: Some(host), connected_at, ..HistoryEntry::default() };
        v.record_connection(at(11, 1));
        v.record_connection(at(12, 2));
        v.record_connection(at(11, 3));
        v.record_connection(at(999, 4));
        v.record_connection(HistoryEntry { host: None, connected_at: 5, ..HistoryEntry::default() });
        let ids: Vec<HostId> = v.recent_hosts().iter().map(|h| h.id).collect();
        assert_eq!(ids, [11, 12], "removed hosts are skipped, repeats collapse");
        assert_eq!(v.host_location(v.host(12).unwrap()), "Personal / work / Ugoloc");
        for i in 0..150 {
            v.record_connection(at(10, i));
        }
        assert_eq!(v.history.len(), 100);
    }

    #[test]
    fn groups_move_without_cycles_and_remove_with_their_hosts() {
        let mut v = vault();
        assert_eq!(v.subtree(1), [1, 3]);
        let choices: Vec<GroupId> = v.parent_choices(1).iter().map(|g| g.id).collect();
        assert_eq!(choices, [2], "work cannot move into itself or Ugoloc");
        assert_eq!(v.group_path(3), "work / Ugoloc");
        v.save_group(Group { id: 3, label: "Ugoloc".into(), parent: Some(2) });
        assert_eq!(v.group_path(3), "private / Ugoloc");
        let removed: Vec<HostId> = v.remove_group(2).iter().map(|h| h.id).collect();
        assert_eq!(removed, [10, 12]);
        assert!(v.group(2).is_none() && v.group(3).is_none());
        assert_eq!(v.hosts.iter().map(|h| h.id).collect::<Vec<_>>(), [11, 13]);
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
        let identity = |has_password, has_key| Identity { id: 1, label: "root".into(), has_password, has_key, ..Identity::default() };
        assert_eq!(identity(true, true).subtitle(), "Auth password and key");
        assert_eq!(identity(false, true).subtitle(), "Auth key");
        let snippet = |label: &str, script: &str| Snippet { id: 1, label: label.into(), script: script.into(), ..Snippet::default() };
        assert_eq!(snippet("sudo su", "sudo su").subtitle(), None);
        assert_eq!(snippet("get pods", "kubectl get pods").subtitle(), Some("kubectl get pods"));
        assert_eq!(snippet("", "\n  uptime\n  w\n").display_label(), "uptime");
        assert_eq!(snippet("", "uptime").subtitle(), None);
    }

    #[test]
    fn extracted_history_deduplicates_by_host_without_inflating_repeats() {
        let mut v = Vault::default();
        v.record_command("live", Some(7), 30);
        let batch: Vec<String> = [" old ", "live", "new", "old", " "].into_iter().map(str::to_owned).collect();
        assert!(v.merge_shell_history(&batch, Some(7)));
        assert_eq!(v.shell_history.iter().map(|c| (c.script.as_str(), c.rating, c.completed_at)).collect::<Vec<_>>(),
            [("live", 1, 30), ("old", 2, 0), ("new", 1, 0)]);
        let previous = v.shell_history.clone();
        assert!(!v.merge_shell_history(&batch, Some(7)));
        assert_eq!(v.shell_history, previous);
        assert!(!v.legacy_shell_history_imported);
        assert!(v.merge_shell_history(&["old".into()], Some(8)));
        assert_eq!(v.shell_history.last().unwrap().host, Some(8));
        assert!(v.merge_shell_history(&["old".into()], None));
        assert_eq!(v.shell_history.last().unwrap().host, None);
    }

    #[test]
    fn extracted_history_keeps_newest_batch_entries_within_the_limit() {
        let mut v = Vault::default();
        v.record_command("live", None, 30);
        let batch: Vec<_> = (0..2100).map(|i| format!("command-{i}")).collect();
        assert!(v.merge_shell_history(&batch, None));
        assert_eq!(v.shell_history.len(), 2000);
        assert_eq!(v.shell_history[0].script, "live");
        assert_eq!(v.shell_history[1].script, "command-2099");
        assert_eq!(v.shell_history.last().unwrap().script, "command-101");
        assert!(!v.merge_shell_history(&batch, None));
    }

    #[test]
    fn shell_history_imports_once_newest_first() {
        let mut v = Vault::default();
        let cmd = |script: &str, completed_at, rating| ShellCommand { script: script.into(), completed_at, rating, host: None };
        v.import_shell_history(vec![cmd("ls", 10, 1), cmd("top", 20, 1), cmd("df", 20, 3)]);
        v.import_shell_history(vec![cmd("ls", 10, 1)]);
        assert_eq!(v.shell_history.iter().map(|c| c.script.as_str()).collect::<Vec<_>>(), ["df", "top", "ls"]);
        assert!(v.legacy_shell_history_imported);
        v.record_command(" uptime ", Some(7), 30);
        v.record_command("ls", None, 31);
        v.record_command("uptime", Some(7), 32);
        assert_eq!((v.shell_history[0].script.as_str(), v.shell_history[0].rating, v.shell_history[0].host), ("uptime", 2, Some(7)), "a repeat moves up");
        assert_eq!(v.shell_history.iter().filter(|c| c.script == "uptime").count(), 1);
        v.record_command("  ", None, 33);
        assert_eq!(v.shell_history[0].script, "uptime");
    }

    #[test]
    fn snippet_packages_and_targets() {
        let mut v = demo();
        assert_eq!(v.snippets_in(None, false).iter().map(|s| s.id).collect::<Vec<_>>(), [1, 2], "outside packages");
        assert_eq!(v.snippets_in(None, true).len(), 3, "searching lists every snippet");
        assert_eq!(v.snippets_in(Some(30), false).iter().map(|s| s.id).collect::<Vec<_>>(), [3]);
        assert_eq!(v.package_named("Kubernetes"), 30);
        let tools = v.package_named("tools");
        assert!(tools > 30 && v.package(tools).is_some());
        let copy = v.duplicate_snippet(3).unwrap();
        assert_eq!(v.package_snippet_count(30), 2);
        v.remove_host(20);
        assert_eq!(v.snippet(copy).unwrap().targets, [21], "removed hosts leave the targets");
        v.remove_package(30);
        assert!(v.package(30).is_none() && v.snippet(3).is_none() && v.snippet(copy).is_none(), "the package goes with its snippets");
        assert!(v.snippet(1).is_some());
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
        settings: HostSettings::default(),
        id,
        label: label.into(),
        address: format!("192.0.2.{id}"),
        os: Some(os.into()),
        group,
        username: user.map(Into::into),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        port: None,
        password: None,
        key: None,
        identity: None,
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
            SshKey { id: 1, label: "kazgau".into(), kind: "ED25519".into(), ..SshKey::default() },
            SshKey { id: 2, label: "My".into(), kind: "ECDSA".into(), ..SshKey::default() },
        ],
        identities: vec![
            Identity { id: 1, label: "root".into(), has_password: false, has_key: true, username: "root".into(), ..Identity::default() },
            Identity { id: 2, label: "pt".into(), has_password: true, has_key: false, username: "pt".into(), ..Identity::default() },
            Identity { id: 3, label: "kazgau".into(), has_password: true, has_key: true, username: "kazgau".into(), ..Identity::default() },
        ],
        snippets: vec![
            Snippet { id: 1, label: "sudo su".into(), script: "sudo su".into(), ..Snippet::default() },
            Snippet { id: 2, label: "cd /var/docker".into(), script: "cd /var/docker".into(), ..Snippet::default() },
            Snippet { id: 3, label: "get pods".into(), script: "kubectl -n default get pods".into(), package: Some(30), targets: vec![20, 21] },
        ],
        packages: vec![SnippetPackage { id: 30, label: "Kubernetes".into() }],
        shell_history: ["docker compose ps", "kubectl -n default get pods", "tail -f /var/log/syslog"]
            .iter()
            .enumerate()
            .map(|(i, script)| ShellCommand { script: script.to_string(), completed_at: 1_794_000_000 - i as i64 * 60, rating: 1, host: None })
            .collect(),
        known_hosts: ["192.168.1.180", "[95.80.105.230]:50703", "10.0.209.12", "[192.168.88.59]:2001"]
            .iter()
            .enumerate()
            .map(|(i, host)| KnownHost { id: i as u64, host: host.to_string() })
            .collect(),
        history: [10, 12, 15]
            .iter()
            .enumerate()
            .map(|(i, host)| HistoryEntry { id: i as u64 + 1, host: Some(*host), connected_at: 1_794_000_000 - i as i64 * 3600, ..HistoryEntry::default() })
            .collect(),
        ..Vault::default()
    }
}
