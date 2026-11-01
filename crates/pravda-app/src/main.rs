use std::time::{Duration, Instant};
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
