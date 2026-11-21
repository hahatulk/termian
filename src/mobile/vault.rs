use futures::channel::mpsc::UnboundedReceiver;
use serde_json::{json, Value};

use super::{text, App, Connection, STANDARD};
use crate::{
    forwarding::{ForwardEvent, Forwarding, Spec},
    keys, known_hosts, session, session_log,
    terminal::Terminal,
    vault::{ForwardingRule, Group, Identity, RuleKind, Snippet, SnippetPackage, SshKey},
};
use base64::Engine as _;

pub(super) struct Forward {
    job: Forwarding,
    events: UnboundedReceiver<ForwardEvent>,
    port: Option<u16>,
}

pub(super) fn optional_id(args: &Value, field: &str) -> Result<Option<u64>, String> {
    let value = text(args, field);
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| format!("Invalid {field}."))
}

fn password(args: &Value, previous: Option<String>) -> Option<String> {
    if args["clearPassword"] == true {
        return None;
    }
    let new = text(args, "password");
    if new.is_empty() {
        previous
    } else {
        Some(new.to_owned())
    }
}

impl App {
    fn lists(&self) -> Value {
        let v = &self.vault;
        json!({
            "workspaces":crate::workspace_template::templates(v).iter().map(|t|json!({"id":t.id.to_string(),"label":t.label,
                "count":t.workspace_terminals.len()})).collect::<Vec<_>>(),
            "hosts":self.hosts(),
            "groups": v.groups.iter().map(|g| json!({"id":g.id.to_string(),"label":g.label,"parent":g.parent.map(|id| id.to_string())})).collect::<Vec<_>>(),
            "keys": v.keys.iter().map(|k| json!({"id":k.id.to_string(),"label":k.label,"kind":k.kind,
                "subtitle":k.subtitle(),"publicKey":k.public_key,"certificate":k.certificate})).collect::<Vec<_>>(),
            "identities": v.identities.iter().map(|i| json!({"id":i.id.to_string(),"label":i.label,"username":i.username,
                "subtitle":i.subtitle(),"key":i.key.map(|id| id.to_string())})).collect::<Vec<_>>(),
            "snippets": v.snippets.iter().map(|s| json!({"id":s.id.to_string(),"label":s.label,"display":s.display_label(),"script":s.script,
                "subtitle":s.subtitle(),"package":s.package.map(|id| id.to_string())})).collect::<Vec<_>>(),
            "packages": v.packages.iter().map(|p| json!({"id":p.id.to_string(),"label":p.label,"count":v.package_snippet_count(p.id)})).collect::<Vec<_>>(),
            "known": known_hosts::read(&self.root.join("known_hosts")).iter().map(|k| json!({"label":k.hostnames,"subtitle":k.key_type,"id":k.line})).collect::<Vec<_>>(),
            "recent": v.recent_hosts().iter().take(12).map(|h| json!({"id":h.id.to_string(),"label":h.label,"address":h.address,"os":h.os,
                "port":h.port.unwrap_or(22),"group":v.host_location(h)})).collect::<Vec<_>>(),
            "logs":v.logs(true).iter().map(|r| json!({"id":r.id.to_string(),"label":r.label,"subtitle":session_log::times_label(r.connected_at,r.disconnected_at.unwrap_or(r.connected_at)),
                "saved":r.saved,"comment":r.comment,"device":r.device,"os":r.log_os,"live":self.sessions.values().any(|c| c.history == Some(r.id))})).collect::<Vec<_>>(),
            "rules":v.rules.iter().map(|r| json!({"id":r.id.to_string(),"label":r.label,"subtitle":r.rule_text(v),
                "kind":match r.kind { RuleKind::Local => "Local", RuleKind::Remote => "Remote", RuleKind::Dynamic => "Dynamic" },
                "host":r.host.map(|h|h.to_string()),"bind":r.bind_address,"port":r.port, "destination":r.destination,
                "destinationPort":r.destination_port,"running":self.forwards.contains_key(&r.id),
                "activePort": self.forwards.get(&r.id).and_then(|f| f.port)
            })).collect::<Vec<_>>(),
            "logsEnabled": !v.logs_disabled
        })
    }

    pub(super) fn forward_poll(&mut self) -> Value {
        let mut events = Vec::new();
        let mut closed = Vec::new();
        for (id, f) in &mut self.forwards {
            for _ in 0..16 {
                let Ok(event) = f.events.try_recv() else {
                    break;
                };
                let mut value = match event {
                    ForwardEvent::Login(event) => match super::login_notice(event) {
                        Some(v) => v,
                        None => continue,
                    },
                    ForwardEvent::Active { port } => {
                        f.port = Some(port);
                        json!({"type":"active","value":format!("Listening on port {port}")})
                    }
                    ForwardEvent::Closed(reason) => {
                        closed.push(*id);
                        json!({"type":"closed","value":reason.unwrap_or_else(|| "Forwarding stopped".into())})
                    }
                };
                value["id"] = json!(id.to_string());
                events.push(value);
            }
        }
        for id in closed {
            self.forwards.remove(&id);
        }
        json!(events)
    }

