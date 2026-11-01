use std::time::{Duration, Instant};
use tauri::{WebviewUrl, WebviewWindowBuilder};

#[tokio::main]
async fn main() {
    let port: u16 = match std::env::var("DASHBOARD_PORT") {
        Ok(raw) => match raw.parse() {
            Ok(port) if port != 0 => port,
            _ => {
                eprintln!("Invalid DASHBOARD_PORT: expected an integer between 1 and 65535");
                std::process::exit(1);
            }
        },
        Err(std::env::VarError::NotPresent) => 8080,
        Err(_) => {
            eprintln!("DASHBOARD_PORT is not valid Unicode");
            std::process::exit(1);
        }
    };

    let address = format!("http://127.0.0.1:{port}");
    // Never open another process's HTTP service in the desktop WebView.
    if std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
        eprintln!("Pravda-Bot cannot start: dashboard port {port} is already occupied");
        std::process::exit(1);
    }

    let server =
        tokio::spawn(async { pravda_server::run_server().await.map_err(|e| e.to_string()) });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(800))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("HTTP client configuration must be valid");

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if server.is_finished() {
            let reason = match server.await {
                Ok(Ok(())) => "the server stopped before becoming ready".to_string(),
                Ok(Err(error)) => error,
                Err(error) => error.to_string(),
            };
            eprintln!("Pravda-Bot could not start: {reason}");
            std::process::exit(1);
        }

        match client.get(format!("{address}/login")).send().await {
            Ok(response) => {
                let genuine_server = response.status() == reqwest::StatusCode::OK
                    && response
                        .headers()
                        .get("x-pravda-server")
                        .and_then(|value| value.to_str().ok())
                        == Some("1");
                if genuine_server {
                    break;
                }
                eprintln!("Pravda-Bot refused to open an unexpected local web service");
                std::process::exit(1);
            }
            Err(_) => {}
        }

        if Instant::now() >= deadline {
            eprintln!("Pravda-Bot server did not become ready before the startup deadline");
            std::process::exit(1);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Config has no automatic windows: build one window only after the server is ready.
    tauri::Builder::default()
        .setup(move |app| {
            WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::External(address.parse().expect("loopback URL is valid")),
            )
            .title("Pravda-Bot")
            .inner_size(1280.0, 800.0)
            .resizable(true)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Pravda-Bot desktop window failed to initialize");
}
