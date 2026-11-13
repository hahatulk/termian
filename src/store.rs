//! The native client's own vault file (ADR 0002): JSON sealed with
//! XSalsa20-Poly1305, `TMNVLT1\n` + nonce + box, written atomically.

use std::path::{Path, PathBuf};

use crypto_secretbox::{aead::Aead, KeyInit, Nonce, XSalsa20Poly1305};

use crate::vault::Vault;

const MAGIC: &[u8] = b"TMNVLT1\n";
const NONCE: usize = 24;
const KEYRING_SERVICE: &str = "termian-native";
const KEYRING_USER: &str = "vault-key";

pub struct Store {
    path: PathBuf,
    cipher: XSalsa20Poly1305,
}

impl Store {
    pub fn new(path: PathBuf, key: [u8; 32]) -> Self {
        Self { path, cipher: XSalsa20Poly1305::new(&key.into()) }
    }

    /// `vault.bin` in the data directory, keyed from the system keychain;
    /// the key is created on first use.
    pub fn open_default() -> Result<Self, String> {
        let dir = dirs::data_dir().ok_or("no data directory")?.join("termian-native");
        Ok(Self::new(dir.join("vault.bin"), keychain_key()?))
    }

    /// Demo mode: a throwaway file with a fixed key, never the keychain.
    pub fn demo() -> Self {
        Self::new(std::env::temp_dir().join("termian-native-demo").join("vault.bin"), [0x5a; 32])
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The saved vault, or `None` when nothing was saved yet.
    pub fn load(&self) -> Result<Option<Vault>, String> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("{}: {error}", self.path.display())),
        };
        let sealed = bytes.strip_prefix(MAGIC).filter(|rest| rest.len() > NONCE).ok_or("vault file is damaged")?;
        let plain = self
            .cipher
            .decrypt(Nonce::from_slice(&sealed[..NONCE]), &sealed[NONCE..])
            .map_err(|_| "vault file cannot be decrypted with this key")?;
        serde_json::from_slice(&plain).map(Some).map_err(|e| format!("vault file is damaged: {e}"))
    }

    pub fn save(&self, vault: &Vault) -> Result<(), String> {
        let plain = serde_json::to_vec(vault).map_err(|e| e.to_string())?;
        let mut nonce = [0u8; NONCE];
        getrandom::getrandom(&mut nonce).map_err(|e| e.to_string())?;
        let sealed = self.cipher.encrypt(Nonce::from_slice(&nonce), plain.as_slice()).map_err(|_| "encryption failed")?;
        let dir = self.path.parent().ok_or("vault path has no folder")?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        // Write next to the file, then rename: a crash never leaves half a vault.
        let temporary = self.path.with_extension("bin.tmp");
        std::fs::write(&temporary, [MAGIC, &nonce, &sealed].concat()).map_err(|e| format!("{}: {e}", temporary.display()))?;
        std::fs::rename(&temporary, &self.path).map_err(|e| format!("{}: {e}", self.path.display()))
    }

    /// The saved vault; on first run, `import()` (the legacy profile) saved as the start.
    pub fn load_or_import(&self, import: impl FnOnce() -> Result<Vault, String>) -> Result<Vault, String> {
        if let Some(vault) = self.load()? {
            return Ok(vault);
        }
        let vault = import()?;
        self.save(&vault)?;
        Ok(vault)
    }
}

fn keychain_key() -> Result<[u8; 32], String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(saved) => STANDARD.decode(saved.trim()).ok().and_then(|k| k.try_into().ok()).ok_or_else(|| "vault key in the keychain is damaged".into()),
        Err(keyring::Error::NoEntry) => {
            let mut key = [0u8; 32];
            getrandom::getrandom(&mut key).map_err(|e| e.to_string())?;
            entry.set_password(&STANDARD.encode(key)).map_err(|e| e.to_string())?;
            Ok(key)
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::tests::host;

    fn store(dir: &Path) -> Store {
        Store::new(dir.join("data").join("vault.bin"), [3; 32])
    }

    #[test]
    fn saves_and_loads_a_vault_with_secrets_sealed() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path());
        assert_eq!(store.load().unwrap(), None);
        let mut vault = Vault::default();
        vault.hosts.push(crate::vault::Host { password: Some("hunter2-secret".into()), port: Some(2222), ..host(1, "db", None) });
        store.save(&vault).unwrap();
        assert_eq!(store.load().unwrap(), Some(vault));
        let raw = std::fs::read(store.path()).unwrap();
        assert!(raw.starts_with(MAGIC));
        assert!(!raw.windows(14).any(|w| w == b"hunter2-secret"), "the password is not in the file in clear text");
        assert!(!store.path().with_extension("bin.tmp").exists());
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
        let imported = store.load_or_import(|| Ok(Vault { hosts: vec![host(1, "from legacy", None)], ..Vault::default() })).unwrap();
        assert_eq!(imported.hosts.len(), 1);
        let again = store.load_or_import(|| panic!("the legacy profile is read only once")).unwrap();
        assert_eq!(again, imported);
    }
}
