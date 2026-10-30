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

fn http_request(port: u16, host: &str, extra: &str, path: &str) -> std::io::Result<String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_millis(300))?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\n{extra}\r\n").as_bytes(),
    )?;
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = socket.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        response.extend_from_slice(&buffer[..count]);
        // WebSocket connections remain open after the handshake.
        if response.starts_with(b"HTTP/1.1 101 ") && response.windows(4).any(|w| w == b"\r\n\r\n") {
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
    let child = Command::new(env!("CARGO_BIN_EXE_pravda-server"))
        .env("DASHBOARD_PORT", port.to_string())
        .env("PRAVDA_DB_PATH", directory.join("test.sqlite3"))
        .env("PRAVDA_MARKET_FEEDS_ENABLED", "false")
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
            match http_request(port, "127.0.0.1", "Connection: close\r\n", "/") {
                Ok(response) => Some(response),
                Err(_) => {
                    thread::sleep(Duration::from_millis(20));
                    None
                }
            }
        })
        .expect("server did not answer within ten seconds");
    assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
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
        let upgrade = format!("Origin: http://{host}:{port}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n");
        let response = http_request(port, host, &upgrade, "/ws/ticks").unwrap();
        assert!(response.starts_with("HTTP/1.1 101 "), "{response}");
    }
    let rejected = http_request(
        port,
        "127.0.0.1",
        "Origin: http://attacker.example\r\nConnection: close\r\n",
        "/api/metrics",
    )
    .unwrap();
    assert!(rejected.starts_with("HTTP/1.1 403 "));
}
