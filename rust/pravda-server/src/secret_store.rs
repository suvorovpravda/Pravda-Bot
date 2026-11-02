//! Windows-native, user-scoped secret storage for future optional exchange integrations.
//!
//! Intentionally NOT exposed through Tauri IPC or HTTP. This module does not
//! enable private exchange APIs or place orders. PBKDF2 password verification
//! remains separate. No secrets are stored in SQLite or YAML.
//!
//! Windows Credential Manager encrypts credentials for the signed-in Windows
//! account. It is *not* a hardware security module or malware isolation.
//! A compromised process running as the same user can access these secrets.
use std::fmt;
use std::sync::atomic::{compiler_fence, Ordering};

const MAX_SECRET_BYTES: usize = 1024;
const TARGET_PREFIX: &str = "Pravda-Bot:exchange:v1:";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretStoreError {
    InvalidName,
    InvalidSize,
    NotFound,
    UnsupportedPlatform,
    OsFailure(u32),
}

impl fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => f.write_str("invalid secret identifier"),
            Self::InvalidSize => f.write_str("invalid secret size"),
            Self::NotFound => f.write_str("secret not found"),
            Self::UnsupportedPlatform => f.write_str("Windows credential storage unavailable"),
            Self::OsFailure(_) => f.write_str("Windows credential storage failure"),
        }
    }
}
impl std::error::Error for SecretStoreError {}

/// Owns plaintext bytes and clears them on drop. Never implements Clone or
/// includes plaintext in Debug. Callers must also avoid copying to logs/UI.
pub struct SecretValue(Vec<u8>);
impl SecretValue {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue([REDACTED])")
    }
}
impl Drop for SecretValue {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            // Prevent the compiler from discarding the wipe as dead stores.
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        compiler_fence(Ordering::SeqCst);
    }
}

/// Only service-layer Rust can access this trait. Do not add Tauri commands
/// or browser endpoints that accept or return these values.
pub trait SecretStore {
    fn put(&self, identifier: &str, secret: &SecretValue) -> Result<(), SecretStoreError>;
    fn get(&self, identifier: &str) -> Result<SecretValue, SecretStoreError>;
    fn remove(&self, identifier: &str) -> Result<(), SecretStoreError>;
}

pub struct WindowsCredentialStore;

fn scoped_target(identifier: &str) -> Result<String, SecretStoreError> {
    if identifier.is_empty()
        || identifier.len() > 64
        || !identifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':' | b'.'))
    {
        return Err(SecretStoreError::InvalidName);
    }
    Ok(format!("{TARGET_PREFIX}{identifier}"))
}

fn check_secret_size(bytes: &[u8]) -> Result<(), SecretStoreError> {
    if bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
        Err(SecretStoreError::InvalidSize)
    } else {
        Ok(())
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;
    const ERROR_NOT_FOUND: u32 = 1168;

    #[repr(C)]
    #[derive(Default)]
    #[allow(non_snake_case)]
    struct FileTime {
        dwLowDateTime: u32,
        dwHighDateTime: u32,
    }

    #[repr(C)]
    #[allow(non_snake_case)]
    struct CredentialW {
        Flags: u32,
        Type: u32,
        TargetName: *mut u16,
        Comment: *mut u16,
        LastWritten: FileTime,
        CredentialBlobSize: u32,
        CredentialBlob: *mut u8,
        Persist: u32,
        AttributeCount: u32,
        Attributes: *mut c_void,
        TargetAlias: *mut u16,
        UserName: *mut u16,
    }

    #[link(name = "Advapi32")]
    unsafe extern "system" {
        fn CredWriteW(credential: *const CredentialW, flags: u32) -> i32;
        fn CredReadW(
            target: *const u16,
            credential_type: u32,
            flags: u32,
            credential: *mut *mut CredentialW,
        ) -> i32;
        fn CredDeleteW(target: *const u16, credential_type: u32, flags: u32) -> i32;
        fn CredFree(credential: *mut c_void);
    }

    fn wide(input: &str) -> Vec<u16> {
        input.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn failure() -> SecretStoreError {
        SecretStoreError::OsFailure(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32,
        )
    }

    struct OwnedCredential(*mut CredentialW);
    impl Drop for OwnedCredential {
        fn drop(&mut self) {
            if self.0.is_null() {
                return;
            }
            // SAFETY: CredReadW allocates this object and its blob; CredFree is
            // the matching deallocator. Wipe the exposed secret before freeing.
            unsafe {
                let record = &mut *self.0;
                if !record.CredentialBlob.is_null() && record.CredentialBlobSize <= 2560 {
                    for index in 0..record.CredentialBlobSize as usize {
                        std::ptr::write_volatile(record.CredentialBlob.add(index), 0);
                    }
                }
                compiler_fence(Ordering::SeqCst);
                CredFree(self.0.cast());
            }
        }
    }

    impl SecretStore for WindowsCredentialStore {
        fn put(&self, identifier: &str, secret: &SecretValue) -> Result<(), SecretStoreError> {
            let target = scoped_target(identifier)?;
            check_secret_size(secret.expose())?;
            let mut target = wide(&target);
            let mut username = wide("Pravda-Bot");
            let record = CredentialW {
                Flags: 0,
                Type: CRED_TYPE_GENERIC,
                TargetName: target.as_mut_ptr(),
                Comment: null_mut(),
                LastWritten: FileTime::default(),
                CredentialBlobSize: secret.expose().len() as u32,
                CredentialBlob: secret.expose().as_ptr() as *mut u8,
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: null_mut(),
                TargetAlias: null_mut(),
                UserName: username.as_mut_ptr(),
            };
            // SAFETY: all pointers remain live for the call and the sizes were
            // checked above. CredWriteW copies the secret into the OS vault.
            if unsafe { CredWriteW(&record, 0) } == 0 {
                return Err(failure());
            }
            Ok(())
        }

        fn get(&self, identifier: &str) -> Result<SecretValue, SecretStoreError> {
            let target = wide(&scoped_target(identifier)?);
            let mut raw = null_mut();
            // SAFETY: target is UTF-16 NUL terminated; raw is an output pointer.
            if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut raw) } == 0 {
                let err = failure();
                return match err {
                    SecretStoreError::OsFailure(ERROR_NOT_FOUND) => Err(SecretStoreError::NotFound),
                    other => Err(other),
                };
            }
            if raw.is_null() {
                return Err(SecretStoreError::OsFailure(0));
            }
            let record = OwnedCredential(raw);
            // SAFETY: CredReadW returned a valid structure owned by record.
            let blob = unsafe { &*record.0 };
            if blob.CredentialBlobSize == 0
                || blob.CredentialBlobSize > MAX_SECRET_BYTES as u32
                || blob.CredentialBlob.is_null()
            {
                return Err(SecretStoreError::InvalidSize);
            }
            // SAFETY: CredReadW owns at least CredentialBlobSize readable bytes;
            // the returned copy is zeroized by SecretValue::drop.
            let plaintext = unsafe {
                std::slice::from_raw_parts(blob.CredentialBlob, blob.CredentialBlobSize as usize)
                    .to_vec()
            };
            Ok(SecretValue::new(plaintext))
        }

        fn remove(&self, identifier: &str) -> Result<(), SecretStoreError> {
            let target = wide(&scoped_target(identifier)?);
            // SAFETY: target is a valid NUL-terminated UTF-16 string.
            if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
                let err = failure();
                return match err {
                    SecretStoreError::OsFailure(ERROR_NOT_FOUND) => Err(SecretStoreError::NotFound),
                    other => Err(other),
                };
            }
            Ok(())
        }
    }
}

