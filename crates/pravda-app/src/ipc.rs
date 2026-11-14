//! Phase 1: typed IPC authentication, shared with the existing HTTP adapter.
//! The current External WebView is NOT granted remote Tauri IPC permissions.
//! The frontend transport switch is a separate, reviewed PR.
use crate::consent::RevolutPublicConsent;
use crate::paper::PaperSandbox;
use pravda_server::{paper_control::PaperControl, Auth, AuthError};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{State, Window};

const MAIN_WINDOW: &str = "main";

const MAX_SECURITY_EVENTS: usize = 64;

/// Codes are a closed enum: callers cannot accidentally log a password,
/// machine identifier, token, path or arbitrary raw error string.
#[derive(Clone, Copy)]
enum SecurityEventCode {
    SetupAccepted,
    SetupRejected,
    LoginAccepted,
    LoginRejected,
    LoginLocked,
    SessionEnded,
    StorageFailure,
}
impl SecurityEventCode {
    const fn label(self) -> &'static str {
        match self {
            Self::SetupAccepted => "setup_accepted",
            Self::SetupRejected => "setup_rejected",
            Self::LoginAccepted => "login_accepted",
            Self::LoginRejected => "login_rejected",
            Self::LoginLocked => "login_locked",
            Self::SessionEnded => "session_ended",
            Self::StorageFailure => "storage_failure",
        }
    }
}

/// Volatile, bounded, in-process security feedback. Does not touch SQLite or
/// persist sensitive authentication activity to disk. It resets on restart.
#[derive(Default)]
pub struct SecurityEvents {
    events: Mutex<VecDeque<(u64, &'static str)>>,
}
impl SecurityEvents {
    fn record(&self, code: SecurityEventCode) {
        let unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut entries = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if entries.len() == MAX_SECURITY_EVENTS {
            entries.pop_front();
        }
        entries.push_back((unix_seconds, code.label()));
    }
    fn snapshot(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .rev()
            .map(|(seconds, label)| format!("{seconds}|{label}"))
            .collect()
    }
}

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

    fn accept_token(&self, token: String, paper: &PaperSandbox) {
        // Match paper_session_guard and logout: session lock, then portfolio lock.
        // The next session is not visible until its portfolio is reset.
        let mut current = self.token();
        if let Some(previous) = current.take() {
            self.authority.logout(&previous);
        }
        paper.reset();
        *current = Some(token);
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

/// Capture the exact authenticated Rust session *before* starting an async
/// read. The mutex guard must never cross an await.
fn authorized_read_token(session: &IpcAuth, label: &str) -> Result<String, &'static str> {
    require_main(label)?;
    let token = session.token();
    let token = token.as_deref().ok_or("UNAUTHORIZED")?;
    if !session.authority.session_valid(token) {
        return Err("UNAUTHORIZED");
    }
    Ok(token.to_owned())
}

/// Reject work completed after logout, expiry or replacement by a new login.
/// Checking only "some session is active" would incorrectly authorize results
/// from a previous login after a new session is installed.
fn finish_authorized_read<T>(
    session: &IpcAuth,
    issued_to: &str,
    completed: Result<T, String>,
) -> Result<T, String> {
    let current = session.token();
    if current.as_deref() != Some(issued_to) || !session.authority.session_valid(issued_to) {
        return Err("UNAUTHORIZED".to_owned());
    }
    completed
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

/// Public bootstrap exception: creates the existing PBKDF2 verifier.
/// Only closed event codes are recorded; never password material.
#[tauri::command]
pub async fn ipc_auth_setup(
    window: Window,
    session: State<'_, IpcAuth>,
    events: State<'_, SecurityEvents>,
    paper: State<'_, PaperSandbox>,
    password: String,
    confirm: String,
) -> Result<(), String> {
    require_main(window.label()).map_err(str::to_string)?;
    if password != confirm {
        events.record(SecurityEventCode::SetupRejected);
        return Err("PASSWORD_MISMATCH".into());
    }
    let authority = session.authority.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        authority.set_password(&password)?;
        authority.login(&password)
    })
    .await;
    match result {
        Ok(Ok(token)) => {
            session.accept_token(token, &paper);
            events.record(SecurityEventCode::SetupAccepted);
            Ok(())
        }
        Ok(Err(error)) => {
            events.record(SecurityEventCode::SetupRejected);
            Err(safe_auth_error(error))
        }
        Err(_) => {
            events.record(SecurityEventCode::StorageFailure);
            Err("STORAGE_FAILURE".into())
        }
    }
}

