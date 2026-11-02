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

#[tauri::command]
pub fn ipc_fx_status(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(desktop.fx_status())
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
