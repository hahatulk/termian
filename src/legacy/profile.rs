//! Read-only import of the legacy (WebKit/macOS) profile. Every IndexedDB
//! database is copied to a temporary directory before it is opened, so the
//! legacy profile is never written to or locked.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use super::cipher::FieldCipher;
use super::ssv::{self, Value};
use crate::vault::{key_kind, ForwardingRule, Group, HistoryEntry, Host, Identity, KnownHost, LogRef, RuleKind, ShellCommand, Snippet, SnippetPackage, SshKey, Vault};

/// Keychain entry that holds the legacy local key.
pub const KEYCHAIN_SERVICE: &str = "termian";
pub const KEYCHAIN_ACCOUNT: &str = "localKey";

/// One object store: record id -> value.
type Store = Vec<(u64, Value)>;

/// The legacy WebKit IndexedDB directory for the default macOS profile.
pub fn default_indexeddb_dir() -> Option<PathBuf> {
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/WebKit/app.termian.desktop/WebsiteData/Default");
    // Layout: Default/<origin hash>/<origin hash>/IndexedDB
    fs::read_dir(base).ok()?.flatten().find_map(|outer| {
        fs::read_dir(outer.path()).ok()?.flatten().map(|inner| inner.path().join("IndexedDB")).find(|p| p.is_dir())
    })
}

/// Where the legacy client keeps session logs: `TERMIAN_LEGACY_DATA`, or its
/// app data folder.
pub fn default_logs_dir() -> Option<PathBuf> {
    let base = std::env::var_os("TERMIAN_LEGACY_DATA").map(PathBuf::from).or_else(|| Some(dirs::data_dir()?.join("app.termian.desktop")))?;
    Some(base.join("session-logs-v2"))
}

pub fn read_local_key() -> Result<String, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .and_then(|entry| entry.get_password())
        .map_err(|error| format!("legacy local key unavailable: {error}"))
}