/// Public bootstrap exception: PBKDF2 runs on a blocking worker, not WebView.
#[tauri::command]
pub async fn ipc_auth_login(
    window: Window,
    session: State<'_, IpcAuth>,
    events: State<'_, SecurityEvents>,
    paper: State<'_, PaperSandbox>,
    password: String,
) -> Result<(), String> {
    require_main(window.label()).map_err(str::to_string)?;
    let authority = session.authority.clone();
    let result = tauri::async_runtime::spawn_blocking(move || authority.login(&password)).await;
    match result {
        Ok(Ok(token)) => {
            session.accept_token(token, &paper);
            events.record(SecurityEventCode::LoginAccepted);
            Ok(())
        }
        Ok(Err(error)) => {
            let code = if matches!(&error, AuthError::Locked(_)) {
                SecurityEventCode::LoginLocked
            } else {
                SecurityEventCode::LoginRejected
            };
            events.record(code);
            Err(safe_auth_error(error))
        }
        Err(_) => {
            events.record(SecurityEventCode::StorageFailure);
            Err("STORAGE_FAILURE".into())
        }
    }
}

/// Protected command: an unsigned-in or expired user cannot revoke sessions.
#[tauri::command]
pub fn ipc_auth_logout(
    window: Window,
    session: State<'_, IpcAuth>,
    events: State<'_, SecurityEvents>,
    paper: State<'_, PaperSandbox>,
) -> Result<(), String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    // Serialize logout with every synchronous paper portfolio mutation.
    let mut current = session.token();
    if let Some(token) = current.take() {
        session.authority.logout(&token);
    }
    paper.reset();
    events.record(SecurityEventCode::SessionEnded);
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
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    finish_authorized_read(&session, &issued_to, Ok(desktop.market_snapshot().await))
}

/// Authenticated, read-only native PAPER research preflight. This is not a
/// trading signal: an observed exchange quote is never portfolio attestation.
#[tauri::command]
pub async fn ipc_paper_research_preflight(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<Vec<String>, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    let snapshot = desktop.market_snapshot().await;
    // Revalidate after the await; a replaced or expired session cannot read results.
    finish_authorized_read(
        &session,
        &issued_to,
        Ok(paper_research_preflight_rows(&snapshot.0)),
    )
}

/// Fixed, non-executable diagnostic; no JS-provided flags, market claims,
/// portfolio balances, or model output are accepted as trusted evidence.
fn paper_research_preflight_rows(snapshot: &serde_json::Value) -> Vec<String> {
    use pravda_core::agent_contract::{PaperSymbol, PaperVenue};
    use pravda_core::agent_pipeline::evidence_from_native_market_snapshot;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(0))
        .unwrap_or(0);
    let observation = evidence_from_native_market_snapshot(
        snapshot,
        PaperSymbol::BtcUsdt,
        PaperVenue::Binance,
        now_ms,
        5_000,
        100,
    );
    let observation_code = match observation {
        Ok(evidence) if !evidence.provenance_verified && !evidence.portfolio_verified => {
            "MDA_OBSERVATION_ONLY"
        }
        Ok(_) => "MDA_ATTESTATION_UNEXPECTED",
        Err(reason) => reason.code(),
    };
    vec![
        "mode|PAPER_RESEARCH_ONLY".to_owned(),
        format!("observation|{observation_code}"),
        "provenance|not_attested".to_owned(),
        "portfolio|not_attested".to_owned(),
        "reg|not_executed".to_owned(),
        "order|disabled".to_owned(),
        "live|disabled".to_owned(),
    ]
}

