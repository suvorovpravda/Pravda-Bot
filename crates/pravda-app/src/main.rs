use tauri::{Manager, WindowBuilder};
use std::process::Command;

#[tokio::main]
async fn main() {
    // Démarre le serveur Axum en arrière-plan
    let _server = Command::new("cargo")
        .args(["run", "--release", "-p", "pravda-server"])
        .spawn()
        .expect("Impossible de démarrer pravda-server");

    // Petit délai pour laisser le serveur démarrer
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    tauri::Builder::default()
        .setup(|app| {
            WindowBuilder::new(
                app,
                "main",
                tauri::WindowUrl::External("http://127.0.0.1:8080".parse().unwrap()),
            )
            .title("Pravda-Bot")
            .inner_size(1280.0, 800.0)
            .resizable(true)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
