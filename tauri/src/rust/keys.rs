//! SSH key generation for libtermius' `generateSshKey` (Keychain → "Generate
//! key"). Keys are produced in OpenSSH format, optionally passphrase-encrypted.

use russh::keys::ssh_key::{private::RsaKeypair, EcdsaCurve, LineEnding};
use russh::keys::{Algorithm, PrivateKey};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyOptions {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    bits: Option<usize>,
    #[serde(default)]
    curve: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    comment: String,
}

fn generate(options: KeyOptions) -> Result<Value, String> {
    let mut rng = rand10::rng();
    let mut key = match options.kind.as_str() {
        "ed" => PrivateKey::random(&mut rng, Algorithm::Ed25519),
        "ec" => {
            let curve = match options.curve.as_str() {
                "nistp384" => EcdsaCurve::NistP384,
                "nistp521" => EcdsaCurve::NistP521,
                _ => EcdsaCurve::NistP256,
            };
            PrivateKey::random(&mut rng, Algorithm::Ecdsa { curve })
        }
        "rsa" => {
            let bits = options.bits.unwrap_or(4096);
            if !(2048..=8192).contains(&bits) {
                return Err("RSA keys must be 2048–8192 bits".into());
            }
            RsaKeypair::random(&mut rng, bits).map(PrivateKey::from)
        }
        other => return Err(format!("Unsupported key type: {other}")),
    }
    .map_err(|error| error.to_string())?;
    key.set_comment(options.comment.as_str());
    let public_key = key.public_key().to_openssh().map_err(|error| error.to_string())?;
    if !options.password.is_empty() {
        key = key.encrypt(&mut rng, options.password.as_bytes()).map_err(|error| error.to_string())?;
    }
    let private_key = key.to_openssh(LineEnding::LF).map_err(|error| error.to_string())?;
    Ok(json!({"privateKey": private_key.as_str(), "publicKey": public_key}))
}

#[tauri::command]
pub async fn ssh_generate_key(options: KeyOptions) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || generate(options))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_encrypted_ed25519_keys() {
        let key = generate(KeyOptions {
            kind: "ed".into(),
            bits: None,
            curve: String::new(),
            password: "secret".into(),
            comment: "test".into(),
        })
        .expect("generate");
        let private = key["privateKey"].as_str().expect("private");
        assert!(private.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
        assert!(key["publicKey"].as_str().expect("public").starts_with("ssh-ed25519 "));
        assert!(russh::keys::decode_secret_key(private, Some("secret")).is_ok());
        assert!(russh::keys::decode_secret_key(private, Some("wrong")).is_err());
    }
}
