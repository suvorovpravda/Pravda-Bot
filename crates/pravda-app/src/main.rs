use tauri::{Manager, WindowBuilder};

#[tokio::main]
async fn main() {
    // Démarrer le serveur Axum en background
    // On utilise le binaire déjà compilé pravda-server
    // Ici on lance via tokio::process::Command pour l'exemple
    // En production, on intégrerait le code du serveur directement

    tauri::Builder::default()
        .setup(|app| {
            let _ = WindowBuilder::new(
                app,
                "main",
                tauri::WindowUrl::External("http://127.0.0.1:8080".parse().unwrap()),
            )
            .title("Pravda-Bot")
            .inner_size(1280.0, 800.0)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
