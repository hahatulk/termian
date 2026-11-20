use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
};

mod files;
mod updates;
mod vault;
use std::sync::{mpsc, Mutex};

use futures::channel::mpsc::Receiver;
use jni::{
    objects::{JByteArray, JClass, JString},
    sys::jstring,
    JNIEnv,
};
use serde_json::{json, Value};
use zeroize::Zeroizing;

use crate::{
    cloud::CloudConfig,
    login::{Answer, HostKeyDecision, LoginEvent, Stage},
    session::{self, Session, SessionEvent, SshTarget},
    store::Store,
    terminal::{self, Modifiers, Terminal},
    vault::{Host, Vault},
};

static APP: Mutex<Option<App>> = Mutex::new(None);

struct Connection {
    session: Option<Session>,
    events: Receiver<SessionEvent>,
    screen: Terminal,
    ready: bool,
    backspace: bool,
    dirty: bool,
    title: String,
    source: Option<crate::workspace_template::Target>,
    history: Option<u64>,
    log: Option<crate::session_log::SessionLog>,
    pending_log: Option<mpsc::Receiver<()>>,
    notices: VecDeque<Value>,
    status: String,
}

struct App {
    store: Store,
    root: PathBuf,
    vault: Vault,
    sessions: BTreeMap<u64, Connection>,
    selected: Option<u64>,
    secondary: Option<u64>,
    broadcast: bool,
    next_tab: u64,
    files: Option<files::Browser>,
    forwards: BTreeMap<u64, vault::Forward>,
    sync: Option<mpsc::Receiver<Result<(Vault, String), String>>>,
    cloud_check: Option<
        mpsc::Receiver<
            Result<
                (
                    crate::self_cloud::Preview,
                    Option<crate::self_cloud::UploadPreview>,
                ),
                String,
            >,
        >,
    >,
    ticket: Option<(bool, crate::self_cloud::UploadPreview)>,
    log_download: Option<mpsc::Receiver<Result<String, String>>>,
    size: (u16, u16),
}

fn completed<T>(job: &mut Option<mpsc::Receiver<Result<T, String>>>) -> Option<Result<T, String>> {
    let result = match job.as_ref()?.try_recv() {
        Ok(result) => result,
        Err(mpsc::TryRecvError::Disconnected) => Err("Operation interrupted.".into()),
        Err(mpsc::TryRecvError::Empty) => return None,
    };
    *job = None;
    Some(result)
}

fn text<'a>(v: &'a Value, field: &str) -> &'a str {
    v[field].as_str().unwrap_or_default()
}

impl App {
    fn new(root: PathBuf, key: [u8; 32]) -> Result<Self, String> {
        let store = Store::new(root.join("vault.bin"), key);
        let vault = store.load()?.unwrap_or_default();
        let logs = root.join("session-logs-v2");
        crate::session_log::prune(
            &logs,
            std::time::SystemTime::now(),
            crate::session_log::RETENTION,
        );
        Ok(Self {
            store,
            root,
            vault,
            sessions: BTreeMap::new(),
            selected: None,
            secondary: None,
            broadcast: false,
            next_tab: 0,
            files: None,
            forwards: BTreeMap::new(),
            sync: None,
            cloud_check: None,
            ticket: None,
            log_download: None,
            size: (40, 24),
        })
    }

    fn hosts(&self) -> Value {
        let mut hosts: Vec<_> = self.vault.hosts.iter().collect();
        hosts.sort_by_cached_key(|h| h.label.to_lowercase());
        json!(hosts.iter().map(|h| json!({
            "id": h.id.to_string(), "label": h.label, "address": h.address,
            "port": h.port.unwrap_or(22), "user": h.username,
            "group": self.vault.host_location(h), "groupId": h.group.map(|id| id.to_string()),
            "os":h.os, "tags": h.tags, "keyId":h.key.map(|id| id.to_string()), "identityId":h.identity.map(|id| id.to_string()),
            "identity": h.identity.and_then(|id| self.vault.identity(id)).map(|i| i.label.as_str()),
        })).collect::<Vec<_>>())
    }

