//! Read-only import of the legacy (WebKit/macOS) profile. Every IndexedDB
//! database is copied to a temporary directory before it is opened, so the
//! legacy profile is never written to or locked.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use super::cipher::FieldCipher;
use super::ssv::{self, Value};
use crate::vault::{Group, Host, Vault};

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
        })
        .collect();

    Ok(Vault { groups, hosts })
}

/// References between records look like `{ local_id: 39, id: undefined }`.
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
    }

    fn record(fields: &[(&str, F)]) -> Vec<u8> {
        let mut e = Encoder::new();
        e.begin_object();
        for (i, (key, field)) in fields.iter().enumerate() {
            e.key(key);
            match field {
                F::Text(s) => e.string(s),
                F::Secret(s) => e.string(&cipher().encrypt(s, [i as u8; 24])),
                F::Link(n) => {
                    e.begin_object();
                    e.key("local_id");
                    e.int(*n);
                    e.key("id");
                    e.tag(3);
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