    pub(super) fn vault_call(&mut self, op: &str, args: &Value) -> Result<Value, String> {
        if op == "vault" {
            return Ok(self.lists());
        }
        if op == "deleteKnown" {
            let path = self.root.join("known_hosts");
            let entries = known_hosts::read(&path)
                .into_iter()
                .filter(|k| k.line == text(args, "id"))
                .collect::<Vec<_>>();
            known_hosts::remove(&path, &entries).map_err(|e| e.to_string())?;
            return Ok(self.lists());
        }
        let id = || optional_id(args, "id").and_then(|id| id.ok_or("No record selected.".into()));
        if op == "snippetRun" {
            let snippet = self.vault.snippet(id()?).ok_or("Snippet not found.")?;
            let connection = self
                .selected
                .and_then(|i| self.sessions.get(&i))
                .filter(|c| c.ready)
                .and_then(|c| c.session.as_ref())
                .ok_or("Select a connected terminal.")?;
            connection.write(crate::terminal::paste_bytes(&snippet.script, false).as_slice());
            connection.write(b"\r");
            return Ok(json!({}));
        }
        if op == "forwardAnswer" {
            self.forwards
                .get(&id()?)
                .ok_or("Rule is not running.")?
                .job
                .answer(super::login_answer(args));
            return Ok(json!({}));
        }
        if op == "forwardToggle" {
            let id = id()?;
            if self.forwards.remove(&id).is_none() {
                let r = self
                    .vault
                    .rule(id)
                    .filter(|r| r.is_valid())
                    .ok_or("Complete the rule before connecting.")?;
                let target = self.target(r.host.ok_or("Select a host.")?)?;
                let mut job = Forwarding::start(
                    target,
                    Spec {
                        kind: r.kind,
                        bind_address: r.bind_address.clone(),
                        port: r.port.unwrap_or(0),
                        destination: r.destination.clone(),
                        destination_port: r.destination_port.unwrap_or(0),
                    },
                );
                let events = job.take_events().ok_or("Could not start forwarding.")?;
                self.forwards.insert(
                    id,
                    Forward {
                        job,
                        events,
                        port: None,
                    },
                );
            }
            return Ok(self.lists());
        }
        if op == "openLog" {
            if self.sessions.len() >= 12 {
                return Err("Close a terminal tab before opening another.".into());
            }
            let id = id()?;
            if self.sessions.values().any(|c| c.history == Some(id)) {
                return Err("Close the session before opening its log.".into());
            }
            let row = self
                .vault
                .history
                .iter()
                .find(|r| r.id == id)
                .ok_or("Log not found.")?;
            let log = row.log.as_ref().ok_or("Log not found.")?;
            let key: [u8; 32] = STANDARD
                .decode(&log.key)
                .ok()
                .and_then(|key| key.try_into().ok())
                .ok_or("Invalid log encryption key.")?;
            let dir = self.root.join("session-logs-v2");
            if row.remote_log && !dir.join(session_log::file_name(&log.name)).exists() {
                if self.log_download.is_some() {
                    return Err("A log download is in progress.".into());
                }
                let cloud = self
                    .vault
                    .cloud
                    .as_ref()
                    .ok_or("Configure the cloud to download this log.")?
                    .validate()?;
                let log = log.clone();
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                std::thread::Builder::new()
                    .spawn(move || {
                        let result =
                            crate::log_backup::restore(&cloud, &dir, &log).and_then(|ok| {
                                if ok {
                                    Ok(id.to_string())
                                } else {
                                    Err("The cloud does not have this log file.".into())
                                }
                            });
                        let _ = tx.send(result);
                    })
                    .map_err(|e| e.to_string())?;
                self.log_download = Some(rx);
                return Ok(json!({"downloading":true}));
            }
            let (data, note) = session_log::read(&dir, &log.name, &key)?;
            let mut replay = session::replay(data);
            let events = replay.take_events().ok_or("Could not replay the log.")?;
            let title = session_log::log_tab_title(&row.label, row.connected_at);
            self.next_tab += 1;
            self.selected = Some(self.next_tab);
            self.secondary = None;
            self.broadcast = false;
            let (x, y) = self.size;
            self.sessions.insert(
                self.next_tab,
                Connection {
                    session: None,
                    events,
                    screen: Terminal::with_settings(x.into(), y.into(), &self.vault.terminal),
                    ready: false,
                    backspace: false,
                    dirty: true,
                    title,
                    source: None,
                    history: None,
                    log: None,
                    pending_log: None,
                    notices: Default::default(),
                    status: note.unwrap_or("Session log").to_owned(),
                },
            );
            return Ok(json!({"tabs":self.tabs(),"selected":self.next_tab.to_string()}));
        }
        if op == "openWorkspace" {
            use crate::workspace_template::{Target, ViewMode};
            let id = id()?;
            let template = crate::workspace_template::templates(&self.vault)
                .into_iter()
                .find(|t| t.id == id)
                .ok_or("Workspace not found.")?;
            let targets = template
                .workspace_terminals
                .iter()
                .filter_map(|pane| pane.target(&self.vault))
                .collect::<Vec<_>>();
            if targets.is_empty() {
                return Err("No available terminals in this workspace.".into());
            }
            if targets.len() + self.sessions.len() > 12 {
                return Err("Close terminal tabs before opening this workspace.".into());
            }
            let mut opened = Vec::new();
            for target in targets {
                match target {
                    Target::Host(id) => self.connect(id)?,
                    Target::Local => self.local()?,
                };
                if let Some(id) = self.selected {
                    opened.push(id);
                }
            }
            if template.view_mode == ViewMode::Split && opened.len() >= 2 {
                self.select(opened[0])?;
                self.secondary = Some(opened[1]);
            }
            self.redraw_panes();
            return Ok(self.tab_state());
        }
        let mut v = self.vault.clone();
        let next = || {
            let next = v.next_id();
            optional_id(args, "id").map(|id| id.unwrap_or(next))
        };
        let label = || text(args, "label").trim().to_owned();
        match op {
            "duplicateHost" => {
                v.duplicate_host(id()?).ok_or("Host not found.")?;
            }
            "saveGroup" => {
                let id = next()?;
                let parent = optional_id(args, "parent")?;
                if parent
                    .is_some_and(|p| p == id || v.subtree(id).contains(&p) || v.group(p).is_none())
                {
                    return Err("Choose a different parent group.".into());
                }
                if label().is_empty() {
                    return Err("Enter a group name.".into());
                }
                v.save_group(Group {
                    id,
                    label: label(),
                    parent,
                });
            }
            "deleteGroup" => {
                v.remove_group(id()?);
            }
            "saveIdentity" => {
                let id = next()?;
                let old = v.identity(id).and_then(|i| i.password.clone());
                v.save_identity(Identity {
                    id,
                    label: label(),
                    username: text(args, "username").trim().to_owned(),
                    key: optional_id(args, "key")?,
                    password: password(args, old),
                    ..Default::default()
                });
            }
            "deleteIdentity" => v.remove_identity(id()?),
            "saveKey" => {
                let id = next()?;
                let mut key = v.key(id).cloned().unwrap_or_else(|| SshKey {
                    id,
                    ..Default::default()
                });
                let raw = text(args, "privateKey").trim();
                let pass = text(args, "password");
                if !raw.is_empty() || args["generate"] == true {
                    let (private, public) = if args["generate"] == true {
                        let pair = keys::generate(keys::KeyAlgorithm::Ed25519, pass)?;
                        (pair.private_key, pair.public_key)
                    } else {
                        let decoded = keys::decode(raw, Some(pass))
                            .map_err(|_| "Check the private key and passphrase.")?;
                        let public = decoded
                            .public_key()
                            .to_openssh()
                            .map_err(|e| e.to_string())?;
                        (raw.to_owned(), public)
                    };
                    key.kind = crate::vault::key_kind(&public);
                    key.public_key = Some(public);
                    key.private_key = Some(private);
                    key.passphrase = (!pass.is_empty()).then(|| pass.to_owned());
                }
                if key.private_key.is_none() {
                    return Err("Import or generate a private key.".into());
                }
                let cert = text(args, "certificate").trim();
                key.certificate = if cert.is_empty() {
                    None
                } else {
                    keys::parse_certificate(cert).ok_or("Invalid OpenSSH certificate.")?;
                    Some(cert.to_owned())
                };
                key.label = label();
                v.save_key(key);
            }
            "deleteKey" => v.remove_key(id()?),
            "saveSnippet" => {
                let id = next()?;
                let targets = v.snippet(id).map(|s| s.targets.clone()).unwrap_or_default();
                v.save_snippet(Snippet {
                    id,
                    label: label(),
                    script: text(args, "script").to_owned(),
                    package: optional_id(args, "package")?,
                    targets,
                });
            }
            "deleteSnippet" => v.remove_snippet(id()?),
            "savePackage" => v.save_package(SnippetPackage {
                id: next()?,
                label: label(),
            }),
            "deletePackage" => v.remove_package(id()?),
            "saveRule" => {
                let id = next()?;
                if self.forwards.contains_key(&id) {
                    return Err("Stop the rule before editing.".into());
                }
                let kind = match text(args, "kind") {
                    "Remote" => RuleKind::Remote,
                    "Dynamic" => RuleKind::Dynamic,
                    _ => RuleKind::Local,
                };
                let mut r = ForwardingRule::new(id, kind);
                r.label = label();
                r.host = optional_id(args, "host")?;
                r.bind_address = text(args, "bind").trim().to_owned();
                r.port = text(args, "port").trim().parse().ok();
                r.destination = text(args, "destination").trim().to_owned();
                r.destination_port = text(args, "destinationPort").trim().parse().ok();
                if !r.is_valid() || r.bind_address.is_empty() {
                    return Err("Check the host, addresses and ports.".into());
                }
                v.save_rule(r);
            }
            "deleteRule" => {
                let id = id()?;
                self.forwards.remove(&id);
                v.remove_rule(id);
            }
            "saveLog" => {
                v.save_log_comment(id()?, text(args, "comment"));
            }
            "unbookmarkLog" => {
                v.remove_log_bookmark(id()?);
            }
            "logsEnabled" => {
                v.logs_disabled = args["enabled"] != true;
                if v.logs_disabled {
                    for c in self.sessions.values_mut() {
                        if let Some(log) = c.log.take() {
                            log.finish();
                        }
                    }
                }
            }
            "clearLogs" => {
                if self.sessions.values().any(|c| c.log.is_some()) {
                    return Err(
                        "Close the sessions that are recording before clearing logs.".into(),
                    );
                }
                let dirs = self.root.join("session-logs-v2");
                let since = args["hours"]
                    .as_u64()
                    .filter(|n| *n > 0 && *n <= 168)
                    .map(|n| {
                        session_log::period_start(
                            std::time::Duration::from_secs(n * 3600),
                            std::time::SystemTime::now(),
                        )
                    })
                    .unwrap_or(0);
                let removed = session_log::clear(&mut v.history, &dirs, since);
                self.save(v)?;
                return Ok(
                    json!({"message":format!("Deleted: {}. Failed: {}.", removed.files, removed.failed)}),
                );
            }
            "saveWorkspace" => {
                use crate::split_tree::{Layout, Position, SplitTree};
                use crate::workspace_template::{OpenTerminal, ViewMode, WorkspaceTemplate};
                let id = v.foreign["workspaceTemplates"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|t| t["local_id"].as_u64())
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)
                    .max(v.next_id());
                let panes = [self.selected, self.secondary]
                    .into_iter()
                    .flatten()
                    .filter_map(|pane| {
                        Some(OpenTerminal {
                            pane,
                            target: self.sessions.get(&pane)?.source?,
                            from: None,
                        })
                    })
                    .collect::<Vec<_>>();
                let first = panes
                    .first()
                    .ok_or("Open a saved host or local terminal first.")?;
                let mut tree = SplitTree::single(first.pane);
                if let Some(second) = panes.get(1) {
                    tree.split(first.pane, second.pane, Layout::Horizontal, Position::End);
                }
                let view = if panes.len() > 1 {
                    ViewMode::Split
                } else {
                    ViewMode::Focus
                };
                let label = label();
                if label.is_empty() {
                    return Err("Enter a workspace name.".into());
                }
                let workspace = WorkspaceTemplate::from_tab(id, &label, view, &panes, &tree, None)
                    .ok_or("Nothing to save.")?;
                crate::workspace_template::save(&mut v, &workspace);
            }
            "deleteWorkspace" => crate::workspace_template::remove(&mut v, id()?),
            "savePrivacy" => {
                if let Some(current) = &v.privacy_password {
                    if !current.verify(text(args, "current")) {
                        return Err("Incorrect lock password.".into());
                    }
                }
                v.privacy_password = Some(
                    crate::privacy_password::PrivacyPassword::new(text(args, "password"))
                        .ok_or("Enter a lock password.")?,
                );
            }
            "unlock" => {
                return if v
                    .privacy_password
                    .as_ref()
                    .is_some_and(|p| p.verify(text(args, "password")))
                {
                    Ok(json!({}))
                } else {
                    Err("Incorrect lock password.".into())
                }
            }
            _ => return Err("Unknown operation.".into()),
        }
        self.save(v)?;
        Ok(self.lists())
    }
}