pub fn load_vault(indexeddb_dir: &Path, cipher: &FieldCipher) -> Result<Vault, String> {
    let stores = read_stores(indexeddb_dir)?;
    let empty = Store::new();
    let store = |name: &str| stores.get(name).unwrap_or(&empty);
    let text = |value: Option<&Value>| value.and_then(Value::as_str).map(|s| decrypt_or_plain(cipher, s));

    let groups = store("groups")
        .iter()
        .map(|(id, v)| Group { id: *id, label: text(v.get("label")).unwrap_or_default(), parent: link(v, "parent_group") })
        .collect();

    let usernames: HashMap<u64, String> =
        store("ssh_identities").iter().filter_map(|(id, v)| Some((*id, text(v.get("username"))?))).collect();
    let mut config_identity: HashMap<u64, u64> =
        store("ssh_configs").iter().filter_map(|(id, v)| Some((*id, link(v, "identity")?))).collect();
    let config_ports: HashMap<u64, u16> = store("ssh_configs")
        .iter()
        .filter_map(|(id, v)| Some((*id, u16::try_from(v.get("port")?.as_f64()? as u64).ok()?)))
        .collect();
    for (_, v) in store("ssh_config_identities") {
        if let (Some(config), Some(identity)) = (link(v, "ssh_config"), link(v, "identity")) {
            config_identity.insert(config, identity);
        }
    }

    let tag_labels: HashMap<u64, String> =
        store("tags").iter().map(|(id, v)| (*id, text(v.get("label")).unwrap_or_default())).collect();
    let mut host_tags: HashMap<u64, Vec<String>> = HashMap::new();
    for (_, v) in store("tag_hosts") {
        if let (Some(host), Some(label)) = (link(v, "host"), link(v, "tag").and_then(|t| tag_labels.get(&t))) {
            host_tags.entry(host).or_default().push(label.clone());
        }
    }

    let hosts = store("hosts")
        .iter()
        .map(|(id, v)| Host {
            id: *id,
            label: text(v.get("label")).unwrap_or_default(),
            address: text(v.get("address")).unwrap_or_default(),
            os: v.get("os_name").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned),
            group: link(v, "group"),
            username: link(v, "ssh_config")
                .and_then(|config| config_identity.get(&config))
                .and_then(|identity| usernames.get(identity))
                .cloned(),
            tags: host_tags.remove(id).unwrap_or_default(),
            port: link(v, "ssh_config").and_then(|config| config_ports.get(&config)).copied(),
            password: None,
            key: None,
            identity: None,
        })
        .collect();

    // Secrets (passwords, private keys, passphrases) are never decrypted here;
    // only whether they are present.
    let keys = store("keys")
        .iter()
        .map(|(id, v)| SshKey {
            id: *id,
            label: text(v.get("label")).unwrap_or_default(),
            kind: key_kind(&text(v.get("public_key")).unwrap_or_default()),
            ..SshKey::default()
        })
        .collect();
    let identities = store("ssh_identities")
        .iter()
        .map(|(id, v)| Identity {
            id: *id,
            label: text(v.get("label")).filter(|l| !l.is_empty()).or_else(|| usernames.get(id).cloned()).unwrap_or_default(),
            has_password: v.get("password").and_then(Value::as_str).is_some_and(|p| !p.is_empty()),
            has_key: link(v, "ssh_key").is_some(),
            username: usernames.get(id).cloned().unwrap_or_default(),
            ..Identity::default()
        })
        .collect();
    let snippets = store("snippets")
        .iter()
        .map(|(id, v)| Snippet {
            id: *id,
            label: text(v.get("label")).unwrap_or_default(),
            script: text(v.get("script")).unwrap_or_default(),
            package: link(v, "package"),
            targets: match v.get("targetHosts") {
                Some(Value::Array(hosts)) => hosts.iter().filter_map(|h| h.get("local_id")?.as_f64().map(|n| n as u64)).collect(),
                _ => Vec::new(),
            },
        })
        .collect();
    let packages = store("snippets_packages")
        .iter()
        .map(|(id, v)| SnippetPackage { id: *id, label: text(v.get("label")).unwrap_or_default() })
        .collect();
    let known_hosts = store("known_hosts")
        .iter()
        .map(|(id, v)| KnownHost {
            id: *id,
            host: text(v.get("hostname").or_else(|| v.get("hostnames"))).unwrap_or_default(),
        })
        .collect();

    let rules = store("pf_rules")
        .iter()
        .map(|(id, v)| {
            let port = |field: &str| v.get(field).and_then(Value::as_f64).and_then(|n| u16::try_from(n as u64).ok());
            ForwardingRule {
                id: *id,
                label: text(v.get("label")).unwrap_or_default(),
                kind: match text(v.get("pf_type")).as_deref() {
                    Some("Remote Rule") => RuleKind::Remote,
                    Some("Dynamic Rule") => RuleKind::Dynamic,
                    _ => RuleKind::Local,
                },
                host: link(v, "host"),
                bind_address: text(v.get("bound_address")).filter(|a| !a.is_empty()).unwrap_or_else(|| "127.0.0.1".into()),
                port: port("local_port"),
                destination: text(v.get("hostname")).unwrap_or_default(),
                destination_port: port("remote_port"),
            }
        })
        .collect();

    // Secrets are never decrypted here, so this data must not be uploaded as is.
    Ok(Vault { groups, hosts, keys, identities, snippets, packages, known_hosts, rules, secrets_missing: true, ..Vault::default() })
}

/// References between records look like `{ local_id: 39, id: undefined }`.
/// Connections from the legacy `activities` store, newest first. The
/// encrypted `command` holds what legacy `dbToEntity` reads: `title`,
/// `host`, `user_name`, `connection_type`, `session_log_data`. Passwords in
/// old records are not read. Logs keep their legacy file names.
/// Shell History (`history_commands`); deleted records are skipped.
pub fn load_shell_history(indexeddb_dir: &Path, cipher: &FieldCipher) -> Result<Vec<ShellCommand>, String> {
    let stores = read_stores(indexeddb_dir)?;
    let Some(records) = stores.get("history_commands") else { return Ok(Vec::new()) };
    Ok(records
        .iter()
        .filter(|(_, v)| !v.get("status").and_then(Value::as_str).is_some_and(|s| s.to_uppercase().contains("DELETED")))
        .filter_map(|(_, v)| {
            let script = decrypt_or_plain(cipher, v.get("script")?.as_str()?);
            let ms = v.get("completed_at").and_then(|c| c.as_f64().or_else(|| c.as_str().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.timestamp_millis() as f64)));
            (!script.trim().is_empty()).then(|| ShellCommand {
                script,
                completed_at: ms.map_or(0, |ms| (ms / 1000.) as i64),
                rating: v.get("rating").and_then(Value::as_f64).map_or(1, |r| r as i64),
            })
        })
        .collect())
}

