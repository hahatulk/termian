use zeroize::Zeroizing;

use super::{KeyProtection, MAX_WRAPPED_KEY};

type Key = Zeroizing<[u8; 32]>;

extern "C" {
    fn termian_vault_wrap(
        key: *const u8,
        output: *mut u8,
        capacity: usize,
        length: *mut usize,
    ) -> i32;
    fn termian_vault_unwrap(
        input: *const u8,
        length: usize,
        key: *mut u8,
        allow_interaction: i32,
    ) -> i32;
}

pub(super) struct SecureEnclave;

impl KeyProtection for SecureEnclave {
    fn wrap(&self, key: &[u8; 32]) -> Result<Vec<u8>, String> {
        let mut wrapped = vec![0; MAX_WRAPPED_KEY];
        let mut length = 0;
        match unsafe {
            termian_vault_wrap(
                key.as_ptr(),
                wrapped.as_mut_ptr(),
                wrapped.len(),
                &mut length,
            )
        } {
            0 if (1..=wrapped.len()).contains(&length) => {
                wrapped.truncate(length);
                Ok(wrapped)
            }
            1 => Err("This Mac cannot use the Secure Enclave. The vault was not changed.".into()),
            _ => Err("macOS could not protect the vault key. The vault was not changed.".into()),
        }
    }

    fn unwrap(&self, wrapped: &[u8]) -> Result<Key, String> {
        self.unwrap_with_interaction(wrapped, true)
    }
}

impl SecureEnclave {
    fn unwrap_with_interaction(&self, wrapped: &[u8], allow: bool) -> Result<Key, String> {
        let mut key = Zeroizing::new([0; 32]);
        match unsafe { termian_vault_unwrap(wrapped.as_ptr(), wrapped.len(), key.as_mut_ptr(), i32::from(allow)) } {
            0 => Ok(key),
            1 => Err("This Mac cannot use the Secure Enclave.".into()),
            3 => Err("The wrapped vault key is damaged.".into()),
            4 => Err("Authentication was canceled.".into()),
            5 => Err("macOS requires user interaction to open the vault.".into()),
            _ => Err("macOS could not open the vault key. Use the same Mac and account that protected it.".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a Mac with Secure Enclave in an unlocked login session"]
    fn a_secure_enclave_key_cannot_be_unwrapped_without_user_interaction() {
        let wrapped = SecureEnclave.wrap(&[17; 32]).unwrap();
        let error = SecureEnclave
            .unwrap_with_interaction(&wrapped, false)
            .err()
            .unwrap();
        assert_eq!(error, "macOS requires user interaction to open the vault.");
    }
}
