//! One-click, no-secrets diagnostic companion for Pravda-Bot.
//!
//! This executable has no trading API, no credentials, no order transport,
//! no HTTP listener and no automatic repair. Run it directly from Explorer.
//! The report is intentionally allowlisted: never print raw OS errors, paths,
//! environment variables, SQLite contents, machine names or process lists.

#[cfg(windows)]
use std::{fs, path::Path, process::Command};
use std::{
    fs::OpenOptions,
    io::{self, Write},
};

#[cfg(windows)]
const DOCTOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Clone, Copy)]
enum Status {
    Pass,
    Warn,
    Fail,
    Skip,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
        }
    }
}

struct Check {
    code: &'static str,
    status: Status,
    advice: String,
}

impl Check {
    fn new(code: &'static str, status: Status, advice: impl Into<String>) -> Self {
        Self {
            code,
            status,
            advice: advice.into(),
        }
    }
}

#[cfg(windows)]
fn inspect_file(path: &Path, label: &'static str) -> Check {
    match fs::symlink_metadata(path) {
        Ok(data) if data.is_file() && !data.file_type().is_symlink() => {
            Check::new(label, Status::Pass, "File present")
        }
        Ok(_) => Check::new(label, Status::Fail, "Expected regular file is unavailable"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Check::new(
            label,
            Status::Warn,
            "File not found; installation or setup may be incomplete",
        ),
        Err(_) => Check::new(label, Status::Warn, "File metadata unavailable"),
    }
}

fn report_lines(checks: &[Check]) -> String {
    let mut result = String::from(
        "PRAVDA-DOCTOR REPORT v2\n\
         Copy only this report into your support conversation.\n\
         SAFE MODE: no repairs, authenticated exchange calls, orders or credentials.\n\
         OFFLINE: no DNS, TCP, HTTP, remote diagnostics or trading.\n\
         No filesystem paths, account IDs, log text or secrets collected.\n\n",
    );
    for check in checks {
        result.push_str(check.status.label());
        result.push_str(" | ");
        result.push_str(check.code);
        result.push_str(" | ");
        result.push_str(&check.advice);
        result.push('\n');
    }
    let failed = checks
        .iter()
        .filter(|c| matches!(c.status, Status::Fail))
        .count();
    let warned = checks
        .iter()
        .filter(|c| matches!(c.status, Status::Warn))
        .count();
    let skipped = checks
        .iter()
        .filter(|c| matches!(c.status, Status::Skip))
        .count();
    result.push_str(&format!(
        "SUMMARY | failures={failed} | warnings={warned} | skipped={skipped}\n"
    ));
    result
}

/// Check only that a local file can be opened. Never print contents,
/// metadata containing identities, file names or raw OS errors.
#[cfg(windows)]
fn inspect_optional_file(path: &Path, label: &'static str) -> Check {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            Check::new(label, Status::Pass, "Optional SQLite sidecar detected")
        }
        Ok(_) => Check::new(
            label,
            Status::Warn,
            "Optional SQLite sidecar has invalid type",
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Check::new(
            label,
            Status::Skip,
            "SQLite sidecar absent; normal when database is idle",
        ),
        Err(_) => Check::new(label, Status::Warn, "SQLite sidecar metadata unreadable"),
    }
}

#[cfg(windows)]
fn inspect_readonly_file(path: &Path, label: &'static str) -> Check {
    match fs::File::open(path) {
        Ok(_) => Check::new(label, Status::Pass, "Readable without changing contents"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Check::new(label, Status::Warn, "File not configured yet")
        }
        Err(_) => Check::new(label, Status::Warn, "File cannot be opened for reading"),
    }
}

/// Do not execute the application: a header probe must not start trading,
/// create new configuration, or change any process state.
#[cfg(windows)]
fn inspect_pe_header(path: &Path) -> Check {
    use io::Read;
    if !path.is_file() {
        return Check::new(
            "adjacent_app_binary",
            Status::Warn,
            "Application not next to Doctor; may be installed elsewhere",
        );
    }
    let mut header = [0_u8; 2];
    match fs::File::open(path).and_then(|mut file| file.read_exact(&mut header)) {
        Ok(()) if header == *b"MZ" => Check::new(
            "adjacent_app_binary",
            Status::Pass,
            "Windows executable header detected",
        ),
        Ok(()) => Check::new(
            "adjacent_app_binary",
            Status::Fail,
            "Executable header invalid",
        ),
        Err(_) => Check::new(
            "adjacent_app_binary",
            Status::Warn,
            "Cannot read executable header",
        ),
    }
}