    fn save(&mut self, vault: Vault) -> Result<(), String> {
        self.ticket = None;
        self.store.save(&vault)?;
        self.vault = vault;
        Ok(())
    }

    fn edit_host(&mut self, args: &Value) -> Result<Value, String> {
        let address = text(args, "address").trim();
        if address.is_empty()
            || address.len() > 512
            || address
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '/' | '@' | '\\'))
        {
            return Err("Введите адрес без имени пользователя и порта.".into());
        }
        let address = address
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(address);
        if address.contains(':') && address.parse::<std::net::Ipv6Addr>().is_err() {
            return Err("Укажите порт в отдельном поле.".into());
        }
        let port = text(args, "port")
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .ok_or("Порт должен быть от 1 до 65535.")?;
        let mut vault = self.vault.clone();
        let id = text(args, "id");
        let mut h = if id.is_empty() {
            Host {
                id: vault.next_id(),
                label: String::new(),
                address: String::new(),
                os: None,
                group: None,
                username: None,
                tags: vec![],
                port: None,
                password: None,
                key: None,
                identity: None,
                settings: Default::default(),
            }
        } else {
            vault
                .host(id.parse().map_err(|_| "Некорректный хост.")?)
                .cloned()
                .ok_or("Хост не найден.")?
        };
        let label = text(args, "label").trim();
        h.label = if label.is_empty() {
            address.into()
        } else {
            label.into()
        };
        h.address = address.into();
        h.port = (port != 22).then_some(port);
        if args.get("identity").is_some() {
            h.identity = vault::optional_id(args, "identity")?;
        }
        if let Some(identity) = h.identity {
            if vault.identity(identity).is_none() {
                return Err("Identity not found.".into());
            }
        }
        if args.get("group").is_some() {
            h.group = vault::optional_id(args, "group")?;
        }
        if args.get("key").is_some() {
            h.key = vault::optional_id(args, "key")?;
        }
        if let Some(tags) = args["tags"].as_str() {
            h.tags = tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect();
        }
        if h.identity.is_none() {
            let user = text(args, "user").trim();
            h.username = (!user.is_empty()).then(|| user.to_owned());
            if args["clearPassword"] == true {
                h.password = None;
            }
            let password = text(args, "password");
            if !password.is_empty() {
                h.password = Some(password.to_owned());
            }
        }
        let id = h.id.to_string();
        vault.save_host(h);
        self.save(vault)?;
        Ok(json!({"hosts": self.hosts(), "id": id}))
    }

    fn target(&self, id: u64) -> Result<SshTarget, String> {
        let host = self.vault.host(id).ok_or("Host not found.")?;
        let mut target = SshTarget::for_host(host, &self.vault);
        target.known_hosts = self.root.join("known_hosts");
        target.identity_files.clear();
        target.options.agent_forwarding = false;
        target.options.term = self.vault.terminal.emulation().to_owned();
        Ok(target)
    }

    fn connect(&mut self, id: u64) -> Result<Value, String> {
        let target = self.target(id)?;
        let host = self.vault.host(id).ok_or("Host not found.")?;
        let (cols, rows) = self.size;
        self.new_terminal(
            session::ssh(target, cols, rows),
            host.label.clone(),
            Some(id),
            host.settings.ctrl_h_backspace,
            false,
        )
    }

    fn tabs(&self) -> Value {
        json!(self
            .sessions
            .iter()
            .map(|(id, c)| json!({
                "id": id.to_string(), "label": c.title, "ready": c.ready, "status": c.status,
            }))
            .collect::<Vec<_>>())
    }

    fn new_terminal(
        &mut self,
        mut session: Session,
        title: String,
        host: Option<u64>,
        backspace: bool,
        local: bool,
    ) -> Result<Value, String> {
        if self.sessions.len() >= 12 {
            return Err("Close a terminal tab before opening another.".into());
        }
        let (cols, rows) = self.size;
        let key = crate::session_log::new_key();
        let name = crate::session_log::new_name();
        let log = if self.vault.logs_disabled {
            None
        } else {
            crate::session_log::SessionLog::start(&self.root.join("session-logs-v2"), &name, key)
                .ok()
        };
        let history = self.vault.record_connection(crate::vault::HistoryEntry {
            label: title.clone(),
            host,
            connected_at: chrono::Utc::now().timestamp(),
            connected: local,
            device: "Android".into(),
            subtitle: if local { "local" } else { "ssh" }.into(),
            log: log.as_ref().map(|_| crate::vault::LogRef {
                name,
                key: STANDARD.encode(key),
            }),
            ..Default::default()
        });
        self.store.save(&self.vault)?;
        let events = session.take_events().ok_or("Could not open terminal.")?;
        self.next_tab += 1;
        self.selected = Some(self.next_tab);
        self.secondary = None;
        self.broadcast = false;
        let status = (if local { "Connected" } else { "Connecting…" }).to_owned();
        self.sessions.insert(
            self.next_tab,
            Connection {
                session: Some(session),
                events,
                screen: Terminal::with_settings(cols.into(), rows.into(), &self.vault.terminal),
                ready: local,
                backspace,
                dirty: true,
                title,
                source: if local {
                    Some(crate::workspace_template::Target::Local)
                } else {
                    host.map(crate::workspace_template::Target::Host)
                },
                history: Some(history),
                log,
                pending_log: None,
                notices: VecDeque::new(),
                status: status.clone(),
            },
        );
        Ok(json!({"tabs": self.tabs(), "selected":self.next_tab.to_string(), "status":status}))
    }

    fn end_history(&mut self, id: u64) {
        if let Some(row) = self.vault.history_entry_mut(id) {
            row.disconnected_at = Some(chrono::Utc::now().timestamp());
            let _ = self.store.save(&self.vault);
        }
    }

    fn redraw_panes(&mut self) {
        for id in [self.selected, self.secondary].into_iter().flatten() {
            if let Some(pane) = self.sessions.get_mut(&id) {
                pane.dirty = true;
            }
        }
    }

    fn tab_state(&self) -> Value {
        json!({"tabs":self.tabs(),"selected":self.selected.map(|id|id.to_string()),
            "secondary":self.secondary.map(|id|id.to_string()),"broadcast":self.broadcast})
    }

    fn select(&mut self, id: u64) -> Result<Value, String> {
        let c = self
            .sessions
            .get_mut(&id)
            .ok_or("Terminal tab not found.")?;
        if self.secondary == Some(id) {
            self.secondary = self.selected;
        } else if self.selected != Some(id) {
            self.secondary = None;
            self.broadcast = false;
        }
        self.selected = Some(id);
        c.dirty = true;
        if let Some(other) = self
            .secondary
            .and_then(|other| self.sessions.get_mut(&other))
        {
            other.dirty = true;
        }
        Ok(self.tab_state())
    }

    fn close_tab(&mut self, id: u64) {
        if let Some(mut c) = self.sessions.remove(&id) {
            if let Some(log) = c.log.take() {
                log.finish();
            }
            if let Some(history) = c.history.take() {
                self.end_history(history);
            }
        }
        if self.secondary == Some(id) {
            self.secondary = None;
            self.broadcast = false;
        }
        if self.selected == Some(id) {
            self.selected = self
                .secondary
                .take()
                .or_else(|| self.sessions.keys().next_back().copied());
            self.broadcast = false;
            if let Some(c) = self.selected.and_then(|id| self.sessions.get_mut(&id)) {
                c.dirty = true;
            }
        }
    }

    fn local(&mut self) -> Result<Value, String> {
        let (cols, rows) = self.size;
        let mut command = portable_pty::CommandBuilder::new("/system/bin/sh");
        command.cwd(&self.root);
        let session = session::local_command(command, cols, rows)?;
        self.new_terminal(session, "Local Terminal".into(), None, false, true)
    }

    fn poll(&mut self) -> Value {
        let mut events = Vec::new();
        let mut frame = None;
        let mut secondary_frame = None;
        let mut ended = Vec::new();
        for (id, c) in &mut self.sessions {
            let mut notices = Vec::new();
            let active = self.selected == Some(*id);
            let secondary = self.secondary == Some(*id);
            let next_frame = c.poll(&mut notices, active || secondary);
            if active {
                frame = next_frame;
            } else if secondary {
                secondary_frame = next_frame;
            }
            for notice in notices {
                match text(&notice, "type") {
                    "closed" => {
                        if let Some(h) = c.history.take() {
                            ended.push(h);
                        }
                        c.status = text(&notice, "value").to_owned();
                        if let Some(log) = c.log.take() {
                            log.finish();
                        }
                    }
                    "status" => {
                        c.status = text(&notice, "value").to_owned();
                    }
                    _ => c.notices.push_back(notice),
                }
            }
            if active {
                events.extend(c.notices.drain(..));
            }
        }
        for id in ended {
            self.end_history(id);
        }
        let files = self.files.as_mut().map(files::Browser::poll);
        let forwards = self.forward_poll();
        let sync = self.sync.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Синхронизация прервана.".into())),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        let sync = sync.map(|result| {
            self.sync = None;
            match result.and_then(|(mut vault, message)| {
                for old in &self.vault.history {
                    if let Some(row) = vault
                        .history_entry_mut(old.id)
                        .filter(|row| row.log == old.log)
                    {
                        row.disconnected_at = old.disconnected_at;
                    }
                }
                self.save(vault).map(|()| message)
            }) {
                Ok(message) => json!({"message":message, "hosts":self.hosts()}),
                Err(message) => json!({"error":message}),
            }
        });
        let preview = completed(&mut self.cloud_check).map(|result| match result {
            Ok((preview,ticket)) => {
                let empty = preview == crate::self_cloud::Preview::Empty;
                self.ticket = ticket.map(|ticket| (empty,ticket));
                let summary = preview.groups().into_iter().map(|g| format!("{}: {} ({})", g.direction, g.action, g.total)).collect::<Vec<_>>();
                json!({"message":preview.headline(), "changes":summary, "canUpload":self.ticket.is_some(),
                    "canDownload":self.ticket.is_some() && !empty})
            }
            Err(error) => json!({"error":error}),
        });
        let log = completed(&mut self.log_download).map(|result| match result {
            Ok(id) => self
                .vault_call("openLog", &json!({"id":id}))
                .unwrap_or_else(|error| json!({"error":error})),
            Err(error) => json!({"error":error}),
        });
        json!({"events":events, "frame":frame, "secondaryFrame":secondary_frame, "sync":sync, "preview":preview, "log":log, "tabs":self.tabs(), "files":files, "forwards":forwards})
    }

    fn call(&mut self, args: &Value) -> Result<Value, String> {
        let op = text(args, "op");
        let id = || {
            text(args, "id")
                .parse::<u64>()
                .map_err(|_| "Некорректный хост.")
        };
        if (self.sync.is_some() || self.cloud_check.is_some() || self.log_download.is_some())
            && (op.starts_with("save")
                || op.starts_with("delete")
                || matches!(
                    op,
                    "configureCloud"
                        | "sync"
                        | "cloudPreview"
                        | "forceUpload"
                        | "forceDownload"
                        | "clearLogs"
                        | "unbookmarkLog"
                        | "connect"
                        | "openWorkspace"
                        | "local"
                        | "duplicateHost"
                        | "logsEnabled"
                ))
        {
            return Err("Дождитесь завершения синхронизации.".into());
        }
        match op {
            "hosts" => Ok(json!({"hosts": self.hosts()})),
            "saveHost" => self.edit_host(args),
            "deleteHost" => {
                let mut vault = self.vault.clone();
                vault.remove_host(id()?).ok_or("Хост не найден.")?;
                self.save(vault)?;
                Ok(json!({"hosts":self.hosts()}))
            }
            "connect" => self.connect(id()?),
            "quickConnect" => {
                if self.sync.is_some() {
                    return Err("Wait for synchronization.".into());
                }
                let mut target = session::parse_destination(text(args, "value"))
                    .ok_or("Enter user@host:port.")?;
                target.known_hosts = self.root.join("known_hosts");
                target.identity_files.clear();
                let title = target.host.clone();
                self.new_terminal(
                    session::ssh(target, self.size.0, self.size.1),
                    title,
                    None,
                    false,
                    false,
                )
            }
            "local" => self.local(),
            "selectTab" => self.select(id()?),
            "disconnect" => {
                self.close_tab(id()?);
                self.redraw_panes();
                Ok(self.tab_state())
            }
            "split" => {
                let other = vault::optional_id(args, "id")?;
                if other
                    .is_some_and(|id| !self.sessions.contains_key(&id) || self.selected == Some(id))
                {
                    return Err("Select a different terminal.".into());
                }
                self.secondary = other;
                self.broadcast = false;
                self.redraw_panes();
                Ok(self.tab_state())
            }
            "broadcast" => {
                self.broadcast = self.secondary.is_some() && args["enabled"] == true;
                self.redraw_panes();
                Ok(self.tab_state())
            }
            "filesOpen" => {
                self.files = Some(files::Browser::new(self.target(id()?)?, self.root.clone())?);
                Ok(json!({}))
            }
            "filesClose" => {
                self.files = None;
                Ok(json!({}))
            }
            _ if op.starts_with("files") => self
                .files
                .as_mut()
                .ok_or("SFTP is not connected.")?
                .call(op, args),
            "poll" => Ok(self.poll()),
            "cloud" => Ok(json!({"url":self.vault.cloud.as_ref().map(|c| &c.url),
                "basicUser":self.vault.cloud.as_ref().map(|c| &c.basic_user)})),
            "configureCloud" => {
                let url = text(args, "url").trim();
                let current = self.vault.cloud.as_ref().filter(|c| c.url == url);
                let secret = |field, previous: Option<&str>| {
                    let value = text(args, field);
                    if value.is_empty() {
                        previous.unwrap_or_default().to_owned()
                    } else {
                        value.to_owned()
                    }
                };
                let basic_user = text(args, "basicUser").trim().to_owned();
                let config = CloudConfig {
                    url: url.to_owned(),
                    token: secret("token", current.map(|c| c.token.as_str())),
                    passphrase: if args["emptyPassphrase"] == true {
                        String::new()
                    } else {
                        secret("passphrase", current.map(|c| c.passphrase.as_str()))
                    },
                    basic_password: if basic_user.is_empty() {
                        String::new()
                    } else {
                        secret(
                            "basicPassword",
                            current
                                .filter(|c| c.basic_user == basic_user)
                                .map(|c| c.basic_password.as_str()),
                        )
                    },
                    basic_user,
                    auto_sync: false,
                };
                config.validate()?;
                let mut vault = self.vault.clone();
                vault.cloud = Some(config);
                self.save(vault)?;
                Ok(json!({}))
            }
            "cloudPreview" => {
                self.ticket = None;
                let vault = self.vault.clone();
                let (tx, rx) = mpsc::sync_channel(1);
                std::thread::Builder::new()
                    .spawn(move || {
                        let _ = tx.send(crate::self_cloud::checked_preview(&vault));
                    })
                    .map_err(|e| e.to_string())?;
                self.cloud_check = Some(rx);
                Ok(json!({}))
            }
            "sync" | "forceUpload" | "forceDownload" => {
                let operation = op.to_owned();
                let ticket = if op == "sync" {
                    None
                } else {
                    let (empty, ticket) =
                        self.ticket.take().ok_or("Check the cloud preview again.")?;
                    if empty && op == "forceDownload" {
                        return Err("The cloud snapshot is empty.".into());
                    }
                    Some(ticket)
                };
                let mut vault = self.vault.clone();
                let backups = self.store.cloud_backups();
                let (tx, rx) = mpsc::sync_channel(1);
                let logs = self.root.join("session-logs-v2");
                let active = self.sessions.values().filter_map(|c| c.history).collect();
                std::thread::Builder::new()
                    .spawn(move || {
                        let notice = (operation != "forceDownload")
                            .then(|| crate::self_cloud::prepare_logs(&mut vault, &logs, &active))
                            .flatten();
                        let result = match (operation.as_str(), ticket) {
                            ("forceUpload", Some(ticket)) => {
                                crate::self_cloud::upload_checked(&mut vault, ticket)
                            }
                            ("forceDownload", Some(ticket)) => {
                                crate::self_cloud::download_checked(&mut vault, &backups, ticket)
                            }
                            ("sync", _) => crate::self_cloud::sync(&mut vault, &backups),
                            _ => Err("Check the cloud preview again.".into()),
                        }
                        .map(|message| {
                            if let Some(notice) = notice {
                                format!("{message}. {notice}")
                            } else {
                                message
                            }
                        });
                        let _ = tx.send(result.map(|message| (vault, message)));
                    })
                    .map_err(|e| e.to_string())?;
                self.sync = Some(rx);
                Ok(json!({}))
            }
            "size" => {
                let dimension =
                    |field, min, max| args[field].as_u64().unwrap_or(min).clamp(min, max) as u16;
                let size = (dimension("columns", 2, 256), dimension("rows", 2, 160));
                let target = vault::optional_id(args, "id")?.or(self.selected);
                if target == self.selected {
                    self.size = size;
                }
                if let Some(c) = target.and_then(|id| self.sessions.get_mut(&id)) {
                    if c.screen.size() != (size.0.into(), size.1.into()) {
                        c.screen.resize(size.0.into(), size.1.into());
                        if let Some(s) = &c.session {
                            s.resize(size.0, size.1);
                        }
                        c.dirty = true;
                    }
                }
                Ok(json!({}))
            }
            "text" | "paste" | "key" | "trust" | "answer" | "scroll" => {
                let target = vault::optional_id(args, "id")?.or(self.selected);
                if let Some(c) = target.and_then(|id| self.sessions.get_mut(&id)) {
                    c.input(op, args);
                }
                if self.broadcast && matches!(op, "text" | "paste" | "key") {
                    if let Some(c) = self.secondary.and_then(|id| self.sessions.get_mut(&id)) {
                        c.input(op, args);
                    }
                }
                Ok(json!({}))
            }
            _ => self.vault_call(op, args),
        }
    }
}

