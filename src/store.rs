//! The native vault file (ADR 0002). JSON sealed with XSalsa20-Poly1305 and written atomically.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use crypto_secretbox::{aead::Aead, KeyInit, Nonce, XSalsa20Poly1305};
use zeroize::Zeroizing;

use crate::vault::Vault;

#[cfg(target_os = "macos")]
mod macos;

const MAGIC: &[u8] = b"TMNVLT1\n";
const NONCE: usize = 24;
const KEYRING_SERVICE: &str = "termian-native";
const KEYRING_USER: &str = "vault-key";

#[cfg(any(target_os = "macos", test))]
const PROTECTED: &[u8] = b"TMNVLT2\n";
#[cfg(any(target_os = "macos", test))]
const MAX_WRAPPED_KEY: usize = 4096;

type Key = Zeroizing<[u8; 32]>;

#[cfg(any(target_os = "macos", test))]
trait KeyProtection {
    fn wrap(&self, key: &[u8; 32]) -> Result<Vec<u8>, String>;
    fn unwrap(&self, wrapped: &[u8]) -> Result<Key, String>;
}

pub struct Store {
    path: PathBuf,
    cipher: XSalsa20Poly1305,
    header: Vec<u8>,
    _lock: Option<std::fs::File>,
}

impl Store {
    pub fn new(path: PathBuf, key: [u8; 32]) -> Self {
        Self::with_header(path, &Zeroizing::new(key), MAGIC.to_vec())
    }

    fn with_header(path: PathBuf, key: &Key, header: Vec<u8>) -> Self {
        Self {
            path,
            cipher: XSalsa20Poly1305::new((&**key).into()),
            header,
            _lock: None,
        }
    }

    pub fn load_default(
        import: impl FnOnce() -> Result<Vault, String>,
    ) -> Result<(Self, Vault), String> {
        #[cfg(not(any(target_os = "macos", windows)))]
        {
            let _ = import;
            Err("Persistent vault storage is only supported on macOS and Windows.".into())
        }
        #[cfg(any(target_os = "macos", windows))]
        {
            let dir = dirs::data_dir()
                .ok_or("no data directory")?
                .join("termian-native");
            let path = dir.join("vault.bin");
            let lock = lock_vault(&dir)?;
            Self::load_locked(path, lock, import)
        }
    }

    #[cfg(any(target_os = "macos", windows))]
    fn load_locked(
        path: PathBuf,
        lock: std::fs::File,
        import: impl FnOnce() -> Result<Vault, String>,
    ) -> Result<(Self, Vault), String> {
        #[cfg(target_os = "macos")]
        {
            let (mut store, vault, migrated) =
                Self::load_protected(path, &macos::SecureEnclave, read_legacy_key, import)?;
            store._lock = Some(lock);
            if migrated {
                match legacy_entry().and_then(|entry| entry.delete_credential()) {
                    Ok(()) | Err(keyring::Error::NoEntry) => {}
                    Err(_) => eprintln!(
                        "[Termian] The retired vault key could not be deleted from Keychain."
                    ),
                }
            }
            Ok((store, vault))
        }
        #[cfg(windows)]
        {
            let bytes = read_vault(&path)?;
            let key = match read_legacy_key() {
                Ok(key) => key,
                Err(keyring::Error::NoEntry) if bytes.is_none() => {
                    use base64::{engine::general_purpose::STANDARD, Engine};
                    let key = random_key()?;
                    let encoded = Zeroizing::new(STANDARD.encode(&*key));
                    legacy_entry()
                        .and_then(|entry| entry.set_password(&encoded))
                        .map_err(|e| e.to_string())?;
                    key
                }
                Err(error) => return Err(format!("vault key unavailable: {error}")),
            };
            let mut store = Self::with_header(path, &key, MAGIC.to_vec());
            store._lock = Some(lock);
            let vault = store.load_or_import(import)?;
            Ok((store, vault))
        }
    }

