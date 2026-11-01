use tauri::{WebviewWindowBuilder, WebviewUrl};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("DASHBOARD_PORT")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .filter(|port| *port != 0)
        .unwrap_or(8080);
    let url = format!("http://127.0.0.1:{port}");
    // Démarre le serveur Axum directement dans le même processus
    let _server_handle = tokio::spawn(async {
        if let Err(e) = pravda_server::run_server().await {
            eprintln!("❌ Pravda server error: {}", e);
        }
    });

    // Attendre que le serveur soit prêt sur le port 8080
    let server_ready = async {
        let client = reqwest::Client::new();
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 60 {
                eprintln!("⚠️ Serveur non prêt après 60 tentatives, on continue quand même");
                break;
            }
            match client.get(&url).send().await {
                Ok(_) => break,
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    };
    // On n'attend pas indéfiniment, on lance en parallèle
    tokio::select! {
        _ = server_ready => {},
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {},
    };


    tauri::Builder::default()
        .setup(move |app| {
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(url.parse().unwrap()),
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
