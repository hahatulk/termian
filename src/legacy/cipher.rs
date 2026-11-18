//! Field encryption used by the legacy profile: base64 of
//! `[version=4][flags][24-byte nonce][XSalsa20-Poly1305 box]`, keyed by the
//! profile's local key (see legacy/src/bridge/ssh-compat-crypto.js).

use base64::{engine::general_purpose::STANDARD, Engine};
use crypto_secretbox::{aead::Aead, KeyInit, Nonce, XSalsa20Poly1305};

const VERSION: u8 = 4;
const HEADER: usize = 2 + 24;

pub struct FieldCipher(XSalsa20Poly1305);

impl FieldCipher {
    /// `key_base64` is the 32-byte local key as the legacy app stores it,
    /// optionally JSON-quoted.
    pub fn from_local_key(key_base64: &str) -> Result<Self, String> {
        let trimmed = key_base64.trim().trim_matches('"');
        let key = STANDARD.decode(trimmed).map_err(|_| "local key is not base64")?;
        let key: [u8; 32] = key.try_into().map_err(|_| "local key must be 32 bytes")?;
        Ok(Self(XSalsa20Poly1305::new(&key.into())))
    }

    pub fn decrypt(&self, field: &str) -> Result<String, String> {
        let bytes = STANDARD.decode(field).map_err(|_| "field is not base64")?;
        if bytes.len() < HEADER || bytes[0] != VERSION {
            return Err("unsupported field envelope".into());
        }
        let plain = self
            .0
            .decrypt(Nonce::from_slice(&bytes[2..HEADER]), &bytes[HEADER..])
            .map_err(|_| "field decryption failed")?;
        String::from_utf8(plain).map_err(|_| "field is not UTF-8".into())
    }

    #[cfg(test)]
    pub fn encrypt(&self, plain: &str, nonce: [u8; 24]) -> String {
        let boxed = self.0.encrypt(Nonce::from_slice(&nonce), plain.as_bytes()).unwrap();
        let mut out = vec![VERSION, 1];
        out.extend(nonce);
        out.extend(boxed);
        STANDARD.encode(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> FieldCipher {
        FieldCipher::from_local_key(&STANDARD.encode([7u8; 32])).unwrap()
    }

    #[test]
    fn round_trips_a_field() {
        let c = cipher();
        assert_eq!(c.decrypt(&c.encrypt("home lab", [1; 24])).unwrap(), "home lab");
    }

    #[test]
    fn accepts_json_quoted_key() {
        let quoted = format!("\"{}\"", STANDARD.encode([7u8; 32]));
        let c = FieldCipher::from_local_key(&quoted).unwrap();
        assert_eq!(c.decrypt(&cipher().encrypt("x", [2; 24])).unwrap(), "x");
    }

    #[test]
    fn wrong_key_fails() {
        let other = FieldCipher::from_local_key(&STANDARD.encode([8u8; 32])).unwrap();
        assert!(other.decrypt(&cipher().encrypt("secret", [3; 24])).is_err());
    }

    #[test]
    fn rejects_short_keys_and_bad_envelopes() {
        assert!(FieldCipher::from_local_key(&STANDARD.encode([0u8; 16])).is_err());
        assert!(cipher().decrypt(&STANDARD.encode([3u8; 40])).is_err());
    }
}