    #[cfg(any(target_os = "macos", test))]
    fn load_protected(
        path: PathBuf,
        protection: &impl KeyProtection,
        legacy_key: impl FnOnce() -> Result<Key, keyring::Error>,
        import: impl FnOnce() -> Result<Vault, String>,
    ) -> Result<(Self, Vault, bool), String> {
        let bytes = read_vault(&path)?;
        if let Some(raw) = bytes.as_deref() {
            if raw.starts_with(PROTECTED) {
                let (header, wrapped) = wrapped_header(raw)?;
                let key = protection.unwrap(wrapped)?;
                let store = Self::with_header(path, &key, header.to_vec());
                let vault = store.decode(raw)?;
                return Ok((store, vault, false));
            }
            if !raw.starts_with(MAGIC) || raw.len() < MAGIC.len() + NONCE + 16 {
                return Err("vault file is damaged".into());
            }
        }
        let key = random_key()?;
        let wrapped = protection.wrap(&key)?;
        if wrapped.is_empty() || wrapped.len() > MAX_WRAPPED_KEY {
            return Err("invalid wrapped vault key".into());
        }
        if *protection.unwrap(&wrapped)? != *key {
            return Err("macOS returned the wrong vault key".into());
        }
        let vault = if let Some(raw) = bytes.as_deref() {
            let old = legacy_key().map_err(|e| format!("previous vault key unavailable: {e}"))?;
            Self::with_header(path.clone(), &old, MAGIC.to_vec()).decode(raw)?
        } else {
            import()?
        };
        let header = [
            PROTECTED,
            &(wrapped.len() as u16).to_be_bytes(),
            wrapped.as_slice(),
        ]
        .concat();
        let store = Self::with_header(path, &key, header);
        store.save_inner(&vault, true)?;
        Ok((store, vault, bytes.is_some()))
    }