/// Read-only native PAPER research probe. Authorization is captured before
/// the async feed read and rechecked after it, including login replacement.
/// Neither the request nor this response can modify the synthetic portfolio.
#[tauri::command]
pub async fn ipc_paper_pipeline_probe(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    symbol: String,
    venue: String,
) -> Result<Vec<String>, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    let snapshot = desktop.market_snapshot().await;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "PAPER_CLOCK_UNAVAILABLE".to_owned())?
        .as_millis();
    let now_ms = u64::try_from(now_ms).map_err(|_| "PAPER_CLOCK_UNAVAILABLE".to_owned())?;
    let result =
        crate::paper_pipeline::native_observation_probe(&snapshot.0, &symbol, &venue, now_ms)
            .map_err(str::to_string);
    finish_authorized_read(&session, &issued_to, result)
}

#[tauri::command]
pub async fn ipc_history_recent(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    finish_authorized_read(
        &session,
        &issued_to,
        desktop.history_recent().await.map_err(str::to_string),
    )
}

#[tauri::command]
pub async fn ipc_history_query(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    query: pravda_server::DesktopHistoryQuery,
) -> Result<pravda_server::DesktopPayload, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    finish_authorized_read(
        &session,
        &issued_to,
        desktop.history_query(query).await.map_err(str::to_string),
    )
}

#[tauri::command]
pub async fn ipc_history_export_csv(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    query: pravda_server::DesktopHistoryQuery,
) -> Result<String, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    finish_authorized_read(
        &session,
        &issued_to,
        desktop
            .history_export_csv(query)
            .await
            .map_err(str::to_string),
    )
}

#[tauri::command]
pub async fn ipc_paper_alerts_snapshot(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    finish_authorized_read(&session, &issued_to, Ok(desktop.alerts_snapshot().await))
}

/// Every paper action holds the Rust session lock during the synchronous
/// mutation. Logout cannot interleave with a simulated trade.
fn paper_session_guard<'a>(
    session: &'a IpcAuth,
    label: &str,
) -> Result<MutexGuard<'a, Option<String>>, String> {
    require_main(label).map_err(str::to_string)?;
    let token = session.token();
    if !token
        .as_deref()
        .is_some_and(|current| session.authority.session_valid(current))
    {
        return Err("UNAUTHORIZED".to_owned());
    }
    Ok(token)
}

/// Only synthetic fixed-price quotes. No exchange, secret or network access.
#[tauri::command]
pub fn ipc_paper_snapshot(
    window: Window,
    session: State<'_, IpcAuth>,
    paper: State<'_, PaperSandbox>,
) -> Result<Vec<String>, String> {
    let _session_guard = paper_session_guard(&session, window.label())?;
    Ok(paper.snapshot())
}

#[tauri::command]
pub fn ipc_paper_trade(
    window: Window,
    session: State<'_, IpcAuth>,
    paper: State<'_, PaperSandbox>,
    control: State<'_, PaperControl>,
    symbol: String,
    side: String,
    quantity_milli: u32,
) -> Result<Vec<String>, String> {
    let _session_guard = paper_session_guard(&session, window.label())?;
    control
        .with_paper_trade(|| paper.trade(&symbol, &side, quantity_milli))
        .map_err(str::to_string)
}

#[tauri::command]
pub fn ipc_paper_reset(
    window: Window,
    session: State<'_, IpcAuth>,
    paper: State<'_, PaperSandbox>,
) -> Result<Vec<String>, String> {
    let _session_guard = paper_session_guard(&session, window.label())?;
    Ok(paper.reset())
}

/// Available even when signed out: stopping is safe and never places orders.
#[tauri::command]
pub fn ipc_paper_emergency_stop(
    window: Window,
    control: State<'_, PaperControl>,
) -> Result<(), String> {
    require_main(window.label()).map_err(str::to_string)?;
    control
        .emergency_stop()
        .map_err(|error| error.code().to_owned())
}

/// Authenticated read returns only state and bounded event count.
#[tauri::command]
pub fn ipc_paper_control_status(
    window: Window,
    session: State<'_, IpcAuth>,
    control: State<'_, PaperControl>,
) -> Result<Vec<String>, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    let status = control.status().map_err(|error| error.code().to_owned())?;
    Ok(vec![
        format!("stopped|{}", status.stopped),
        format!("audit_records|{}", status.audit_records),
        "mode|PAPER_ONLY".to_owned(),
    ])
}