/// Fingerprint the installed executable so support can distinguish exact
/// builds. Only the 64-character SHA-256 is emitted; never the local path.
#[cfg(windows)]
fn inspect_application_digest(path: &Path) -> Check {
    use io::Read;
    use sha2::{Digest, Sha256};
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return Check::new(
                "application_sha256",
                Status::Skip,
                "No readable executable to fingerprint",
            )
        }
    };
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 65_536];
    loop {
        let bytes = match file.read(&mut buf) {
            Ok(n) => n,
            Err(_) => {
                return Check::new(
                    "application_sha256",
                    Status::Warn,
                    "Executable fingerprint could not be calculated",
                )
            }
        };
        if bytes == 0 {
            break;
        }
        hasher.update(&buf[..bytes]);
    }
    let digest = hasher.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut fingerprint = String::with_capacity(64);
    for &byte in digest.iter() {
        fingerprint.push(char::from(HEX[usize::from(byte >> 4)]));
        fingerprint.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Check::new(
        "application_sha256",
        Status::Pass,
        format!("SHA256={fingerprint}"),
    )
}

/// Reuse the production password-record validation, but do not ask for
/// passwords or authenticate. No serialized verifier values enter the report.
#[cfg(windows)]
fn inspect_auth_record(path: &Path) -> Check {
    if !path.is_file() {
        return Check::new(
            "auth_record_validity",
            Status::Skip,
            "Authentication record has not been created",
        );
    }
    match pravda_server::Auth::load(path, pravda_server::PBKDF2_ITERATIONS) {
        Ok(auth) if auth.is_configured() => Check::new(
            "auth_record_validity",
            Status::Pass,
            "Password verifier schema and algorithm valid; login not tested",
        ),
        Ok(_) => Check::new(
            "auth_record_validity",
            Status::Warn,
            "Password record has not been configured",
        ),
        Err(_) => Check::new(
            "auth_record_validity",
            Status::Fail,
            "Authentication record invalid or cannot be loaded",
        ),
    }
}

/// Fixed Windows registry command: never interpolate user-provided strings.
/// Only the command exit status is included in the support report.
#[cfg(windows)]
fn inspect_windows_webview2() -> Check {
    let status = Command::new("reg.exe")
        .args([
            "query",
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            "/v",
            "pv",
        ])
        .output();
    match status {
        Ok(result) if result.status.success() => Check::new(
            "webview2_runtime",
            Status::Pass,
            "WebView2 registry entry detected",
        ),
        Ok(_) => Check::new(
            "webview2_runtime",
            Status::Warn,
            "WebView2 not detected at standard machine registry location",
        ),
        Err(_) => Check::new(
            "webview2_runtime",
            Status::Skip,
            "Registry diagnostic utility unavailable",
        ),
    }
}

#[cfg(windows)]
fn inspect_pravda_process() -> Check {
    let result = Command::new("tasklist.exe")
        .args(["/FI", "IMAGENAME eq pravda-app.exe", "/NH", "/FO", "CSV"])
        .output();
    match result {
        Ok(output) if output.status.success() => {
            let found = String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim_start().starts_with("\"pravda-app.exe\""));
            if found {
                Check::new(
                    "application_process",
                    Status::Pass,
                    "Pravda-Bot process observed",
                )
            } else {
                Check::new(
                    "application_process",
                    Status::Warn,
                    "Pravda-Bot process not observed during snapshot",
                )
            }
        }
        _ => Check::new(
            "application_process",
            Status::Skip,
            "Process inspection unavailable",
        ),
    }
}

