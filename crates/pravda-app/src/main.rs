use tauri::Manager;

#[tokio::main]
async fn main() {
    // Démarrer le serveur Axum en arrière-plan
    let server_handle = tokio::spawn(async {
        let _ = rust::pravda_server::main().await;
    });

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            // Ouvre le dashboard une fois le serveur prêt
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                window.open_devtools();
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Le serveur tourne jusqu'à la fermeture de l'app
    let _ = server_handle.await;
}
