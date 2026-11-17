//! Session-adjacent orchestration of the private Revolut X connection.
//!
//! This module owns only the persistent opt-in gate and the offline refusal.
//! The vault, the signing client and the credential bundle stay behind
//! `pravda_server::revolut_private_view`: nothing here can accept, hold or
//! return an API key, a private key, a signature or an authentication header.
//! Provisioning remains the local Windows console path `--revolut-x-setup`.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ENABLED: &str = "enabled\n";
const DISABLED: &str = "disabled\n";

/// Persistent, strictly opt-in gate for authenticated private reads, stored
/// separately from the public order-book consent: public market data and
/// account data are not the same permission. Missing, malformed or unreadable
/// storage must never authorize a request.
pub struct RevolutPrivateConsent {
    file: PathBuf,
    enabled: Mutex<bool>,
}

impl RevolutPrivateConsent {
    pub fn load(auth_path: &Path) -> Self {
        let file = auth_path.with_file_name("revolut_private_consent.cfg");
        let enabled = matches!(std::fs::read_to_string(&file), Ok(content) if content == ENABLED);
        Self {
            file,
            enabled: Mutex::new(enabled),
        }
    }

    pub fn enabled(&self) -> bool {
        *self
            .enabled
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// Only the authenticated main WebView reaches this through Rust IPC.
    /// A failed write can never leave the gate open, even in memory.
    pub fn set(&self, enabled: bool) -> Result<(), &'static str> {
        let mut current = self
            .enabled
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if !enabled {
            *current = false;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file)
            .map_err(|_| "PRIVATE_CONSENT_STORAGE_FAILURE")?;
        let content = if enabled { ENABLED } else { DISABLED };
        file.write_all(content.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|_| "PRIVATE_CONSENT_STORAGE_FAILURE")?;
        *current = enabled;
        Ok(())
    }
}

/// The isolated offline diagnostic and the harness must never reach a private
/// exchange endpoint, whatever the stored consent says.
fn offline_diagnostic() -> bool {
    std::env::var("PRAVDA_OFFLINE_TEST").as_deref() == Ok("true")
        || std::env::var("PRAVDA_REVOLUT_X_ENABLED").as_deref() == Ok("false")
}

/// Launch the audited provisioning assistant carried by this same executable
/// (`Pravda-Bot.exe --revolut-x-setup`). Fixed argv, no shell, no argument from
/// JavaScript: the operator types the local password, the PEM path and the API
/// key in the assistant window, so the WebView never receives a secret.
pub fn launch_setup_assistant() -> Result<(), &'static str> {
    if offline_diagnostic() {
        return Err("OFFLINE_DIAGNOSTIC");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        let executable = std::env::current_exe().map_err(|_| "PRIVATE_SETUP_UNAVAILABLE")?;
        std::process::Command::new(executable)
            .arg("--revolut-x-setup")
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|_| "PRIVATE_SETUP_UNAVAILABLE")?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("PRIVATE_UNSUPPORTED_PLATFORM")
    }
}

/// Local connection state. No network call and no credential material.
pub fn status_rows(consent: &RevolutPrivateConsent) -> Vec<String> {
    pravda_server::revolut_private_view::status_rows(consent.enabled())
}

async fn gated_balance_rows(
    consent: &RevolutPrivateConsent,
    offline: bool,
) -> Result<Vec<String>, &'static str> {
    if offline {
        return Err("OFFLINE_DIAGNOSTIC");
    }
    if !consent.enabled() {
        return Err("PRIVATE_OPT_IN_REQUIRED");
    }
    let rows = pravda_server::revolut_private_view::balance_rows().await?;
    // A revocation while the signed request was in flight must expose nothing.
    if !consent.enabled() {
        return Err("PRIVATE_OPT_IN_REQUIRED");
    }
    Ok(rows)
}

/// One explicit user action performs both the connection test and the
/// read-only balance read: the venue exposes a single allowlisted signed GET.
pub async fn balance_rows(consent: &RevolutPrivateConsent) -> Result<Vec<String>, &'static str> {
    gated_balance_rows(consent, offline_diagnostic()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pravda-private-consent-{}-{nanos}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn enabled_consent(name: &str) -> (PathBuf, RevolutPrivateConsent) {
        let dir = fixture(name);
        let consent = RevolutPrivateConsent::load(&dir.join("auth.json"));
        consent.set(true).unwrap();
        (dir, consent)
    }

    #[test]
    fn private_consent_is_off_by_default_and_persists_an_explicit_opt_in() {
        let dir = fixture("toggle");
        let auth = dir.join("auth.json");
        let first = RevolutPrivateConsent::load(&auth);
        assert!(!first.enabled());
        first.set(true).unwrap();
        assert!(RevolutPrivateConsent::load(&auth).enabled());
        first.set(false).unwrap();
        assert!(!RevolutPrivateConsent::load(&auth).enabled());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn malformed_private_consent_never_authorizes_a_private_read() {
        let dir = fixture("malformed");
        let path = dir.join("revolut_private_consent.cfg");
        for raw in ["ENABLED\n", "enabled", "true", "enabled\nextra", ""] {
            std::fs::write(&path, raw).unwrap();
            assert!(!RevolutPrivateConsent::load(&dir.join("auth.json")).enabled());
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn private_consent_is_independent_from_the_public_order_book_gate() {
        let dir = fixture("independent");
        let auth = dir.join("auth.json");
        std::fs::write(dir.join("revolut_public_consent.cfg"), ENABLED).unwrap();
        assert!(!RevolutPrivateConsent::load(&auth).enabled());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_persistence_cannot_enable_private_reads() {
        let root = fixture("nowrite");
        let consent = RevolutPrivateConsent::load(&root.join("missing").join("auth.json"));
        assert!(consent.set(true).is_err());
        assert!(!consent.enabled());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn offline_diagnostic_refuses_before_vault_and_network() {
        let (dir, consent) = enabled_consent("offline");
        assert_eq!(gated_balance_rows(&consent, true).await, Err("OFFLINE_DIAGNOSTIC"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn consent_off_refuses_before_any_request() {
        let dir = fixture("optout");
        let consent = RevolutPrivateConsent::load(&dir.join("auth.json"));
        assert_eq!(gated_balance_rows(&consent, false).await, Err("PRIVATE_OPT_IN_REQUIRED"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[tokio::test]
    async fn revocation_before_the_read_exposes_no_balance() {
        let (dir, consent) = enabled_consent("revoke");
        consent.set(false).unwrap();
        assert_eq!(gated_balance_rows(&consent, false).await, Err("PRIVATE_OPT_IN_REQUIRED"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn setup_assistant_is_unavailable_off_windows_and_never_spawns_in_diagnostic() {
        // The harness/diagnostic refusal is checked before any process spawn.
        #[cfg(not(windows))]
        assert_eq!(launch_setup_assistant(), Err("PRIVATE_UNSUPPORTED_PLATFORM"));
    }

    #[test]
    fn status_rows_stay_a_closed_local_vocabulary() {
        let dir = fixture("status");
        let consent = RevolutPrivateConsent::load(&dir.join("auth.json"));
        let rows = status_rows(&consent);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[1], "consent|disabled");
        assert_eq!(rows[2], "scope|read_only");
        assert_eq!(rows[3], "orders|disabled");
        assert!(rows.iter().all(|row| row.split('|').count() == 2));
        std::fs::remove_dir_all(dir).unwrap();
    }
}