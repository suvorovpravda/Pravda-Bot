use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct ServerProcess {
    child: Child,
    directory: PathBuf,
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.directory);
    }
}

// The integration test never embeds a reusable administrator password.
fn test_password() -> String {
    let mut bytes = [0_u8; 20];
    getrandom::getrandom(&mut bytes).expect("test entropy must be available");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn http_request(port: u16, host: &str, extra: &str, path: &str) -> std::io::Result<String> {
    request(port, host, extra, "GET", path, "")
}

fn request(
    port: u16,
    host: &str,
    extra: &str,
    method: &str,
    path: &str,
    body: &str,
) -> std::io::Result<String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_millis(300))?;
    socket.set_read_timeout(Some(Duration::from_secs(10)))?;
    socket.write_all(
        format!("{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\n{extra}Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes(),
    )?;
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = socket.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        response.extend_from_slice(&buffer[..count]);
        // Tests only need response headers for WebSockets and login POSTs.
        // Do not wait for EOF on a server-managed keep-alive connection.
        let headers_complete = response.windows(4).any(|w| w == b"\r\n\r\n");
        if headers_complete
            && (response.starts_with(b"HTTP/1.1 101 ")
                || (path == "/ws/ticks" && response.starts_with(b"HTTP/1.1 401 "))
                || (method == "POST" && path == "/login"))
        {
            break;
        }
        assert!(
            response.len() <= 2_000_000,
            "HTTP response exceeded test limit"
        );
    }
    Ok(String::from_utf8(response).expect("response is UTF-8"))
}

#[test]
fn binary_starts_once_and_serves_local_http_and_websocket() {
    // Obtain an unused port without assuming that the developer's 8080 is free.
    let probe = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("pravda-startup-{}-{nonce}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let password = test_password();
    let child = Command::new(env!("CARGO_BIN_EXE_pravda-server"))
        .env("DASHBOARD_PORT", port.to_string())
        .env("PRAVDA_DB_PATH", directory.join("test.sqlite3"))
        .env("PRAVDA_MARKET_FEEDS_ENABLED", "false")
        .env("PRAVDA_AUTH_PATH", directory.join("auth.json"))
        .env("PRAVDA_ADMIN_PASSWORD", &password)
        .env_remove("MARKET_DATA_STALE_SECONDS")
        .env_remove("MAX_EXCHANGE_SKEW_SECONDS")
        .env_remove("MAX_ALLOWED_SPREAD_PERCENT")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = ServerProcess { child, directory };
    let response = (0..500)
        .find_map(|_| {
            if let Some(status) = server.child.try_wait().unwrap() {
                panic!("server exited before becoming ready: {status}");
            }
            match http_request(port, "127.0.0.1", "Connection: close\r\n", "/login") {
                Ok(response) => Some(response),
                Err(_) => {
                    thread::sleep(Duration::from_millis(20));
                    None
                }
            }
        })
        .expect("server did not answer within ten seconds");
    assert!(
        response.starts_with("HTTP/1.1 200 "),
        "login page must return HTTP 200"
    );
    assert!(response
        .to_ascii_lowercase()
        .contains("x-pravda-server: 1\r\n"));

    // Everything except the login page is closed to unauthenticated clients.
    for path in ["/", "/api/metrics", "/api/market"] {
        let denied = http_request(port, "127.0.0.1", "Connection: close\r\n", path).unwrap();
        assert!(
            !denied.starts_with("HTTP/1.1 200 "),
            "protected route must reject unauthenticated requests"
        );
    }
    let upgrade = format!("Origin: http://127.0.0.1:{port}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n");
    let denied = http_request(port, "127.0.0.1", &upgrade, "/ws/ticks").unwrap();
    assert!(
        denied.starts_with("HTTP/1.1 401 "),
        "unauthenticated WebSocket upgrade must be denied"
    );

    let form = "Connection: close\r\nContent-Type: application/x-www-form-urlencoded\r\n";
    let wrong = request(
        port,
        "127.0.0.1",
        form,
        "POST",
        "/login",
        &format!("password={password}-wrong"),
    )
    .unwrap();
    assert!(
        wrong.starts_with("HTTP/1.1 401 "),
        "invalid password must be denied"
    );
    let login = request(
        port,
        "127.0.0.1",
        form,
        "POST",
        "/login",
        &format!("password={password}"),
    )
    .unwrap();
    assert!(
        login.starts_with("HTTP/1.1 303 "),
        "valid password must redirect"
    );
    let cookie = login
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .starts_with("set-cookie:")
                .then(|| line[11..].trim().split(';').next().unwrap().to_owned())
        })
        .expect("session cookie");
    let response = http_request(
        port,
        "127.0.0.1",
        &format!("Connection: close\r\nCookie: {cookie}\r\n"),
        "/",
    )
    .unwrap();
    assert!(
        response.starts_with("HTTP/1.1 200 "),
        "authenticated dashboard must return HTTP 200"
    );
    let headers = response
        .split("\r\n\r\n")
        .next()
        .unwrap()
        .to_ascii_lowercase();
    assert!(headers.contains(&format!("ws://127.0.0.1:{port}")));
    assert!(headers.contains(&format!("ws://localhost:{port}")));
    assert!(headers.contains("connect-src 'self' "));
    assert!(headers.contains("frame-ancestors 'none'"));
    assert!(response.contains("new WebSocket(`ws://${location.host}/ws/ticks`)"));

    for host in ["127.0.0.1", "localhost"] {
        let upgrade = format!("Cookie: {cookie}\r\nOrigin: http://{host}:{port}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n");
        let response = http_request(port, host, &upgrade, "/ws/ticks").unwrap();
        assert!(
            response.starts_with("HTTP/1.1 101 "),
            "authenticated WebSocket upgrade must succeed"
        );
    }
    let rejected = http_request(
        port,
        "127.0.0.1",
        &format!("Cookie: {cookie}\r\nOrigin: http://attacker.example\r\nConnection: close\r\n"),
        "/api/metrics",
    )
    .unwrap();
    assert!(rejected.starts_with("HTTP/1.1 403 "));
}