    /// Demo mode: a throwaway file with a fixed key, never the keychain.
    pub fn demo() -> Self {
        Self::new(
            std::env::temp_dir()
                .join("termian-native-demo")
                .join("vault.bin"),
            [0x5a; 32],
        )
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn decrypt(&self, bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>, String> {
        let sealed = bytes
            .strip_prefix(self.header.as_slice())
            .filter(|rest| rest.len() >= NONCE + 16)
            .ok_or("vault file is damaged")?;
        self.cipher
            .decrypt(Nonce::from_slice(&sealed[..NONCE]), &sealed[NONCE..])
            .map(Zeroizing::new)
            .map_err(|_| "vault file cannot be decrypted with this key".into())
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vault, String> {
        let plain = self.decrypt(bytes)?;
        serde_json::from_slice(&plain).map_err(|e| format!("vault file is damaged: {e}"))
    }

    /// The saved vault, or `None` when nothing was saved yet.
    pub fn load(&self) -> Result<Option<Vault>, String> {
        read_vault(&self.path)?
            .as_deref()
            .map(|bytes| self.decode(bytes))
            .transpose()
    }

    pub fn save(&self, vault: &Vault) -> Result<(), String> {
        self.save_inner(vault, false)
    }

    fn save_inner(&self, vault: &Vault, verify: bool) -> Result<(), String> {
        let plain = Zeroizing::new(serde_json::to_vec(vault).map_err(|e| e.to_string())?);
        let mut nonce = [0u8; NONCE];
        getrandom::getrandom(&mut nonce).map_err(|e| e.to_string())?;
        let sealed = self
            .cipher
            .encrypt(Nonce::from_slice(&nonce), plain.as_slice())
            .map_err(|_| "encryption failed")?;
        let dir = self.path.parent().ok_or("vault path has no folder")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        temporary
            .write_all(&self.header)
            .and_then(|_| temporary.write_all(&nonce))
            .and_then(|_| temporary.write_all(&sealed))
            .map_err(|e| e.to_string())?;
        temporary.as_file().sync_all().map_err(|e| e.to_string())?;
        if verify {
            let check = std::fs::read(temporary.path()).map_err(|e| e.to_string())?;
            if self.decrypt(&check)?.as_slice() != plain.as_slice() {
                return Err("vault verification failed".into());
            }
        }
        temporary
            .persist(&self.path)
            .map_err(|e| format!("{}: {e}", self.path.display()))?;
        #[cfg(unix)]
        {
            std::fs::File::open(dir)
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// The saved vault; on first run, `import()` (the legacy profile) saved as the start.
    pub fn load_or_import(
        &self,
        import: impl FnOnce() -> Result<Vault, String>,
    ) -> Result<Vault, String> {
        if let Some(vault) = self.load()? {
            return Ok(vault);
        }
        let vault = import()?;
        self.save(&vault)?;
        Ok(vault)
    }
}

#[cfg(any(target_os = "macos", windows))]
fn lock_vault(dir: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let lock = std::fs::OpenOptions::new()
        .read(true).write(true).create(true).truncate(false)
        .open(dir.join("vault.lock")).map_err(|e| e.to_string())?;
    lock.try_lock().map_err(|e| match e {
        std::fs::TryLockError::WouldBlock => "Close the other Termian window before opening this vault.".into(),
        std::fs::TryLockError::Error(error) => error.to_string(),
    })?;
    Ok(lock)
}

fn read_vault(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

#[cfg(any(target_os = "macos", test))]
fn wrapped_header(bytes: &[u8]) -> Result<(&[u8], &[u8]), String> {
    let start = PROTECTED.len() + 2;
    let size = bytes
        .get(PROTECTED.len()..start)
        .and_then(|size| size.try_into().ok())
        .map(u16::from_be_bytes)
        .map(usize::from)
        .filter(|size| (1..=MAX_WRAPPED_KEY).contains(size))
        .ok_or("vault file is damaged")?;
    let end = start + size;
    if bytes.len() < end + NONCE + 16 {
        return Err("vault file is damaged".into());
    }
    Ok((&bytes[..end], &bytes[start..end]))
}

fn legacy_entry() -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
}

fn read_legacy_key() -> Result<Key, keyring::Error> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let saved = Zeroizing::new(legacy_entry()?.get_password()?);
    let mut key = Zeroizing::new([0; 32]);
    match STANDARD.decode_slice(saved.trim(), &mut *key) {
        Ok(32) => Ok(key),
        _ => Err(keyring::Error::Invalid(
            "vault key".into(),
            "invalid encoding or length".into(),
        )),
    }
}

fn random_key() -> Result<Key, String> {
    let mut key = Zeroizing::new([0; 32]);
    getrandom::getrandom(&mut *key).map_err(|e| e.to_string())?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::tests::host;

    #[cfg(any(target_os = "macos", windows))]
    #[test]
    fn vault_lock_excludes_other_openers_until_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let guard = lock_vault(dir.path()).unwrap();
        assert!(lock_vault(dir.path()).is_err());
        drop(guard);
        assert!(lock_vault(dir.path()).is_ok());
    }

    struct TestProtection(bool);

    impl KeyProtection for TestProtection {
        fn wrap(&self, key: &[u8; 32]) -> Result<Vec<u8>, String> {
            let cipher = XSalsa20Poly1305::new((&[5; 32]).into());
            let mut nonce = [0; NONCE];
            getrandom::getrandom(&mut nonce).unwrap();
            let sealed = cipher
                .encrypt(Nonce::from_slice(&nonce), key.as_slice())
                .unwrap();
            Ok([nonce.as_slice(), sealed.as_slice()].concat())
        }

        fn unwrap(&self, wrapped: &[u8]) -> Result<Key, String> {
            if !self.0 {
                return Err("Authentication was canceled.".into());
            }
            let cipher = XSalsa20Poly1305::new((&[5; 32]).into());
            let plain = cipher
                .decrypt(Nonce::from_slice(&wrapped[..NONCE]), &wrapped[NONCE..])
                .map_err(|_| "wrong key")?;
            Ok(Zeroizing::new(
                plain.try_into().map_err(|_| "wrong length")?,
            ))
        }
    }

    #[test]
    fn canceled_authentication_cannot_read_or_replace_a_vault() {
        let dir = tempfile::tempdir().unwrap();
        let old = store(dir.path());
        for expected in [None, Some(Vault::default())] {
            if let Some(vault) = expected {
                old.save(&vault).unwrap();
            }
            let original = read_vault(old.path()).unwrap();
            let result = Store::load_protected(
                old.path().into(),
                &TestProtection(false),
                || panic!("must authenticate before reading the old key"),
                || panic!("must authenticate before importing"),
            );
            assert_eq!(
                result.err().as_deref(),
                Some("Authentication was canceled.")
            );
            assert_eq!(read_vault(old.path()).unwrap(), original);
        }
    }

    #[test]
    fn migration_preserves_secrets_and_merge_state_after_relaunch() {
        let dir = tempfile::tempdir().unwrap();
        let backups = dir.path().join("backups");
        let (url, server) = crate::cloud::tests::server();
        let mut remote = Vault {
            cloud: Some(crate::cloud::tests::config(&url)),
            hosts: vec![crate::vault::Host {
                password: Some("migration-test-secret".into()),
                ..host(1, "db", None)
            }],
            ..Vault::default()
        };
        crate::self_cloud::upload(&mut remote).unwrap();
        let mut local = Vault {
            cloud: remote.cloud.clone(),
            ..Vault::default()
        };
        crate::self_cloud::download(&mut local, &backups).unwrap();
        assert!(local.sync_state.is_some());

        let old = store(dir.path());
        old.save(&local).unwrap();
        let original = read_vault(old.path()).unwrap();
        let result = Store::load_protected(
            old.path().into(),
            &TestProtection(true),
            || Err(keyring::Error::NoEntry),
            || panic!("existing vault must not be replaced"),
        );
        assert!(result.is_err());
        assert_eq!(read_vault(old.path()).unwrap(), original);

        let (protected, loaded, migrated) = Store::load_protected(
            old.path().into(),
            &TestProtection(true),
            || Ok(Zeroizing::new([3; 32])),
            || panic!("existing vault must not be replaced"),
        )
        .unwrap();
        assert!(migrated);
        assert_eq!(loaded, local);
        assert!(old.load().is_err());
        let sealed = read_vault(protected.path()).unwrap().unwrap();
        assert!(sealed.starts_with(PROTECTED));
        assert!(!sealed
            .windows(b"migration-test-secret".len())
            .any(|w| w == b"migration-test-secret"));

        let denied = Store::load_protected(
            protected.path().into(),
            &TestProtection(false),
            || panic!("the old key must not be used"),
            || panic!("must not import"),
        );
        assert_eq!(
            denied.err().as_deref(),
            Some("Authentication was canceled.")
        );

        let (reopened, mut loaded, migrated) = Store::load_protected(
            protected.path().into(),
            &TestProtection(true),
            || panic!("the old key must not be used"),
            || panic!("must not import"),
        )
        .unwrap();
        assert!(!migrated);
        assert_eq!(loaded, local);

        let before = server.lock().unwrap().clone();
        assert_eq!(
            crate::self_cloud::sync(&mut loaded, &backups).unwrap(),
            "Данные уже синхронизированы"
        );
        assert_eq!(*server.lock().unwrap(), before);
        reopened.save(&loaded).unwrap();
        assert_eq!(reopened.load().unwrap(), Some(loaded));
    }

    fn store(dir: &Path) -> Store {
        Store::new(dir.join("data").join("vault.bin"), [3; 32])
    }

    #[test]
    fn saves_and_loads_a_vault_with_secrets_sealed() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        assert_eq!(store.load().unwrap(), None);
        let mut vault = Vault::default();
        vault.hosts.push(crate::vault::Host {
            password: Some("hunter2-secret".into()),
            port: Some(2222),
            ..host(1, "db", None)
        });
        store.save(&vault).unwrap();
        assert_eq!(store.load().unwrap(), Some(vault));
        let raw = std::fs::read(store.path()).unwrap();
        assert!(raw.starts_with(MAGIC));
        assert!(
            !raw.windows(14).any(|w| w == b"hunter2-secret"),
            "the password is not in the file in clear text"
        );
        assert!(!store.path().with_extension("bin.tmp").exists());
    }

    #[test]
    fn saved_log_comments_stay_in_the_encrypted_device_vault() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let mut vault = Vault::default();
        let id = vault.record_connection(crate::vault::HistoryEntry {
            log: Some(crate::vault::LogRef {
                name: "local-log".into(),
                key: "k".into(),
            }),
            ..Default::default()
        });
        vault.save_log_comment(id, "private-comment-не-публиковать");
        store.save(&vault).unwrap();
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(
            loaded.logs(true)[0].comment,
            "private-comment-не-публиковать"
        );
        assert!(loaded.logs(true)[0].saved);
        let raw = std::fs::read(store.path()).unwrap();
        assert!(!raw
            .windows(b"private-comment".len())
            .any(|w| w == b"private-comment"));
        assert!(
            !crate::portable::export(&loaded, "2026-11-14")
                .to_string()
                .contains("private-comment"),
            "comments are not part of the sync snapshot"
        );
    }

    #[test]
    fn a_wrong_key_or_damage_is_an_error_not_an_empty_vault() {
        let dir = tempfile::tempdir().unwrap();
        store(dir.path()).save(&Vault::default()).unwrap();
        let other = Store::new(dir.path().join("data").join("vault.bin"), [4; 32]);
        assert!(other.load().is_err());
        std::fs::write(other.path(), b"junk").unwrap();
        assert!(other.load().is_err());
    }

    #[test]
    fn imports_once_then_keeps_its_own_copy() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        let imported = store
            .load_or_import(|| {
                Ok(Vault {
                    hosts: vec![host(1, "from legacy", None)],
                    ..Vault::default()
                })
            })
            .unwrap();
        assert_eq!(imported.hosts.len(), 1);
        let again = store
            .load_or_import(|| panic!("the legacy profile is read only once"))
            .unwrap();
        assert_eq!(again, imported);
    }
}
