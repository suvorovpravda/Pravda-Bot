//! Persistent, strictly opt-in gate for optional public Revolut X order book.
//! This is configuration only, not exchange authentication or order permission.
//! Missing, malformed or unreadable storage must never enable the network call.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ENABLED: &str = "enabled\n";
const DISABLED: &str = "disabled\n";

pub struct RevolutPublicConsent {
    file: PathBuf,
    enabled: Mutex<bool>,
}

impl RevolutPublicConsent {
    pub fn load(auth_path: &Path) -> Self {
        let file = auth_path.with_file_name("revolut_public_consent.cfg");
        let enabled = matches!(std::fs::read_to_string(&file), Ok(content) if content == ENABLED);
        Self {
            file,
            enabled: Mutex::new(enabled),
        }
    }

    pub fn enabled(&self) -> bool {
        *self.enabled.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    /// Only the authenticated main WebView can call this method via IPC.
    /// The preference is never stored in JavaScript or passed to a broker.
    pub fn set(&self, enabled: bool) -> Result<(), &'static str> {
        let mut current = self.enabled.lock().unwrap_or_else(|poison| poison.into_inner());
        if !enabled {
            // Fail closed in memory even if the write below encounters an error.
            *current = false;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file)
            .map_err(|_| "CONSENT_STORAGE_FAILURE")?;
        file.write_all(if enabled { ENABLED } else { DISABLED }.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|_| "CONSENT_STORAGE_FAILURE")?;
        *current = enabled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "pravda-optin-test-{}-{nanos}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn default_is_off_and_enable_is_explicit_and_persistent() {
        let dir = fixture("toggle");
        std::fs::create_dir(&dir).unwrap();
        let auth = dir.join("auth.json");
        let first = RevolutPublicConsent::load(&auth);
        assert!(!first.enabled());
        first.set(true).unwrap();
        assert!(first.enabled());
        assert!(RevolutPublicConsent::load(&auth).enabled());
        first.set(false).unwrap();
        assert!(!first.enabled());
        assert!(!RevolutPublicConsent::load(&auth).enabled());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn malformed_config_does_not_authorize_optional_network_access() {
        let dir = fixture("invalid");
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("revolut_public_consent.cfg");
        for raw in ["ENABLEd\n", "enabled", "true", "enabled\nextra"] {
            std::fs::write(&path, raw).unwrap();
            assert!(!RevolutPublicConsent::load(&dir.join("auth.json")).enabled());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failure_to_persist_cannot_enable_a_feed() {
        let root = fixture("nowrite");
        let consent = RevolutPublicConsent::load(&root.join("missing").join("auth.json"));
        assert!(consent.set(true).is_err());
        assert!(!consent.enabled());
    }
}
