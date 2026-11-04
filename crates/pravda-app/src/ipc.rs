//! Phase 1: typed IPC authentication, shared with the existing HTTP adapter.
//! The current External WebView is NOT granted remote Tauri IPC permissions.
//! The frontend transport switch is a separate, reviewed PR.
use pravda_server::{Auth, AuthError};
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{State, Window};

const MAIN_WINDOW: &str = "main";

/// Only Rust owns this session token. It is never returned to JavaScript.
pub struct IpcAuth {
    authority: Arc<Auth>,
    current_token: Mutex<Option<String>>,
}

impl IpcAuth {
    pub fn new(authority: Arc<Auth>) -> Self {
        Self {
            authority,
            current_token: Mutex::new(None),
        }
    }

    fn token(&self) -> MutexGuard<'_, Option<String>> {
        self.current_token
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn accept_token(&self, token: String) {
        if let Some(previous) = self.token().replace(token) {
            self.authority.logout(&previous);
        }
    }

    fn logout(&self) {
        if let Some(token) = self.token().take() {
            self.authority.logout(&token);
        }
    }
}

impl Drop for IpcAuth {
    fn drop(&mut self) {
        self.logout();
    }
}

fn require_main(label: &str) -> Result<(), &'static str> {
    if label != MAIN_WINDOW {
        return Err("FORBIDDEN_WINDOW");
    }
    Ok(())
}

/// Shared by every privileged IPC entry point; a window capability is not
/// sufficient for business authorization.
fn require_authenticated(session: &IpcAuth, label: &str) -> Result<(), &'static str> {
    require_main(label)?;
    let token = session.token();
    if !token
        .as_deref()
        .is_some_and(|value| session.authority.session_valid(value))
    {
        return Err("UNAUTHORIZED");
    }
    Ok(())
}

fn safe_auth_error(error: AuthError) -> String {
    match error {
        AuthError::WeakPassword => "WEAK_PASSWORD",
        AuthError::Mismatch => "PASSWORD_MISMATCH",
        AuthError::Invalid => "INVALID_CREDENTIALS",
        AuthError::Locked(_) => "RATE_LIMITED",
        AuthError::NotConfigured => "NOT_CONFIGURED",
        AuthError::AlreadyConfigured => "ALREADY_CONFIGURED",
        AuthError::Storage => "STORAGE_FAILURE",
    }
    .to_string()
}

/// Public bootstrap exception: the login screen needs to know whether setup
/// is required before a session exists. No secrets are returned.
#[tauri::command]
pub fn ipc_auth_status(window: Window, session: State<'_, IpcAuth>) -> Result<bool, String> {
    require_main(window.label()).map_err(str::to_string)?;
    Ok(session.authority.is_configured())
}

/// Public bootstrap exception: creates the *existing* PBKDF2 verifier.
#[tauri::command]
pub async fn ipc_auth_setup(
    window: Window,
    session: State<'_, IpcAuth>,
    password: String,
    confirm: String,
) -> Result<(), String> {
    require_main(window.label()).map_err(str::to_string)?;
    if password != confirm {
        return Err("PASSWORD_MISMATCH".into());
    }
    let authority = session.authority.clone();
    let created = tauri::async_runtime::spawn_blocking(move || {
        authority.set_password(&password)?;
        authority.login(&password)
    })
    .await
    .map_err(|_| "STORAGE_FAILURE".to_owned())?
    .map_err(safe_auth_error)?;
    session.accept_token(created);
    Ok(())
}

/// Public bootstrap exception: PBKDF2 runs on a blocking worker, not WebView.
#[tauri::command]
pub async fn ipc_auth_login(
    window: Window,
    session: State<'_, IpcAuth>,
    password: String,
) -> Result<(), String> {
    require_main(window.label()).map_err(str::to_string)?;
    let authority = session.authority.clone();
    let token = tauri::async_runtime::spawn_blocking(move || authority.login(&password))
        .await
        .map_err(|_| "STORAGE_FAILURE".to_owned())?
        .map_err(safe_auth_error)?;
    session.accept_token(token);
    Ok(())
}

/// Protected command: an unsigned-in or expired user cannot revoke sessions.
#[tauri::command]
pub fn ipc_auth_logout(window: Window, session: State<'_, IpcAuth>) -> Result<(), String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    session.logout();
    Ok(())
}

/// First protected IPC contract. No business data is exposed in Phase 1.
/// The read-only market/history IPC commands are added only after the
/// runtime becomes desktop-owned (Phase 2).
#[tauri::command]
pub fn ipc_session_check(window: Window, session: State<'_, IpcAuth>) -> Result<bool, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(true)
}

// Every read command must authorize against the Rust-owned live session
// BEFORE touching snapshots, SQLite, CSV or any other business data.
#[tauri::command]
pub async fn ipc_market_snapshot(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(desktop.market_snapshot().await)
}

