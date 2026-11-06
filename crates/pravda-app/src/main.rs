#![cfg_attr(windows, windows_subsystem = "windows")]
// Windows GUI release: do not allocate a lifetime-bound console window.
// CLI/diagnostic tools remain separate Rust executables.
mod consent;
mod ipc;
mod paper;

use std::sync::Arc;
use tauri::{WebviewUrl, WebviewWindowBuilder};

// Keep desktop credentials and SQLite in a stable per-user directory, rather than
// inheriting Explorer, shortcut or terminal working directories. Explicit server
// environment overrides always take precedence.
#[cfg(windows)]
fn default_storage_paths(
    local_app_data: &std::path::Path,
    custom_db: bool,
    custom_auth: bool,
) -> Vec<(&'static str, std::path::PathBuf)> {
    let root = local_app_data.join("Pravda-Bot");
    let mut defaults = Vec::new();
    if !custom_db {
        defaults.push(("PRAVDA_DB_PATH", root.join("pravda.db")));
    }
    if !custom_auth {
        defaults.push(("PRAVDA_AUTH_PATH", root.join("pravda_auth.json")));
    }
    defaults
}

#[cfg(windows)]
fn configure_desktop_storage() -> Result<(), &'static str> {
    let custom_db = std::env::var_os("PRAVDA_DB_PATH").is_some();
    let custom_auth = std::env::var_os("PRAVDA_AUTH_PATH").is_some();
    if custom_db && custom_auth {
        return Ok(());
    }
    let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not defined")?;
    if local_app_data.is_empty() {
        return Err("LOCALAPPDATA is empty");
    }
    let defaults = default_storage_paths(
        std::path::Path::new(&local_app_data),
        custom_db,
        custom_auth,
    );
    let folder = std::path::Path::new(&local_app_data).join("Pravda-Bot");
    std::fs::create_dir_all(&folder).map_err(|_| {
        "Cannot create Pravda-Bot data directory in LOCALAPPDATA; check permissions"
    })?;
    for (key, path) in defaults {
        // Called before launching the embedded HTTP server or any file workers.
        std::env::set_var(key, path);
    }
    Ok(())
}

#[cfg(not(windows))]
fn configure_desktop_storage() -> Result<(), &'static str> {
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn default_data_paths_do_not_depend_on_current_working_directory() {
        let base = std::path::Path::new(r"C:\Users\Tester\AppData\Local");
        let result = default_storage_paths(base, false, false);
        assert_eq!(
            result,
            vec![
                ("PRAVDA_DB_PATH", base.join("Pravda-Bot").join("pravda.db")),
                (
                    "PRAVDA_AUTH_PATH",
                    base.join("Pravda-Bot").join("pravda_auth.json")
                )
            ]
        );
    }
    #[test]
    fn explicit_paths_are_never_replaced() {
        let base = std::path::Path::new(r"C:\Users\Tester\AppData\Local");
        assert!(default_storage_paths(base, true, true).is_empty());
        assert_eq!(
            default_storage_paths(base, true, false),
            vec![(
                "PRAVDA_AUTH_PATH",
                base.join("Pravda-Bot").join("pravda_auth.json")
            )]
        );
        assert_eq!(
            default_storage_paths(base, false, true),
            vec![("PRAVDA_DB_PATH", base.join("Pravda-Bot").join("pravda.db"))]
        );
    }
}

