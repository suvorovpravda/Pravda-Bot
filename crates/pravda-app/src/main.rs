use tauri::{WebviewWindowBuilder, WebviewUrl};

#[tokio::main]
async fn main() {
    // Démarre le serveur Axum directement dans le même processus
    tokio::spawn(async {
        if let Err(e) = pravda_server::run_server().await {
            eprintln!("❌ Pravda server error: {}", e);
        }
    });

    // Petit délai pour laisser le serveur démarrer
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    tauri::Builder::default()
        .setup(|app| {
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External("http://127.0.0.1:8080".parse().unwrap()),
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