#[tauri::command]
pub async fn ipc_history_recent(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    desktop.history_recent().await.map_err(str::to_string)
}

#[tauri::command]
pub async fn ipc_history_query(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    query: pravda_server::DesktopHistoryQuery,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    desktop.history_query(query).await.map_err(str::to_string)
}

#[tauri::command]
pub async fn ipc_history_export_csv(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    query: pravda_server::DesktopHistoryQuery,
) -> Result<String, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    desktop
        .history_export_csv(query)
        .await
        .map_err(str::to_string)
}

#[tauri::command]
pub async fn ipc_paper_alerts_snapshot(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(desktop.alerts_snapshot().await)
}

#[tauri::command]
pub fn ipc_runtime_metrics(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(desktop.metrics_snapshot())
}

// Privacy-preserving status lines. The IPC client sees only code-owned,
// allowlisted summaries; never paths, OS identifiers, SQL rows, credentials,
// raw errors or a guessed VPN/installation status.
fn security_report_rows() -> Vec<String> {
    const CHECKS: [(&str, &str, &str); 8] = [
        ("session", "verified", "Session Rust active; fenetre principale authentifiee"),
        ("sqlite", "unknown", "Integrite et restauration SQLite non testees par cette consultation; utiliser Doctor"),
        ("authenticity", "unknown", "Signature Authenticode et provenance du binaire non verifiees ici"),
        ("capabilities", "unknown", "Audit des permissions IPC et CSP non effectue pendant cette consultation"),
        ("market", "unknown", "Fraicheur et confidentialite des flux non verifiees ici"),
        ("orders", "unknown", "Etat financier et autorisation de trading non audites pendant cette consultation"),
        ("proton", "unknown", "Connexion Proton VPN non verifiee; consulter l'application Proton officielle"),
        ("kill_switch", "unknown", "Kill switch et protection DNS/IP du VPN non verifies"),
    ];
    CHECKS
        .iter()
        .map(|(id, status, details)| format!("{id}|{status}|{details}"))
        .collect()
}

/// No internet probe, process enumeration, shell command or broker request.
/// Must remain main-window-only and behind the Rust-owned live session.
#[tauri::command]
pub async fn ipc_security_report(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<Vec<String>, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    let mut rows = security_report_rows();
    rows[1] = match desktop.security_database_health_code().await {
        "database_ok" => "sqlite|verified|SQLite quick_check OK; restauration de sauvegarde non testee",
        "database_missing" => "sqlite|blocked|Base SQLite absente; consulter Doctor sans reinitialiser",
        "database_invalid_path" => "sqlite|blocked|Chemin de base invalide; consulter Doctor",
        "database_integrity_failed" => "sqlite|blocked|Controle SQLite en echec; sauvegarder avant toute reparation",
        "database_schema_missing" => "sqlite|blocked|Schema du magasin SQLite incomplet; consulter Doctor",
        _ => "sqlite|unknown|Base indisponible ou occupee; aucune reparation automatique",
    }
    .to_string();
    Ok(rows)
}

#[tauri::command]
pub async fn ipc_fx_status(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(desktop.fx_status().await)
}

#[tauri::command]
pub async fn ipc_public_feed(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    source: String,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    if !matches!(source.as_str(), "energy" | "news" | "revolut" | "sources") {
        return Err("INVALID_PUBLIC_SOURCE".to_owned());
    }
    Ok(desktop.public_feed(&source).await)
}

fn safe_article_link(raw: &str) -> Result<String, &'static str> {
    if raw.len() > 1500 || raw.contains('\r') || raw.contains('\n') {
        return Err("INVALID_ARTICLE_LINK");
    }
    let link = reqwest::Url::parse(raw).map_err(|_| "INVALID_ARTICLE_LINK")?;
    if link.scheme() != "https"
        || !link.username().is_empty()
        || link.password().is_some()
        || !matches!(link.port(), None | Some(443))
    {
        return Err("INVALID_ARTICLE_LINK");
    }
    let host = link
        .host_str()
        .ok_or("INVALID_ARTICLE_LINK")?
        .to_ascii_lowercase();
    if !host.contains('.')
        || !host.bytes().any(|x| x.is_ascii_alphabetic())
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.ends_with(".test")
        || host.ends_with(".invalid")
        || host.ends_with(".onion")
    {
        return Err("INVALID_ARTICLE_LINK");
    }
    Ok(link.to_string())
}