#[tokio::main]
async fn main() {
    if let Err(message) = configure_desktop_storage() {
        eprintln!("Pravda-Bot desktop storage error: {message}");
        std::process::exit(1);
    }

    // One Rust password/session authority shared by the compatibility HTTP
    // adapter and IPC. No independent password caches or parallel verifiers.
    let auth_path =
        std::env::var_os("PRAVDA_AUTH_PATH").unwrap_or_else(|| "pravda_auth.json".into());
    if auth_path.is_empty() {
        eprintln!("Pravda-Bot cannot start: authentication path is empty");
        std::process::exit(1);
    }
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if let Some(result) = pravda_server::revolut_cli::run(&arguments).await {
        if let Err(message) = result {
            eprintln!("{message}");
            std::process::exit(1);
        }
        return;
    }
    let public_consent = consent::RevolutPublicConsent::load(std::path::Path::new(&auth_path));
    let shared_auth = match pravda_server::Auth::load(
        std::path::PathBuf::from(auth_path),
        pravda_server::PBKDF2_ITERATIONS,
    ) {
        Ok(authority) => Arc::new(authority),
        Err(message) => {
            eprintln!("Pravda-Bot cannot start: {message}");
            std::process::exit(1);
        }
    };
    let ipc_auth = ipc::IpcAuth::new(shared_auth.clone());

    // The desktop owns its SQLite store, migration and market engine directly.
    // No HTTP listener, reqwest readiness poll, or Axum child task is started.
    let desktop = match pravda_server::DesktopRuntime::start(shared_auth).await {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Pravda-Bot desktop initialization failed: {error}");
            std::process::exit(1);
        }
    };

    // The only native WebView opens bundled local assets; no remote origin.
    tauri::Builder::default()
        .manage(ipc_auth)
        .manage(public_consent)
        .manage(ipc::SecurityEvents::default())
        .manage(paper::PaperSandbox::default())
        .manage(desktop)
        .invoke_handler(tauri::generate_handler![
            ipc::ipc_auth_status,
            ipc::ipc_auth_setup,
            ipc::ipc_auth_login,
            ipc::ipc_auth_logout,
            ipc::ipc_session_check,
            ipc::ipc_market_snapshot,
            ipc::ipc_history_recent,
            ipc::ipc_history_query,
            ipc::ipc_history_export_csv,
            ipc::ipc_paper_alerts_snapshot,
            ipc::ipc_paper_snapshot,
            ipc::ipc_paper_trade,
            ipc::ipc_paper_reset,
            ipc::ipc_runtime_metrics,
            ipc::ipc_update_status,
            ipc::ipc_security_report,
            ipc::ipc_acceptance_report,
            ipc::ipc_security_events,
            ipc::ipc_fx_status,
            ipc::ipc_public_feed,
            ipc::ipc_revolut_consent_get,
            ipc::ipc_revolut_consent_set,
            ipc::ipc_open_source
        ])
        .setup(|app| {
            // WebDriver's Windows Edge adapter may only enumerate the primary
            // WebView. A debug-only test mode starts the unauthorized local
            // window ALONE, with no new capability or JS command permissions.
            // The regular main window is used in a separate native test run.
            #[cfg(all(windows, debug_assertions, feature = "security-e2e"))]
            if std::env::var("PRAVDA_E2E_PROBE_ONLY").as_deref() == Ok("true") {
                let probe_url = WebviewUrl::App("security-probe.html".into());
                WebviewWindowBuilder::new(app, "security-probe", probe_url)
                    .title("Pravda-Bot Security Probe (NO IPC PERMISSION)")
                    .inner_size(700.0, 500.0)
                    .build()?;
                return Ok(());
            }
            // Production-center mode must never reuse the installed app's
            // WebView2 cache, cookies or localStorage during a diagnostic.
            let mut main_window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()));
            #[cfg(windows)]
            if std::env::var("PRAVDA_OFFLINE_TEST").as_deref() == Ok("true") {
                if let Some(path) = std::env::var_os("PRAVDA_DIAGNOSTIC_WEBVIEW_DIR") {
                    main_window = main_window.data_directory(std::path::PathBuf::from(path));
                    // Only the isolated offline diagnostic may hide its temporary
                // window. Never suppress the normal interactive application.
                if std::env::var("PRAVDA_BACKGROUND_TEST").as_deref() == Ok("true") {
                    main_window = main_window.visible(false);
                }
            }
            }
            main_window
                .title("Pravda-Bot")
                .inner_size(1280.0, 800.0)
                .resizable(true)
                .build()?;

            // Native Windows E2E only: simultaneously run an unprivileged
            // WebView inside this SAME authenticated Rust/Tauri process.
            // The capability manifest is never modified and this code is
            // absent from Release builds even when the feature is enabled.
            #[cfg(all(windows, debug_assertions, feature = "security-e2e"))]
            if std::env::var("PRAVDA_E2E_CONCURRENT").as_deref() == Ok("true") {
                let report_path = std::path::PathBuf::from(
                    std::env::var_os("PRAVDA_E2E_CONCURRENT_REPORT")
                        .expect("E2E concurrent probe requires a test-only report path"),
                );
                let url = WebviewUrl::App("security-probe-concurrent.html".into());
                WebviewWindowBuilder::new(app, "security-probe", url)
                    .title("Pravda security probe (concurrent, NO IPC)")
                    .on_document_title_changed(move |_window, title| {
                        // This is a DEBUG-ONLY evidence side channel, NOT a
                        // Tauri command, permission, listener or production API.
                        // A trusted local probe script emits bounded base64.
                        if let Some(payload) = title.strip_prefix("PRAVDA_E2E_PROBE:") {
                            if payload.len() > 3_000 {
                                return;
                            }
                            let millis = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis();
                            if let Ok(mut report) = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&report_path)
                            {
                                use std::io::Write;
                                let _ =
                                    writeln!(report, "{millis}|{}|{payload}", std::process::id());
                            }
                        }
                    })
                    .inner_size(680.0, 450.0)
                    .build()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Pravda-Bot desktop window failed to initialize");
}