/// A fresh password is checked off the Tauri event thread. Once it succeeds,
/// the original authenticated session is rechecked under the native session
/// mutex, then the journal transition and paper reset run together.
#[tauri::command]
pub async fn ipc_paper_rearm(
    window: Window,
    session: State<'_, IpcAuth>,
    paper: State<'_, PaperSandbox>,
    control: State<'_, PaperControl>,
    password: String,
) -> Result<Vec<String>, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    let authority = session.authority.clone();
    let verified = tauri::async_runtime::spawn_blocking(move || -> Result<(), AuthError> {
        // Freshly authenticate without replacing the active desktop session.
        // The one-time token is immediately revoked, even though no token
        // reaches the untrusted WebView.
        let token = authority.login(&password)?;
        authority.logout(&token);
        Ok(())
    })
    .await
    .map_err(|_| "STORAGE_FAILURE".to_owned())?;
    verified.map_err(safe_auth_error)?;

    let session_guard = paper_session_guard(&session, window.label())?;
    if session_guard.as_deref() != Some(issued_to.as_str()) {
        return Err("UNAUTHORIZED".to_owned());
    }
    // Resetting before the transition used to erase a portfolio on rejected
    // rearms (already armed, cooldown or corrupt audit). The closure executes
    // only after a successful, verified, durable REARM_PAPER append.
    control
        .rearm_with_reset(|| paper.reset())
        .map_err(|error| error.code().to_owned())?;
    Ok(vec![
        "mode|PAPER_ONLY".to_owned(),
        "stopped|false".to_owned(),
    ])
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

// Local-only updater readiness: this release does not initialize an updater
// plugin or ship a public verification key. Never infer auto-update support
// from the existence of a signed packaging workflow. No network or filesystem
// access happens in this command, and no raw config/secrets are exposed.
fn update_status_rows() -> Vec<String> {
    vec![
        format!("version|{}", env!("CARGO_PKG_VERSION")),
        "channel|not_configured".to_owned(),
        "signature|not_verified".to_owned(),
        "automatic|disabled".to_owned(),
        "last_check|never".to_owned(),
    ]
}

#[tauri::command]
pub fn ipc_update_status(
    window: Window,
    session: State<'_, IpcAuth>,
) -> Result<Vec<String>, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(update_status_rows())
}

// Privacy-preserving status lines. The IPC client sees only code-owned,
// allowlisted summaries; never paths, OS identifiers, SQL rows, credentials,
// raw errors or a guessed VPN/installation status.
fn security_report_rows() -> Vec<String> {
    const CHECKS: [(&str, &str, &str); 8] = [
        (
            "session",
            "verified",
            "Session Rust active; fenetre principale authentifiee",
        ),
        (
            "sqlite",
            "unknown",
            "Integrite et restauration SQLite non testees par cette consultation; utiliser Doctor",
        ),
        (
            "authenticity",
            "unknown",
            "Signature Authenticode et provenance du binaire non verifiees ici",
        ),
        (
            "capabilities",
            "unknown",
            "Audit des permissions IPC et CSP non effectue pendant cette consultation",
        ),
        (
            "market",
            "unknown",
            "Fraicheur et confidentialite des flux non verifiees ici",
        ),
        (
            "orders",
            "unknown",
            "Etat financier et autorisation de trading non audites pendant cette consultation",
        ),
        (
            "proton",
            "unknown",
            "Connexion Proton VPN non verifiee; consulter l'application Proton officielle",
        ),
        (
            "kill_switch",
            "unknown",
            "Kill switch et protection DNS/IP du VPN non verifies",
        ),
    ];
    CHECKS
        .iter()
        .map(|(id, status, details)| format!("{id}|{status}|{details}"))
        .collect()
}

/// Locally generated, allowlisted codes only. No paths, payloads or raw errors.
#[tauri::command]
pub fn ipc_security_events(
    window: Window,
    session: State<'_, IpcAuth>,
    events: State<'_, SecurityEvents>,
) -> Result<Vec<String>, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(events.snapshot())
}

