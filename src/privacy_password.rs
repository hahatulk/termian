use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub const MAX_BYTES: usize = 4096;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyPassword {
    salt: [u8; 16],
    verifier: [u8; 32],
}

impl std::fmt::Debug for PrivacyPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivacyPassword")
    }
}

fn derive(password: &str, salt: &[u8; 16]) -> Option<Zeroizing<[u8; 32]>> {
    if password.is_empty() || password.len() > MAX_BYTES {
        return None;
    }
    let params = scrypt::Params::new(15, 8, 1, 32).ok()?;
    let mut output = Zeroizing::new([0; 32]);
    scrypt::scrypt(password.as_bytes(), salt, &params, output.as_mut()).ok()?;
    Some(output)
}

impl PrivacyPassword {
    pub fn new(password: &str) -> Option<Self> {
        let mut salt = [0; 16];
        getrandom::getrandom(&mut salt).ok()?;
        let verifier = *derive(password, &salt)?;
        Some(Self { salt, verifier })
    }

    pub fn verify(&self, password: &str) -> bool {
        derive(password, &self.salt).is_some_and(|trial| bool::from(trial.ct_eq(&self.verifier)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_verifier_accepts_only_the_matching_password() {
        let password = "test パスワード";
        let verifier = PrivacyPassword::new(password).unwrap();
        let saved = serde_json::to_string(&verifier).unwrap();
        assert!(!saved.contains(password));
        let restored: PrivacyPassword = serde_json::from_str(&saved).unwrap();
        assert!(restored.verify(password));
        assert!(!restored.verify("test パスワード "));
        assert!(!restored.verify(""));
        assert_ne!(PrivacyPassword::new(password).unwrap(), verifier);
    }
}