/// Opens allowlisted source websites or a user-clicked public HTTPS news URL.
/// Never pass an arbitrary URL from an exchange response, a web page, or the UI.
#[tauri::command]
pub fn ipc_open_source(
    window: Window,
    session: State<'_, IpcAuth>,
    source: String,
    link: Option<String>,
) -> Result<(), String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    let url = match source.as_str() {
        "ecb" => "https://data.ecb.europa.eu/data/datasets/EXR".to_owned(),
        "frankfurter" => "https://frankfurter.dev/".to_owned(),
        "entsog" => "https://transparency.entsog.eu/".to_owned(),
        "gdelt" => "https://www.gdeltproject.org/".to_owned(),
        "aisstream" => "https://aisstream.io/".to_owned(),
        "revolutx" => "https://exchange.revolut.com/".to_owned(),
        "ice" => "https://www.ice.com/products/27996665/Dutch-TTF-Natural-Gas-Futures".to_owned(),
        "github" => "https://github.com/suvorovpravda/Pravda-Bot".to_owned(),
        "protonvpn" => "https://protonvpn.com/support/protonvpn-windows-vpn-application".to_owned(),
        "article" => {
            safe_article_link(link.as_deref().unwrap_or_default()).map_err(str::to_string)?
        }
        _ => return Err("INVALID_SOURCE_LINK".to_owned()),
    };
    #[cfg(windows)]
    {
        use std::ffi::c_void;
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn ShellExecuteW(
                hwnd: *mut c_void,
                action: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                display: i32,
            ) -> *mut c_void;
        }
        let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: ShellExecuteW receives only hardcoded HTTPS URLs, no caller strings.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                open.as_ptr(),
                wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        } as isize;
        if result <= 32 {
            return Err("EXTERNAL_BROWSER_UNAVAILABLE".to_owned());
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("WINDOWS_DESKTOP_REQUIRED".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn new_session() -> (IpcAuth, PathBuf) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pravda-ipc-auth-test-{}-{nonce}.json",
            std::process::id()
        ));
        let authority = Arc::new(Auth::load(&path, 1_000).unwrap());
        (IpcAuth::new(authority), path)
    }

    #[test]
    fn security_report_never_claims_vpn_or_integrity_without_verification() {
        let rows = security_report_rows();
        assert_eq!(rows.len(), 8);
        assert_eq!(rows[0].split('|').nth(1), Some("verified"));
        for row in &rows[1..] {
            assert_eq!(row.split('|').nth(1), Some("unknown"));
        }
        assert!(rows.iter().all(|row| row.split('|').count() == 3));
        assert!(!rows.join(" ").contains("CONNECTED"));
        assert!(!rows.join(" ").contains("API_KEY"));
    }

    #[test]
    fn external_article_urls_refuse_local_network_and_unsafe_schemes() {
        for link in [
            "file:///C:/secret",
            "http://example.org",
            "https://localhost/",
            "https://127.0.0.1/private",
            "https://server.internal/test",
            "https://user:pass@example.org/",
            "javascript:alert(1)",
        ] {
            assert!(safe_article_link(link).is_err(), "{link}");
        }
        assert_eq!(
            safe_article_link("https://www.reuters.com/world/").unwrap(),
            "https://www.reuters.com/world/"
        );
    }
    #[test]
    fn all_protected_commands_reject_unauthenticated_and_other_windows() {
        let (session, path) = new_session();
        assert_eq!(require_authenticated(&session, "main"), Err("UNAUTHORIZED"));
        assert_eq!(
            require_authenticated(&session, "other"),
            Err("FORBIDDEN_WINDOW")
        );
        assert_eq!(require_main("main"), Ok(()));
        assert_eq!(require_main("other"), Err("FORBIDDEN_WINDOW"));

        let password = format!("TestOnlySecret-{}-abcdef", std::process::id());
        session.authority.set_password(&password).unwrap();
        assert_eq!(require_authenticated(&session, "main"), Err("UNAUTHORIZED"));

        let token = session.authority.login(&password).unwrap();
        session.accept_token(token);
        assert_eq!(require_authenticated(&session, "main"), Ok(()));
        assert_eq!(
            require_authenticated(&session, "other"),
            Err("FORBIDDEN_WINDOW")
        );

        session.logout();
        assert_eq!(require_authenticated(&session, "main"), Err("UNAUTHORIZED"));
        assert_eq!(session.token().as_deref(), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn expired_or_revoked_session_fails_closed() {
        let (session, path) = new_session();
        let password = format!("TestOnlySecret-{}-uvwxyz", std::process::id());
        session.authority.set_password(&password).unwrap();
        let token = session.authority.login(&password).unwrap();
        session.accept_token(token.clone());
        session.authority.logout(&token);
        assert_eq!(require_authenticated(&session, "main"), Err("UNAUTHORIZED"));
        assert_eq!(safe_auth_error(AuthError::Invalid), "INVALID_CREDENTIALS");
        let _ = std::fs::remove_file(path);
    }
}