#[cfg(windows)]
fn checks() -> Vec<Check> {
    let mut checks = vec![Check::new(
        "windows_os",
        Status::Pass,
        "Windows native diagnostic executable running",
    )];
    checks.push(Check::new("doctor_version", Status::Pass, DOCTOR_VERSION));
    checks.push(Check::new(
        "process_architecture",
        if cfg!(target_arch = "x86_64") {
            Status::Pass
        } else {
            Status::Warn
        },
        if cfg!(target_arch = "x86_64") {
            "Native 64-bit x86 application"
        } else {
            "Not an x86-64 diagnostic executable"
        },
    ));
    checks.push(if std::env::temp_dir().is_dir() {
        Check::new(
            "temporary_directory",
            Status::Pass,
            "Temporary report directory exists",
        )
    } else {
        Check::new(
            "temporary_directory",
            Status::Fail,
            "Temporary report directory unavailable",
        )
    });
    checks.push(inspect_windows_webview2());
    checks.push(inspect_pravda_process());
    // The diagnostic must be safe to run on an air-gapped production host.
    // No socket is opened, even to localhost.
    checks.push(Check::new(
        "network_probe",
        Status::Skip,
        "Air-gapped mode: all outbound and loopback connection tests disabled",
    ));
    checks.push(Check::new(
        "application_gui",
        Status::Skip,
        "Interactive GUI behavior requires a user session and is not asserted",
    ));
    checks.push(Check::new(
        "live_trading",
        Status::Skip,
        "Live trading intentionally not tested or activated",
    ));

    let root = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Pravda-Bot"));
    match root {
        Some(ref root) => {
            let auth = inspect_file(&root.join("pravda_auth.json"), "local_auth_file");
            checks.push(auth);
            checks.push(inspect_readonly_file(
                &root.join("pravda_auth.json"),
                "auth_readability",
            ));
            checks.push(inspect_auth_record(&root.join("pravda_auth.json")));
            checks.push(inspect_optional_file(
                &root.join("pravda.db-wal"),
                "sqlite_wal_sidecar",
            ));
            checks.push(inspect_optional_file(
                &root.join("pravda.db-shm"),
                "sqlite_shm_sidecar",
            ));

            let db_path = root.join("pravda.db");
            use pravda_store::doctor::{inspect_database, DatabaseHealth};
            let db_health = inspect_database(&db_path);
            let (state, description) = match db_health {
                DatabaseHealth::Healthy => (Status::Pass, "SQLite integrity and ticks schema OK"),
                DatabaseHealth::Missing => (Status::Warn, "SQLite missing; start Pravda-Bot once"),
                DatabaseHealth::NotAFile => (Status::Fail, "SQLite path is not a regular file"),
                DatabaseHealth::Unavailable => (
                    Status::Warn,
                    "SQLite inaccessible; inspect file permissions or lock",
                ),
                DatabaseHealth::Corrupt => (Status::Fail, "SQLite integrity check failed"),
                DatabaseHealth::SchemaMissing => {
                    (Status::Fail, "SQLite expected ticks schema not found")
                }
            };
            checks.push(Check::new("sqlite_health", state, description));
        }
        None => {
            checks.push(Check::new(
                "local_auth_file",
                Status::Warn,
                "Windows local data directory unavailable",
            ));
            checks.push(Check::new(
                "sqlite_health",
                Status::Warn,
                "Database check skipped: local data directory unavailable",
            ));
        }
    }

    // Check a small fixed set of supported install layouts. Never report
    // the actual installation path, user folder or full executable name.
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(dir) = executable.parent() {
            candidates.push(dir.join("pravda-app.exe"));
        }
    }
    for key in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(folder) = std::env::var_os(key) {
            candidates.push(
                std::path::PathBuf::from(folder)
                    .join("Pravda-Bot")
                    .join("pravda-app.exe"),
            );
        }
    }
    let application = candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .or_else(|| candidates.first());
    checks.push(match application {
        Some(path) => inspect_pe_header(path),
        None => Check::new(
            "adjacent_app_binary",
            Status::Warn,
            "Cannot locate application in known installation layouts",
        ),
    });
    checks.push(match application {
        Some(path) => inspect_application_digest(path),
        None => Check::new(
            "application_sha256",
            Status::Skip,
            "Application executable not detected",
        ),
    });

    checks.push(Check::new(
        "market_feed_reachability",
        Status::Skip,
        "Passive offline diagnosis cannot establish public market feed health",
    ));
    checks.push(Check::new(
        "runtime_auth",
        Status::Skip,
        "Password and session verification require interactive authenticated test",
    ));
    checks.push(Check::new(
        "revolut_exchange",
        Status::Skip,
        "Private API disabled for diagnostics: no keys accessed",
    ));
    checks.push(Check::new(
        "installer_signature",
        Status::Skip,
        "Authenticode release signing must be checked independently",
    ));
    checks
}

#[cfg(not(windows))]
fn checks() -> Vec<Check> {
    vec![Check::new(
        "windows_os",
        Status::Warn,
        "This diagnostic is intended for Windows",
    )]
}

/// Writes only an allowlisted report in the user's temporary directory.
/// A fresh filename avoids overwriting an earlier report. No app data changed.
fn write_report(report: &str) -> io::Result<std::path::PathBuf> {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    for suffix in 0..16 {
        let filename = format!("pravda-doctor-{pid}-{stamp}-{suffix}.txt");
        let path = base.join(filename);
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(mut f) => {
                f.write_all(report.as_bytes())?;
                f.flush()?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Could not allocate report file",
    ))
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let quiet = args.len() == 1 && args[0] == "--no-open";
    let report = report_lines(&checks());
    println!("{report}");

    if !quiet {
        match write_report(&report) {
            Ok(path) => {
                #[cfg(windows)]
                if Command::new("notepad.exe").arg(&path).spawn().is_err() {
                    eprintln!("Could not open Notepad. Copy the console output above.");
                }
                #[cfg(not(windows))]
                let _ = path;
                println!("Copy the report from Notepad and paste it into ChatGPT.");
            }
            Err(_) => eprintln!("Cannot save report. Copy the console output above."),
        }
        println!("Press Enter to close this window.");
        let mut input = String::new();
        let _ = io::stdin().read_line(&mut input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_report_contains_only_allowlisted_fields() {
        let report = report_lines(&[
            Check::new("local_auth_file", Status::Pass, "File present"),
            Check::new(
                "sqlite_health",
                Status::Fail,
                "SQLite integrity check failed",
            ),
        ]);
        assert!(report.contains("PASS | local_auth_file"));
        assert!(report.contains("FAIL | sqlite_health"));
        assert!(report.contains("SUMMARY | failures=1 | warnings=0 | skipped=0"));
        assert!(!report.contains("C:\\Users"));
        assert!(!report.contains("API_KEY"));
    }
}