pub fn load_history(indexeddb_dir: &Path, cipher: &FieldCipher) -> Result<Vec<HistoryEntry>, String> {
    let stores = read_stores(indexeddb_dir)?;
    let Some(records) = stores.get("activities") else { return Ok(Vec::new()) };
    let mut entries: Vec<HistoryEntry> = records
        .iter()
        .filter(|(_, v)| !v.get("status").and_then(Value::as_str).is_some_and(|s| s.to_uppercase().contains("DELETED")))
        .filter_map(|(_, v)| {
            let command: serde_json::Value = serde_json::from_str(&cipher.decrypt(v.get("command")?.as_str()?).ok()?).ok()?;
            let field = |name: &str| command.get(name).and_then(serde_json::Value::as_str).unwrap_or("").to_owned();
            let kind = field("connection_type");
            let label = match kind.as_str() {
                "local" => "Local Terminal".to_owned(),
                "serial" => "Serial".to_owned(),
                _ => [field("title"), field("host")].into_iter().find(|s| !s.is_empty()).unwrap_or_else(|| "Unknown".into()),
            };
            let subtitle = if kind == "local" {
                "local".to_owned()
            } else {
                std::iter::once(if kind.is_empty() { "ssh".to_owned() } else { kind.clone() }).chain(Some(field("user_name")).filter(|u| !u.is_empty())).collect::<Vec<_>>().join(", ")
            };
            let connected_ms = v.get("connected_at").and_then(Value::as_f64)?;
            let log = command.get("session_log_data").and_then(|data| {
                let name = data.get("name")?.as_str()?.rsplit(['/', '\\']).next()?.to_owned();
                let secret = data.get("secretKey")?.as_str()?;
                (crate::session_log::valid_name(&name) && secret.len() == 32).then(|| LogRef { name, key: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, secret.as_bytes()) })
            });
            Some(HistoryEntry {
                id: 0,
                host: v.get("host").and_then(|h| h.get("local_id").or(Some(h))).and_then(Value::as_f64).map(|n| n as u64),
                connected_at: (connected_ms / 1000.) as i64,
                disconnected_at: None,
                label,
                subtitle,
                log,
                saved: false,
            })
        })
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.connected_at));
    Ok(entries)
}

fn link(value: &Value, field: &str) -> Option<u64> {
    value.get(field)?.get("local_id")?.as_f64().map(|n| n as u64)
}

/// Secure fields are encrypted; older or default values may be plain text.
fn decrypt_or_plain(cipher: &FieldCipher, field: &str) -> String {
    cipher.decrypt(field).unwrap_or_else(|_| field.to_owned())
}

fn read_stores(indexeddb_dir: &Path) -> Result<HashMap<String, Store>, String> {
    let scratch = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut stores = HashMap::new();
    let entries = fs::read_dir(indexeddb_dir).map_err(|e| format!("{}: {e}", indexeddb_dir.display()))?;
    for (index, entry) in entries.flatten().enumerate() {
        let source = entry.path().join("IndexedDB.sqlite3");
        if !source.is_file() {
            continue;
        }
        let copy = scratch.path().join(index.to_string());
        fs::create_dir(&copy).map_err(|e| e.to_string())?;
        for suffix in ["", "-wal", "-shm"] {
            let file = PathBuf::from(format!("{}{suffix}", source.display()));
            if file.is_file() {
                fs::copy(&file, copy.join(format!("IndexedDB.sqlite3{suffix}"))).map_err(|e| e.to_string())?;
            }
        }
        read_database(&copy.join("IndexedDB.sqlite3"), &mut stores)?;
    }
    Ok(stores)
}

