//! One-click, no-secrets diagnostic companion for Pravda-Bot.
//!
//! This executable has no exchange SDK, no credentials, no trading transport,
//! no HTTP listener and no automatic repair. Run it directly from Explorer.
//! The report is intentionally allowlisted: never print raw OS errors, paths,
//! environment variables, SQLite contents, machine names or process lists.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    process::Command,
};

#[derive(Clone, Copy)]
enum Status {
    Pass,
    Warn,
    Fail,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

struct Check {
    code: &'static str,
    status: Status,
    advice: &'static str,
}

impl Check {
    fn new(code: &'static str, status: Status, advice: &'static str) -> Self {
        Self {
            code,
            status,
            advice,
        }
    }
}

fn inspect_file(path: &Path, label: &'static str) -> Check {
    match fs::symlink_metadata(path) {
        Ok(data) if data.is_file() && !data.file_type().is_symlink() => {
            Check::new(label, Status::Pass, "File present")
        }
        Ok(_) => Check::new(label, Status::Fail, "Expected regular file is unavailable"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Check::new(label, Status::Warn, "File not found; installation or setup may be incomplete")
        }
        Err(_) => Check::new(label, Status::Warn, "File metadata unavailable"),
    }
}

fn report_lines(checks: &[Check]) -> String {
    let mut result = String::from(
        "PRAVDA-DOCTOR REPORT v1\n\
         Copy only this report into your support conversation.\n\
         SAFE MODE: no repairs, exchange connections, orders or credentials.\n\
         No filesystem paths, account IDs or logs collected.\n\n",
    );
    for check in checks {
        result.push_str(check.status.label());
        result.push_str(" | ");
        result.push_str(check.code);
        result.push_str(" | ");
        result.push_str(check.advice);
        result.push('\n');
    }
    let failed = checks.iter().filter(|c| matches!(c.status, Status::Fail)).count();
    let warned = checks.iter().filter(|c| matches!(c.status, Status::Warn)).count();
    result.push_str(&format!("SUMMARY | failures={failed} | warnings={warned}\n"));
    result
}

#[cfg(windows)]
fn checks() -> Vec<Check> {
    let mut checks = vec![Check::new(
        "windows_os",
        Status::Pass,
        "Windows native diagnostic executable running",
    )];

    let root = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .map(|p| p.join("Pravda-Bot"));
    match root {
        Some(ref root) => {
            let auth = inspect_file(&root.join("pravda_auth.json"), "local_auth_file");
            checks.push(auth);

            let db_path = root.join("pravda.db");
            use pravda_store::doctor::{inspect_database, DatabaseHealth};
            let db_health = inspect_database(&db_path);
            let (state, description) = match db_health {
                DatabaseHealth::Healthy => (Status::Pass, "SQLite integrity and ticks schema OK"),
                DatabaseHealth::Missing => (Status::Warn, "SQLite missing; start Pravda-Bot once"),
                DatabaseHealth::NotAFile => (Status::Fail, "SQLite path is not a regular file"),
                DatabaseHealth::Unavailable => {
                    (Status::Warn, "SQLite inaccessible; inspect file permissions or lock")
                }
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

    let application = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("pravda-app.exe")));
    checks.push(match application {
        Some(path) => inspect_file(&path, "adjacent_app_binary"),
        None => Check::new(
            "adjacent_app_binary",
            Status::Warn,
            "Cannot inspect companion application",
        ),
    });

    checks.push(Check::new(
        "revolut_exchange",
        Status::Warn,
        "Not tested offline: no credential or trading access by design",
    ));
    checks.push(Check::new(
        "installer_signature",
        Status::Warn,
        "Not attested by this probe; signed release validation is separate",
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
            Check::new("sqlite_health", Status::Fail, "SQLite integrity check failed"),
        ]);
        assert!(report.contains("PASS | local_auth_file"));
        assert!(report.contains("FAIL | sqlite_health"));
        assert!(report.contains("SUMMARY | failures=1 | warnings=0"));
        assert!(!report.contains("C:\\Users"));
        assert!(!report.contains("API_KEY"));
    }
}