/// No internet probe, process enumeration, shell command or broker request.
/// Must remain main-window-only and behind the Rust-owned live session.
#[tauri::command]
pub async fn ipc_security_report(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<Vec<String>, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    let mut rows = security_report_rows();
    rows[1] = match desktop.security_database_health_code().await {
        "database_ok" => {
            "sqlite|verified|SQLite quick_check OK; restauration de sauvegarde non testee"
        }
        "database_missing" => {
            "sqlite|blocked|Base SQLite absente; consulter Doctor sans reinitialiser"
        }
        "database_invalid_path" => "sqlite|blocked|Chemin de base invalide; consulter Doctor",
        "database_integrity_failed" => {
            "sqlite|blocked|Controle SQLite en echec; sauvegarder avant toute reparation"
        }
        "database_schema_missing" => {
            "sqlite|blocked|Schema du magasin SQLite incomplet; consulter Doctor"
        }
        _ => "sqlite|unknown|Base indisponible ou occupee; aucune reparation automatique",
    }
    .to_string();
    finish_authorized_read(&session, &issued_to, Ok(rows))
}

// Acceptance tests are requested from the installed desktop UI, not a separate
// diagnostic app. All results are allowlisted. No shell, broker API, network,
// user-profile enumeration, backup mutation or installation action is performed.
fn acceptance_report_rows(phase: &str) -> Result<Vec<String>, &'static str> {
    if phase != "preproduction" && phase != "postproduction" {
        return Err("INVALID_ACCEPTANCE_PHASE");
    }
    let synthetic = PaperSandbox::default();
    let original = synthetic.snapshot();
    let verified = synthetic.trade("SOL", "BUY", 1).is_ok()
        && synthetic.trade("DOGE", "BUY", 1).is_err()
        && synthetic.reset() == original;
    let paper_status = if verified { "verified" } else { "blocked" };
    let paper_detail = if verified {
        "Achat fictif, instrument interdit et reinitialisation testes en memoire"
    } else {
        "Echec d'un test de simulation fictive; operations reelles interdites"
    };
    // The phase is report metadata, not a tested condition or a PASS.
    // Both modes deliberately run the same bounded, local acceptance checks.
    Ok(vec![
        "session|verified|Session Rust requise pour demander et lire ce rapport".to_owned(),
        "ipc|verified|Commande IPC de la fenetre principale autorisee".to_owned(),
        format!("paper|{paper_status}|{paper_detail}"),
        "sqlite|unknown|Controle SQLite en cours; restauration non verifiee".to_owned(),
        "webview|verified|Cette fenetre Tauri peut communiquer par IPC Rust".to_owned(),
        "installer|unknown|Installation hors reseau sur VM sans WebView2 non verifiee ici"
            .to_owned(),
        "persistence|unknown|Redemarrage, mise a niveau et sauvegarde non verifies ici".to_owned(),
        "authenticode|unknown|Signature et horodatage non verifies ici".to_owned(),
        "live|unknown|Aucune verification d'une autorisation de trading reel; ne pas l'activer"
            .to_owned(),
    ])
}

/// Read-only, local pre/postproduction acceptance directly inside Pravda-Bot.
/// Unrelated diagnostic suites and physical installation tests remain separate
/// gates; a local PASS must never replace a host-level VM/installer acceptance.
#[tauri::command]
pub async fn ipc_acceptance_report(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    phase: String,
) -> Result<Vec<String>, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    let mut rows = acceptance_report_rows(&phase).map_err(str::to_string)?;
    rows[3] = match desktop.security_database_health_code().await {
        "database_ok" => {
            "sqlite|verified|SQLite quick_check OK; sauvegarde et restauration non verifiees"
        }
        "database_missing" => {
            "sqlite|blocked|Base SQLite absente; aucune reinitialisation automatique"
        }
        "database_invalid_path" => {
            "sqlite|blocked|Chemin de base invalide; aucune reinitialisation automatique"
        }
        "database_integrity_failed" => {
            "sqlite|blocked|SQLite quick_check en echec; aucune reparation automatique"
        }
        "database_schema_missing" => {
            "sqlite|blocked|Schema SQLite attendu manquant; aucune reparation automatique"
        }
        _ => "sqlite|unknown|SQLite indisponible ou occupee; aucune action automatique",
    }
    .to_owned();
    finish_authorized_read(&session, &issued_to, Ok(rows))
}

#[tauri::command]
pub async fn ipc_fx_status(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
) -> Result<pravda_server::DesktopPayload, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    finish_authorized_read(&session, &issued_to, Ok(desktop.fx_status().await))
}