#[cfg(not(windows))]
impl SecretStore for WindowsCredentialStore {
    fn put(&self, identifier: &str, secret: &SecretValue) -> Result<(), SecretStoreError> {
        scoped_target(identifier)?;
        check_secret_size(secret.expose())?;
        Err(SecretStoreError::UnsupportedPlatform)
    }
    fn get(&self, identifier: &str) -> Result<SecretValue, SecretStoreError> {
        scoped_target(identifier)?;
        Err(SecretStoreError::UnsupportedPlatform)
    }
    fn remove(&self, identifier: &str) -> Result<(), SecretStoreError> {
        scoped_target(identifier)?;
        Err(SecretStoreError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_cannot_escape_the_application_scope() {
        for bad in ["", "../other", "name/secret", "space key", "bad\0key", "é", "a@b"] {
            assert_eq!(scoped_target(bad), Err(SecretStoreError::InvalidName));
        }
        assert_eq!(
            scoped_target("revolut_x:read_only").unwrap(),
            "Pravda-Bot:exchange:v1:revolut_x:read_only"
        );
    }

    #[test]
    fn secret_value_has_no_plaintext_debug_and_size_is_bounded() {
        let secret = SecretValue::new(b"not-a-real-key".to_vec());
        assert_eq!(format!("{secret:?}"), "SecretValue([REDACTED])");
        assert!(check_secret_size(secret.expose()).is_ok());
        assert_eq!(check_secret_size(&[]), Err(SecretStoreError::InvalidSize));
        assert_eq!(check_secret_size(&vec![1; 1025]), Err(SecretStoreError::InvalidSize));
    }

    #[cfg(not(windows))]
    #[test]
    fn no_plaintext_file_fallback_on_unsupported_os() {
        let store = WindowsCredentialStore;
        let secret = SecretValue::new(b"example".to_vec());
        assert_eq!(
            store.put("demo", &secret),
            Err(SecretStoreError::UnsupportedPlatform)
        );
    }

    #[cfg(windows)]
    #[test]
    fn credential_manager_roundtrip_is_user_scoped_and_deleted() {
        let store = WindowsCredentialStore;
        let unique = format!(
            "test:pid{}:n{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let _ = store.remove(&unique); // Fresh scoped name, ignore missing.
        assert!(matches!(store.get(&unique), Err(SecretStoreError::NotFound)));
        let original = SecretValue::new(b"test-only-secret-123".to_vec());
        store.put(&unique, &original).expect("Credential Manager write");
        let loaded = store.get(&unique).expect("Credential Manager read");
        assert_eq!(loaded.expose(), original.expose());
        drop(loaded);
        store.remove(&unique).expect("Credential Manager delete");
        assert!(matches!(store.get(&unique), Err(SecretStoreError::NotFound)));
    }
}
