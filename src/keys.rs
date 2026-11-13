//! Checks a private key typed or imported in the key form, the way the
//! legacy form does: whether it parses, needs a passphrase, and which public
//! key and algorithm it has.

use russh::keys::{decode_secret_key, PrivateKey};

use crate::vault::key_kind;

#[derive(Clone, Debug, PartialEq)]
pub enum KeyCheck {
    Empty,
    /// Encrypted, and the passphrase is missing or wrong.
    NeedsPassphrase,
    /// Shown as "Incorrect format".
    Invalid,
    Valid { kind: String, public_key: String },
}

/// Decodes an OpenSSH, PEM or PuTTY private key.
pub fn decode(private_key: &str, passphrase: Option<&str>) -> Result<PrivateKey, KeyCheck> {
    if private_key.trim().is_empty() {
        return Err(KeyCheck::Empty);
    }
    let passphrase = passphrase.filter(|p| !p.is_empty());
    match decode_secret_key(private_key, passphrase) {
        Ok(key) => Ok(key),
        Err(_) if is_encrypted(private_key) => Err(KeyCheck::NeedsPassphrase),
        Err(_) => Err(KeyCheck::Invalid),
    }
}

pub fn check(private_key: &str, passphrase: Option<&str>) -> KeyCheck {
    match decode(private_key, passphrase) {
        Ok(key) => {
            let public_key = key.public_key().to_openssh().unwrap_or_default();
            KeyCheck::Valid { kind: key_kind(&public_key), public_key }
        }
        Err(check) => check,
    }
}

/// Whether the key needs a passphrase: the legacy form shows the Passphrase field then.
pub fn is_encrypted(private_key: &str) -> bool {
    if private_key.contains("ENCRYPTED") || private_key.contains("Proc-Type: 4,ENCRYPTED") {
        return true;
    }
    if private_key.trim_start().starts_with("PuTTY-User-Key-File-") {
        return private_key.lines().any(|l| l.starts_with("Encryption:") && !l.ends_with("none"));
    }
    PrivateKey::from_openssh(private_key).is_ok_and(|key| key.is_encrypted())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Throwaway test keys, generated for these tests only.
    pub const PLAIN: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACBmNFf3F19mQdrK1wY3C16YQqSTxV/ZdWBBXRzFcNeDsgAAAJAAyQ0CAMkN
AgAAAAtzc2gtZWQyNTUxOQAAACBmNFf3F19mQdrK1wY3C16YQqSTxV/ZdWBBXRzFcNeDsg
AAAECQYVR85HVFxNwoJetQHXKlZnGHf9TUTj4kGBMR+gkQRmY0V/cXX2ZB2srXBjcLXphC
pJPFX9l1YEFdHMVw14OyAAAADHRlcm1pYW4tdGVzdAE=
-----END OPENSSH PRIVATE KEY-----
";
    pub const PLAIN_PUBLIC: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGY0V/cXX2ZB2srXBjcLXphCpJPFX9l1YEFdHMVw14Oy";
    pub const ENCRYPTED: &str = "-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABDvSoRYAp
QkTBWic/0r5gkdAAAAGAAAAAEAAAAzAAAAC3NzaC1lZDI1NTE5AAAAIKvv1Go/SmoiU2Ay
tErU783+Mot7TyRjOC6ZPTyR66XsAAAAkNzivjXsiJHZHi8cS2Qcof4ilZZeK+oTPaygCI
TTvI2fMkmRvCoo2AIPdZYy4F+2HPEb0IE9l4hAym46bjEz65U7oOB+rIKvUS2JjqJWk/7Z
QcWoupCbydVgcFsVnDV1FqxCaTukq9T97QlAwUZGjvpbUEjbj56lS/mcHhZaD1sYKb4e4H
v0vrHK56LAVmqA9g==
-----END OPENSSH PRIVATE KEY-----
";
    pub const PASSPHRASE: &str = "fixture-pass";

    #[test]
    fn a_plain_key_gives_its_public_key_and_type() {
        let KeyCheck::Valid { kind, public_key } = check(PLAIN, None) else { panic!("valid key") };
        assert_eq!(kind, "ED25519");
        assert!(public_key.starts_with(PLAIN_PUBLIC), "{public_key}");
    }

    #[test]
    fn an_encrypted_key_asks_for_its_passphrase() {
        assert!(is_encrypted(ENCRYPTED) && !is_encrypted(PLAIN));
        assert_eq!(check(ENCRYPTED, None), KeyCheck::NeedsPassphrase);
        assert_eq!(check(ENCRYPTED, Some("wrong")), KeyCheck::NeedsPassphrase);
        assert!(matches!(check(ENCRYPTED, Some(PASSPHRASE)), KeyCheck::Valid { .. }));
    }

    #[test]
    fn junk_is_incorrect_format() {
        assert_eq!(check("hello", None), KeyCheck::Invalid);
        assert_eq!(check("  ", None), KeyCheck::Empty);
    }
}