#[tauri::command]
pub async fn ipc_public_feed(
    window: Window,
    session: State<'_, IpcAuth>,
    desktop: State<'_, pravda_server::DesktopRuntime>,
    consent: State<'_, RevolutPublicConsent>,
    source: String,
) -> Result<pravda_server::DesktopPayload, String> {
    let issued_to = authorized_read_token(&session, window.label()).map_err(str::to_string)?;
    if !matches!(source.as_str(), "energy" | "news" | "revolut" | "sources") {
        return Err("INVALID_PUBLIC_SOURCE".to_owned());
    }
    if source == "revolut" && !consent.enabled() {
        return Err("PUBLIC_FEED_OPT_IN_REQUIRED".to_owned());
    }
    let result = desktop.public_feed(&source).await;
    // A revocation while the HTTPS request was in flight must not expose data.
    if source == "revolut" && !consent.enabled() {
        return Err("PUBLIC_FEED_OPT_IN_REQUIRED".to_owned());
    }
    finish_authorized_read(&session, &issued_to, Ok(result))
}

#[tauri::command]
pub fn ipc_revolut_consent_get(
    window: Window,
    session: State<'_, IpcAuth>,
    consent: State<'_, RevolutPublicConsent>,
) -> Result<bool, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    Ok(consent.enabled())
}

#[tauri::command]
pub fn ipc_revolut_consent_set(
    window: Window,
    session: State<'_, IpcAuth>,
    consent: State<'_, RevolutPublicConsent>,
    enabled: bool,
) -> Result<bool, String> {
    require_authenticated(&session, window.label()).map_err(str::to_string)?;
    consent.set(enabled).map_err(str::to_string)?;
    Ok(consent.enabled())
}