fn read_database(path: &Path, stores: &mut HashMap<String, Store>) -> Result<(), String> {
    // Read-write on the private copy so SQLite can replay the WAL.
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(|e| e.to_string())?;
    let mut query = db
        .prepare("SELECT o.name, r.key, r.value FROM Records r JOIN ObjectStoreInfo o ON o.id = r.objectStoreID")
        .map_err(|e| e.to_string())?;
    let rows = query
        // Keys and values are blobs, but SQLite may hand back TEXT for the key column.
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get_ref(1)?.as_bytes()?.to_vec(), row.get_ref(2)?.as_bytes()?.to_vec())))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (store, key, value) = row.map_err(|e| e.to_string())?;
        let (Some(id), Ok(value)) = (ssv::decode_number_key(&key), ssv::decode(&value)) else { continue };
        stores.entry(store).or_insert_with(Vec::new).push((id as u64, value));
    }
    stores.values_mut().for_each(|records| records.sort_by_key(|(id, _)| *id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::ssv::tests::Encoder;
    use base64::{engine::general_purpose::STANDARD, Engine};

    const KEY: [u8; 32] = [9; 32];

    fn cipher() -> FieldCipher {
        FieldCipher::from_local_key(&STANDARD.encode(KEY)).unwrap()
    }

    /// A record as a list of fields; `Link` renders `{ local_id: n, id: undefined }`.
    enum F<'a> {
        Text(&'a str),
        Secret(&'a str),
        Link(i32),
        Links(&'a [i32]),
        Int(i32),
        Double(f64),
    }

    fn record(fields: &[(&str, F)]) -> Vec<u8> {
        let mut e = Encoder::new();
        e.begin_object();
        for (i, (key, field)) in fields.iter().enumerate() {
            e.key(key);
            match field {
                F::Text(s) => e.string(s),
                F::Int(n) => e.int(*n),
                F::Double(n) => e.double(*n),
                F::Secret(s) => e.string(&cipher().encrypt(s, [i as u8; 24])),
                F::Link(n) => {
                    e.begin_object();
                    e.key("local_id");
                    e.int(*n);
                    e.key("id");
                    e.tag(3);
                    e.end();
                }
                F::Links(ids) => {
                    e.begin_array(ids.len() as u32);
                    for (i, n) in ids.iter().enumerate() {
                        e.index(i as u32);
                        e.begin_object();
                        e.key("local_id");
                        e.int(*n);
                        e.end();
                    }
                    e.end();
                }
            }
        }
        e.end();
        e.out
    }

    fn key(id: f64) -> Vec<u8> {
        let mut k = vec![0x00, 0x20];
        k.extend(id.to_le_bytes());
        k
    }

    fn write_store(root: &Path, name: &str, records: Vec<(f64, Vec<u8>)>) {
        let dir = root.join(format!("db-{name}"));
        fs::create_dir(&dir).unwrap();
        let db = Connection::open(dir.join("IndexedDB.sqlite3")).unwrap();
        db.execute_batch(
            "CREATE TABLE ObjectStoreInfo (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE Records (objectStoreID INTEGER, key TEXT, value BLOB, recordID INTEGER PRIMARY KEY);",
        )
        .unwrap();
        db.execute("INSERT INTO ObjectStoreInfo VALUES (1, ?1)", [name]).unwrap();
        for (id, value) in records {
            db.execute("INSERT INTO Records (objectStoreID, key, value) VALUES (1, ?1, ?2)", (key(id), value)).unwrap();
        }
    }

    #[test]
    fn loads_connection_history_with_its_logs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ssh = r#"{"connection_type":"ssh","host":"192.0.2.10","port":22,"title":"Ugoloc Prod","user_name":"root","version_code":1,
            "session_log_data":{"name":"/Users/me/Library/Application Support/app.termian.desktop/session-logs-v2/6f9a1c2e-1b2c.log","secretKey":"0123456789abcdef0123456789abcdef"}}"#;
        let local = r#"{"connection_type":"local","host":"","title":""}"#;
        let quick = r#"{"connection_type":"ssh","host":"10.0.0.5","title":"","user_name":""}"#;
        write_store(root, "activities", vec![
            (1., record(&[("command", F::Secret(ssh)), ("connected_at", F::Double(1_794_000_000_000.)), ("host", F::Link(220)), ("status", F::Text("TO_BE_CREATED"))])),
            (2., record(&[("command", F::Secret(local)), ("connected_at", F::Double(1_794_100_000_000.)), ("status", F::Text("TO_BE_UPDATED"))])),
            (3., record(&[("command", F::Secret(quick)), ("connected_at", F::Double(1_793_000_000_000.)), ("status", F::Text("TO_BE_DELETED"))])),
            (4., record(&[("command", F::Text("not encrypted json")), ("connected_at", F::Double(1.))])),
        ]);
        let history = load_history(root, &cipher()).unwrap();
        assert_eq!(history.len(), 2, "deleted and unreadable records are skipped");
        assert_eq!((history[0].label.as_str(), history[0].subtitle.as_str(), history[0].host), ("Local Terminal", "local", None));
        let ssh = &history[1];
        assert_eq!((ssh.label.as_str(), ssh.subtitle.as_str(), ssh.host, ssh.connected_at), ("Ugoloc Prod", "ssh, root", Some(220), 1_794_000_000));
        let log = ssh.log.as_ref().unwrap();
        assert_eq!(log.name, "6f9a1c2e-1b2c.log");
        assert_eq!(STANDARD.decode(&log.key).unwrap(), b"0123456789abcdef0123456789abcdef");
        assert_eq!(crate::session_log::file_name(&log.name), "6f9a1c2e-1b2c.log");

        let mut vault = Vault { history: vec![HistoryEntry { id: 7, connected_at: 1_794_000_000, label: "Ugoloc Prod".into(), ..HistoryEntry::default() }], ..Vault::default() };
        assert_eq!(vault.import_history(history), 1, "the same connection is not added twice");
        assert!(vault.legacy_history_imported);
        assert_eq!(vault.history.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(), ["Local Terminal", "Ugoloc Prod"]);
        assert!(vault.history[0].id > 7);
    }

    #[test]
    fn loads_decrypted_hosts_groups_users_and_tags() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_store(root, "groups", vec![
            (1., record(&[("label", F::Secret("work"))])),
            (3., record(&[("label", F::Secret("Ugoloc")), ("parent_group", F::Link(1))])),
        ]);
        write_store(root, "hosts", vec![
            (220., record(&[
                ("label", F::Secret("Ugoloc Prod")),
                ("address", F::Secret("192.0.2.10")),
                ("os_name", F::Text("debian")),
                ("group", F::Link(3)),
                ("ssh_config", F::Link(230)),
            ])),
            (221., record(&[("label", F::Text("plain"))])),
        ]);
        write_store(root, "ssh_configs", vec![(230., record(&[]))]);
        write_store(root, "ssh_config_identities", vec![(64., record(&[("ssh_config", F::Link(230)), ("identity", F::Link(22))]))]);
        write_store(root, "ssh_identities", vec![(22., record(&[("username", F::Secret("root"))]))]);
        write_store(root, "tags", vec![(1., record(&[("label", F::Secret("prod"))]))]);
        write_store(root, "tag_hosts", vec![(5., record(&[("host", F::Link(220)), ("tag", F::Link(1))]))]);

        let vault = load_vault(root, &cipher()).unwrap();

        assert_eq!(vault.groups[1], Group { id: 3, label: "Ugoloc".into(), parent: Some(1) });
        let host = &vault.hosts[0];
        assert_eq!((host.label.as_str(), host.address.as_str()), ("Ugoloc Prod", "192.0.2.10"));
        assert_eq!(host.os.as_deref(), Some("debian"));
        assert_eq!(host.group, Some(3));
        assert_eq!(host.subtitle(), "ssh, root, prod");
        assert_eq!(vault.hosts[1].label, "plain");
        assert_eq!(vault.hosts_under(Some(1)).len(), 1);
    }

    #[test]
    fn loads_keychain_snippets_and_known_hosts_without_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_store(root, "keys", vec![(1., record(&[
            ("label", F::Secret("kazgau")),
            ("public_key", F::Secret("ssh-ed25519 AAAA")),
            ("private_key", F::Secret("-----BEGIN OPENSSH PRIVATE KEY-----")),
        ]))]);
        write_store(root, "ssh_identities", vec![
            (22., record(&[("label", F::Secret("kazgau")), ("password", F::Secret("pw")), ("ssh_key", F::Link(1))])),
            (23., record(&[("username", F::Secret("root")), ("ssh_key", F::Link(1))])),
        ]);
        write_store(root, "snippets", vec![(3., record(&[
            ("label", F::Secret("get pods")),
            ("script", F::Secret("kubectl get pods")),
            ("package", F::Link(5)),
            ("targetHosts", F::Links(&[7, 8])),
        ]))]);
        write_store(root, "snippets_packages", vec![(5., record(&[("label", F::Secret("k8s"))]))]);
        write_store(root, "known_hosts", vec![(4., record(&[("hostname", F::Secret("[10.0.0.1]:2001"))]))]);

        let vault = load_vault(root, &cipher()).unwrap();

        assert_eq!(vault.keys, vec![SshKey { id: 1, label: "kazgau".into(), kind: "ED25519".into(), ..SshKey::default() }]);
        assert_eq!(vault.identities[0].subtitle(), "Auth password and key");
        assert_eq!((vault.identities[1].label.as_str(), vault.identities[1].subtitle()), ("root", "Auth key"));
        assert_eq!(vault.snippets[0].subtitle(), Some("kubectl get pods"));
        assert_eq!((vault.snippets[0].package, vault.snippets[0].targets.as_slice()), (Some(5), &[7, 8][..]));
        assert_eq!(vault.packages, vec![SnippetPackage { id: 5, label: "k8s".into() }]);
        assert_eq!(vault.known_hosts[0].host, "[10.0.0.1]:2001");
        let debug = format!("{vault:?}");
        assert!(!debug.contains("PRIVATE KEY") && !debug.contains("\"pw\""), "secrets leaked: {debug}");
    }

    #[test]
    fn loads_shell_history_commands() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_store(root, "history_commands", vec![
            (1., record(&[("script", F::Secret("ls -la")), ("completed_at", F::Double(1_794_000_000_000.)), ("rating", F::Int(2))])),
            (2., record(&[("script", F::Text("uptime"))])),
            (3., record(&[("script", F::Secret("gone")), ("status", F::Text("TO_BE_DELETED"))])),
        ]);
        let commands = load_shell_history(root, &cipher()).unwrap();
        assert_eq!(commands, vec![
            ShellCommand { script: "ls -la".into(), completed_at: 1_794_000_000, rating: 2 },
            ShellCommand { script: "uptime".into(), completed_at: 0, rating: 1 },
        ]);
    }

    #[test]
    fn loads_port_forwarding_rules() {
        let dir = tempfile::tempdir().unwrap();
        write_store(dir.path(), "pf_rules", vec![(9., record(&[
            ("label", F::Secret("postgres")),
            ("pf_type", F::Text("Remote Rule")),
            ("host", F::Link(220)),
            ("local_port", F::Int(8080)),
            ("hostname", F::Secret("127.0.0.1")),
            ("remote_port", F::Int(5432)),
        ]))]);
        let vault = load_vault(dir.path(), &cipher()).unwrap();
        assert_eq!(vault.rules, vec![ForwardingRule {
            id: 9,
            label: "postgres".into(),
            kind: RuleKind::Remote,
            host: Some(220),
            bind_address: "127.0.0.1".into(),
            port: Some(8080),
            destination: "127.0.0.1".into(),
            destination_port: Some(5432),
        }]);
        assert!(vault.secrets_missing, "profile data is never sent to the cloud as is");
    }

    #[test]
    fn leaves_the_legacy_profile_untouched() {
        let dir = tempfile::tempdir().unwrap();
        write_store(dir.path(), "hosts", vec![(1., record(&[("label", F::Text("h"))]))]);
        let file = dir.path().join("db-hosts/IndexedDB.sqlite3");
        let before = fs::read(&file).unwrap();
        let listing = |p: &Path| fs::read_dir(p).unwrap().count();
        let files_before = listing(&file.parent().unwrap());

        load_vault(dir.path(), &cipher()).unwrap();

        assert_eq!(fs::read(&file).unwrap(), before);
        assert_eq!(listing(&file.parent().unwrap()), files_before);
    }

    #[test]
    fn missing_directory_is_an_error() {
        assert!(load_vault(Path::new("/nonexistent/termian"), &cipher()).is_err());
    }
}