impl Connection {
    fn input(&mut self, op: &str, args: &Value) {
        let Some(s) = &self.session else {
            if op == "scroll" {
                self.scroll(args);
            }
            return;
        };
        match op {
            "trust" | "answer" => s.answer(login_answer(args)),
            "scroll" => self.scroll(args),
            "text" if self.ready => s.write(text(args, "value").as_bytes()),
            "paste" if self.ready => s.write(&terminal::paste_bytes(
                text(args, "value"),
                self.screen.bracketed_paste(),
            )),
            "key" if self.ready => {
                let key = text(args, "value");
                let mods = Modifiers {
                    control: args["ctrl"] == true,
                    alt: args["alt"] == true,
                    shift: args["shift"] == true,
                };
                if key == "backspace" && self.backspace {
                    s.write(b"\x08");
                } else if let Some(bytes) =
                    terminal::key_bytes(key, args["typed"].as_str(), mods, self.screen.app_cursor())
                {
                    s.write(&bytes);
                }
            }
            _ => {}
        }
    }

    fn scroll(&mut self, args: &Value) {
        self.screen
            .scroll(args["lines"].as_i64().unwrap_or(0).clamp(-160, 160) as i32);
        self.dirty = true;
    }

    fn poll(&mut self, notices: &mut Vec<Value>, visible: bool) -> Option<Value> {
        if self
            .pending_log
            .as_ref()
            .is_some_and(|rx| matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)))
        {
            return None;
        }
        self.pending_log = None;
        for _ in 0..4 {
            let event = match self.events.try_recv() {
                Ok(event) => event,
                Err(e) if e.is_closed() && self.session.is_some() => SessionEvent::Closed(None),
                Err(_) => break,
            };
            match event {
                SessionEvent::Output(bytes) => {
                    self.screen.feed(&bytes);
                    let replies = self.screen.take_replies();
                    if let Some(s) = self.session.as_ref().filter(|_| !replies.is_empty()) {
                        s.write(&replies);
                    }
                    self.dirty = true;
                    if let Some(write) = self.log.as_ref().and_then(|log| log.queue(bytes)) {
                        let (tx, rx) = mpsc::channel();
                        std::thread::spawn(move || {
                            write();
                            let _ = tx.send(());
                        });
                        self.pending_log = Some(rx);
                        break;
                    }
                }
                SessionEvent::Login(LoginEvent::Stage(stage)) => {
                    self.ready = stage == Stage::Ready;
                    notices.push(json!({"type":"status","ready":self.ready,"value":match stage {
                        Stage::Connecting => "Подключение…", Stage::Verifying => "Проверка ключа…",
                        Stage::Authenticating => "Вход…", Stage::Ready => "Подключено",
                    }}));
                }
                SessionEvent::Login(event) => {
                    if let Some(v) = login_notice(event) {
                        notices.push(v);
                    }
                }
                SessionEvent::Closed(reason) => {
                    self.ready = false;
                    self.session = None;
                    notices.push(json!({"type":"closed","value":reason.unwrap_or_else(|| "Сессия завершена".to_owned())}));
                    break;
                }
                _ => {}
            }
        }
        if !visible || !std::mem::take(&mut self.dirty) {
            return None;
        }
        let frame = self.screen.snapshot();
        let lines: Vec<_> = frame
            .lines
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|r| json!([r.text, r.columns, r.fg, r.bg, r.bold]))
                    .collect::<Vec<_>>()
            })
            .collect();
        Some(
            json!({"lines":lines, "cursor":frame.cursor, "bg":self.screen.look().background,
            "cursorColor":self.screen.look().cursor}),
        )
    }
}

