fn main() {
    // Every custom command requires its own generated permission.
    // No wildcard allowance and no remote origins.
    const COMMANDS: &[&str] = &[
        "ipc_auth_status",
        "ipc_auth_setup",
        "ipc_auth_login",
        "ipc_auth_logout",
        "ipc_session_check",
        "ipc_market_snapshot",
        "ipc_history_recent",
        "ipc_history_query",
        "ipc_history_export_csv",
        "ipc_paper_alerts_snapshot",
        "ipc_paper_snapshot",
        "ipc_paper_trade",
        "ipc_paper_reset",
        "ipc_runtime_metrics",
        "ipc_security_report",
        "ipc_security_events",
        "ipc_fx_status",
        "ipc_public_feed",
        "ipc_revolut_consent_get",
        "ipc_revolut_consent_set",
        "ipc_open_source",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("Tauri IPC permission manifest must be valid");
}
