fn main() {
    // Every custom command requires its own generated permission.
    // No wildcard allowance and no remote origins.
    const COMMANDS: &[&str] = &[
        "ipc_auth_status",
        "ipc_auth_setup",
        "ipc_auth_login",
        "ipc_auth_logout",
        "ipc_session_check",
    ];
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("Tauri IPC permission manifest must be valid");
}