fn safe_article_link(raw: &str) -> Result<String, &'static str> {
    if raw.len() > 1500 || raw.contains('\r') || raw.contains('\n') {
        return Err("INVALID_ARTICLE_LINK");
    }
    let link = url::Url::parse(raw).map_err(|_| "INVALID_ARTICLE_LINK")?;
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
    fn paper_research_preflight_cannot_turn_synthetic_quote_into_attestation() {
        let snapshot = serde_json::json!({
            "stream_state": "observations_recent",
            "symbols": {}
        });
        let rows = paper_research_preflight_rows(&snapshot);
        assert_eq!(rows[0], "mode|PAPER_RESEARCH_ONLY");
        assert!(rows[1].starts_with("observation|MDA_QUOTE_UNAVAILABLE"));
        assert_eq!(rows[2], "provenance|not_attested");
        assert_eq!(rows[3], "portfolio|not_attested");
        assert_eq!(rows[4], "reg|not_executed");
        assert_eq!(rows[5], "order|disabled");
        assert_eq!(rows[6], "live|disabled");
    }

    #[test]
    fn acceptance_report_keeps_physical_and_financial_gates_unverified() {
        for phase in ["preproduction", "postproduction"] {
            let rows = acceptance_report_rows(phase).unwrap();
            assert_eq!(rows.len(), 9);
            assert!(rows.iter().all(|line| line.split('|').count() == 3));
            assert_eq!(
                rows.iter()
                    .map(|row| row.split('|').next().unwrap())
                    .collect::<Vec<_>>(),
                [
                    "session",
                    "ipc",
                    "paper",
                    "sqlite",
                    "webview",
                    "installer",
                    "persistence",
                    "authenticode",
                    "live"
                ]
            );
            assert!(rows[2].starts_with("paper|verified|"));
            assert_eq!(
                rows.iter()
                    .filter(|row| row.split('|').nth(1) == Some("verified"))
                    .count(),
                4
            );
            for row in &rows[5..] {
                assert_eq!(row.split('|').nth(1), Some("unknown"));
            }
            assert!(!rows.join(" ").contains("API_KEY"));
        }
        assert!(acceptance_report_rows("production").is_err());
        assert!(acceptance_report_rows("../install").is_err());
    }

    #[test]
    fn security_events_are_bounded_constant_only_and_newest_first() {
        let events = SecurityEvents::default();
        events.record(SecurityEventCode::StorageFailure);
        for _ in 0..70 {
            events.record(SecurityEventCode::LoginRejected);
        }
        let list = events.snapshot();
        assert_eq!(list.len(), MAX_SECURITY_EVENTS);
        assert!(list.iter().all(|line| {
            let mut parts = line.split('|');
            parts
                .next()
                .is_some_and(|stamp| stamp.parse::<u64>().is_ok())
                && parts.next() == Some("login_rejected")
                && parts.next().is_none()
        }));
        events.record(SecurityEventCode::LoginAccepted);
        assert!(events.snapshot()[0].ends_with("|login_accepted"));
    }

    #[test]
    fn update_readiness_never_implies_network_or_installation() {
        let rows = update_status_rows();
        assert_eq!(rows.len(), 5);
        assert!(rows[0].starts_with("version|"));
        assert_eq!(
            &rows[1..],
            &[
                "channel|not_configured",
                "signature|not_verified",
                "automatic|disabled",
                "last_check|never",
            ]
        );
        assert!(!rows.iter().any(|row| {
            row.ends_with("|verified")
                || row.contains("|available")
                || row.contains("|installed")
                || row.contains("https://")
        }));
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
        session.accept_token(token, &PaperSandbox::default());
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

    #[tokio::test]
    async fn in_flight_read_is_denied_after_logout_and_new_login() {
        let (session, path) = new_session();
        let session = Arc::new(session);
        let password = format!("TestOnlySecret-{}-concurrent", std::process::id());
        session.authority.set_password(&password).unwrap();
        let token = session.authority.login(&password).unwrap();
        session.accept_token(token, &PaperSandbox::default());
        let issued_to = authorized_read_token(&session, "main").unwrap();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel();
        let in_flight = Arc::clone(&session);
        let pending = tokio::spawn(async move {
            let _ = ready_tx.send(());
            let _ = release_rx.await;
            finish_authorized_read(&in_flight, &issued_to, Ok("private-data".to_owned()))
        });
        ready_rx.await.unwrap();
        session.logout();
        let new_token = session.authority.login(&password).unwrap();
        session.accept_token(new_token, &PaperSandbox::default());
        release_tx.send(()).unwrap();
        assert_eq!(pending.await.unwrap(), Err("UNAUTHORIZED".to_owned()));

        let current = authorized_read_token(&session, "main").unwrap();
        assert_eq!(
            finish_authorized_read(&session, &current, Ok("new-session".to_owned())),
            Ok("new-session".to_owned())
        );
        session.authority.logout(&current);
        assert_eq!(
            finish_authorized_read(&session, &current, Ok("expired".to_owned())),
            Err("UNAUTHORIZED".to_owned())
        );
        assert_eq!(
            authorized_read_token(&session, "other"),
            Err("FORBIDDEN_WINDOW")
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn replacing_session_resets_synthetic_portfolio_before_authorizing_new_trades() {
        let (session, path) = new_session();
        let paper = PaperSandbox::default();
        let password = format!("TestOnlySecret-{}-paper-reset", std::process::id());
        session.authority.set_password(&password).unwrap();

        let first = session.authority.login(&password).unwrap();
        session.accept_token(first, &paper);
        {
            let _guard = paper_session_guard(&session, "main").unwrap();
            paper.trade("SOL", "BUY", 10).unwrap();
        }
        assert!(paper
            .snapshot()
            .iter()
            .any(|row| row == "position_milli|SOL|10"));

        let second = session.authority.login(&password).unwrap();
        session.accept_token(second, &paper);
        {
            let _guard = paper_session_guard(&session, "main").unwrap();
            assert_eq!(paper.snapshot(), PaperSandbox::default().snapshot());
            paper.trade("SOL", "BUY", 1).unwrap();
        }
        assert!(paper
            .snapshot()
            .iter()
            .any(|row| row == "position_milli|SOL|1"));
        session.logout();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn expired_or_revoked_session_fails_closed() {
        let (session, path) = new_session();
        let password = format!("TestOnlySecret-{}-uvwxyz", std::process::id());
        session.authority.set_password(&password).unwrap();
        let token = session.authority.login(&password).unwrap();
        session.accept_token(token.clone(), &PaperSandbox::default());
        session.authority.logout(&token);
        assert_eq!(require_authenticated(&session, "main"), Err("UNAUTHORIZED"));
        assert_eq!(safe_auth_error(AuthError::Invalid), "INVALID_CREDENTIALS");
        let _ = std::fs::remove_file(path);
    }
}