fn login_answer(args: &Value) -> Answer {
    match text(args, "mode") {
        "trust" => Answer::HostKey(match text(args, "value") {
            "save" => HostKeyDecision::Save,
            "once" => HostKeyDecision::Once,
            _ => HostKeyDecision::Reject,
        }),
        _ => Answer::Text(args["value"].as_str().map(str::to_owned)),
    }
}

fn login_notice(event: LoginEvent) -> Option<Value> {
    let (kind, info) = match event {
        LoginEvent::HostKey(info) => ("trust", info),
        LoginEvent::HostKeyChanged(info) => ("changed", info),
        LoginEvent::Prompt { text, echo } => {
            return Some(json!({"type":"prompt","value":text,"echo":echo}))
        }
        _ => return None,
    };
    Some(
        json!({"type":kind,"host":info.host,"port":info.port,"algorithm":info.algorithm,"fingerprint":info.fingerprint}),
    )
}

#[no_mangle]
pub extern "system" fn Java_app_termian_mobile_MainActivity_nativeCall(
    mut env: JNIEnv,
    _: JClass,
    request: JString,
    key: JByteArray,
) -> jstring {
    let reply = (|| -> Result<Value, String> {
        let request = Zeroizing::new(String::from(
            env.get_string(&request).map_err(|e| e.to_string())?,
        ));
        let args: Value = serde_json::from_str(&request).map_err(|_| "Некорректная команда.")?;
        let mut app = APP.lock().map_err(|_| "Перезапустите приложение.")?;
        match text(&args, "op") {
            "open" => {
                let raw = Zeroizing::new(env.convert_byte_array(&key).map_err(|e| e.to_string())?);
                let bytes: [u8; 32] = raw
                    .as_slice()
                    .try_into()
                    .map_err(|_| "Некорректный ключ Vault.")?;
                let opened = App::new(PathBuf::from(text(&args, "root")), bytes)?;
                let hosts = opened.hosts();
                *app = Some(opened);
                Ok(json!({"hosts":hosts}))
            }
            "close" => {
                if let Some(open) = app.as_mut() {
                    for id in open.sessions.keys().copied().collect::<Vec<_>>() {
                        open.close_tab(id);
                    }
                }
                *app = None;
                Ok(json!({}))
            }
            _ => app
                .as_mut()
                .ok_or_else(|| "Vault не открыт.".to_owned())?
                .call(&args),
        }
    })()
    .unwrap_or_else(|error| json!({"error":error}));
    env.new_string(reply.to_string())
        .map(JString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}
