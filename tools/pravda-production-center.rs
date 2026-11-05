//! Standalone Windows acceptance companion: one double-click, three embedded
//! release binaries, a local human-readable report, and no external runtime.
//! It NEVER installs/uninstalls software, uses exchange credentials, or places orders.
//! The desktop is launched with isolated paths and public market feeds disabled.
#![cfg_attr(windows, windows_subsystem = "windows")]
#![cfg_attr(not(windows), allow(dead_code))]
#[cfg(not(windows))]
compile_error!("pravda-production-center must be compiled for Windows");

use std::ffi::{c_void, OsStr};
use std::fmt::Write as FmtWrite;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::os::windows::process::CommandExt;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const APP: &[u8] = include_bytes!(env!("PRAVDA_EMBED_APP"));
const DOCTOR: &[u8] = include_bytes!(env!("PRAVDA_EMBED_DOCTOR"));
const HARNESS: &[u8] = include_bytes!(env!("PRAVDA_EMBED_HARNESS"));
const BUILD_COMMIT: &str = env!("PRAVDA_SOURCE_SHA");

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: *mut c_void,
        operation: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}

#[link(name = "user32")]
extern "system" {
    fn EnumWindows(callback: unsafe extern "system" fn(isize, isize) -> i32, param: isize) -> i32;
    fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    fn IsWindowVisible(hwnd: isize) -> i32;
}

struct WindowProbe {
    pid: u32,
    found: bool,
}
unsafe extern "system" fn find_window(hwnd: isize, param: isize) -> i32 {
    let probe = &mut *(param as *mut WindowProbe);
    let mut pid = 0_u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == probe.pid && IsWindowVisible(hwnd) != 0 {
        probe.found = true;
        return 0;
    }
    1
}
fn desktop_window_visible(pid: u32) -> bool {
    let mut probe = WindowProbe { pid, found: false };
    unsafe {
        EnumWindows(find_window, &mut probe as *mut WindowProbe as isize);
    }
    probe.found
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}
fn open_report(path: &Path) -> bool {
    let verb = wide(OsStr::new("open"));
    let file = wide(path.as_os_str());
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(),
            std::ptr::null(), std::ptr::null(), 1,
        ) > 32
    }
}

#[derive(Clone)]
struct Finding {
    name: &'static str,
    status: &'static str,
    detail: String,
}
struct Report {
    checks: Vec<Finding>,
    doctor: String,
    harness: String,
    notes: Vec<&'static str>,
}
impl Report {
    fn new() -> Self {
        Self {
            checks: vec![],
            doctor: String::new(),
            harness: String::new(),
            notes: vec![
                "This is a local acceptance report, not a security certification.",
                "No MSI/NSIS installation, WebView2 bootstrap, network air-gap, Authenticode, or GUI login flow is certified by this launcher.",
                "The desktop is only probed with temporary profile paths; real market feeds and live trading are not exercised.",
                "No results are transmitted. Review the report and attach the TXT to your support conversation manually.",
            ],
        }
    }
    fn add(&mut self, name: &'static str, status: &'static str, detail: impl Into<String>) {
        self.checks.push(Finding { name, status, detail: detail.into() });
    }
    fn summary(&self) -> (usize, usize, usize, usize) {
        (
            self.checks.iter().filter(|c| c.status == "PASS").count(),
            self.checks.iter().filter(|c| c.status == "WARN").count(),
            self.checks.iter().filter(|c| c.status == "FAIL").count(),
            self.checks.iter().filter(|c| c.status == "SKIP").count(),
        )
    }
    fn text(&self) -> String {
        let (pass, warn, fail, skip) = self.summary();
        let mut out = format!(
            "PRAVDA PRODUCTION CENTER / REPORT v1\n\
             Source commit: {BUILD_COMMIT}\n\
             Platform: Windows / x64\n\
             SUMMARY | PASS={pass} | WARN={warn} | FAIL={fail} | SKIP={skip}\n\
             \nCHECKS\n"
        );
        for c in &self.checks {
            let _ = writeln!(out, "{} | {} | {}", c.status, c.name, c.detail);
        }
        out.push_str("\nDOCTOR / ALLOWLISTED OUTPUT\n");
        if self.doctor.is_empty() { out.push_str("Not available\n"); }
        else { out.push_str(&self.doctor); out.push('\n'); }
        out.push_str("\nHARNESS / SYNTHETIC SUMMARY\n");
        if self.harness.is_empty() { out.push_str("Not available\n"); }
        else { out.push_str(&self.harness); out.push('\n'); }
        out.push_str("\nLIMITATIONS AND DATA SAFETY\n");
        for note in &self.notes { let _ = writeln!(out, "- {note}"); }
        out
    }
    fn html(&self, refreshing: bool) -> String {
        let (pass, warn, fail, skip) = self.summary();
        let mut html = String::from(
            "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\">\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
             <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'\">"
        );
        if refreshing {
            html.push_str("<meta http-equiv=\"refresh\" content=\"2\">");
        }
        html.push_str(
            "<title>Pravda · Centre de recette</title><style>\
             :root{color-scheme:dark}*{box-sizing:border-box}body{font:16px/1.55 Segoe UI,Arial,sans-serif;background:#0b1220;color:#dce8f6;margin:0}\
             main{max-width:1000px;margin:0 auto;padding:38px 20px 80px}\
             h1{font-size:31px;margin:0 0 4px;color:#e9f6ff}h2{font-size:19px;margin:28px 0 12px;color:#8fc7ff}\
             .subtitle{color:#9fb8d3}.panel{border:1px solid #263a52;border-radius:14px;background:#131f31;padding:18px;margin-top:17px}\
             .metrics{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px;margin-top:22px}\
             .metric{padding:15px;border-radius:12px;background:#152238;text-align:center}.metric strong{display:block;font-size:27px}\
             .pass{color:#63e0a0}.warn{color:#ffd06d}.fail{color:#ff8795}.skip{color:#9bb6d3}\
             .item{border-bottom:1px solid #283950;padding:12px 0}.item:last-child{border:0}\
             .tag{display:inline-block;font-size:12px;font-weight:800;letter-spacing:.04em;min-width:50px}\
             pre{white-space:pre-wrap;overflow-wrap:anywhere;font:12px/1.5 Consolas,monospace;color:#c5def3}\
             code{color:#8fc7ff}a{color:#8fc7ff}.small{font-size:13px;color:#9eb4c9}\
             @media(max-width:500px){.metrics{grid-template-columns:repeat(2,1fr)}}\
             </style></head><body><main><h1>🛡 Pravda · Centre de recette</h1>\
             <div class=\"subtitle\">Une seule application · Contrôles locaux · Aucune donnée transmise</div>"
        );
        if refreshing {
            html.push_str("<div class=\"panel warn\">⏳ Analyse en cours. Cette page se met à jour automatiquement.</div>");
        } else {
            html.push_str("<div class=\"panel pass\">✓ Analyse terminée. Le fichier TXT voisin peut être joint à votre conversation d'assistance.</div>");
        }
        let _ = write!(html,
            "<div class=\"metrics\"><div class=\"metric pass\"><strong>{pass}</strong>PASS</div>\
             <div class=\"metric warn\"><strong>{warn}</strong>WARN</div>\
             <div class=\"metric fail\"><strong>{fail}</strong>FAIL</div>\
             <div class=\"metric skip\"><strong>{skip}</strong>SKIP</div></div>"
        );
        html.push_str("<section class=\"panel\"><h2>Contrôles effectués</h2>");
        for c in &self.checks {
            let kind = match c.status { "PASS"=>"pass", "WARN"=>"warn", "FAIL"=>"fail", _=>"skip" };
            let _ = write!(html,
                "<div class=\"item\"><span class=\"tag {kind}\">{}</span> <b>{}</b>\
                 <div class=\"small\">{}</div></div>",
                c.status, escape_html(c.name), escape_html(&c.detail)
            );
        }
        html.push_str("</section><section class=\"panel\"><h2>Rapport technique détaillé</h2><pre>");
        html.push_str(&escape_html(&self.text()));
        html.push_str("</pre></section><p class=\"small\">Rapport lié au commit <code>");
        html.push_str(BUILD_COMMIT);
        html.push_str("</code>. Pour le partager, joindre le fichier <code>Pravda-Production-Report.txt</code> de ce dossier.</p></main></body></html>");
        html
    }
}
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        .replace('"', "&quot;").replace('\'', "&#39;")
}

fn output_dir(cli: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = cli { return dir; }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    let name = format!("Pravda-Report-{stamp}-{}", std::process::id());
    if let Some(home) = std::env::var_os("USERPROFILE") {
        let home = PathBuf::from(home);
        for place in ["Desktop", "Documents"] {
            let folder = home.join(place);
            if folder.is_dir() { return folder.join(&name); }
        }
    }
    std::env::temp_dir().join(name)
}
fn refuses_reparse(path: &Path) -> io::Result<()> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_attributes() & 0x400 != 0 {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "reparse_point_denied"));
        }
    }
    Ok(())
}
fn replace_file_safely(path: &Path, contents: &[u8]) -> io::Result<()> {
    // Create a fresh sibling and rename it over the destination: unlike
    // fs::write, the existing pathname is never opened and truncated.
    refuses_reparse(path)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default().as_nanos();
    let target_name = path.file_name().ok_or(io::ErrorKind::InvalidInput)?;
    let temp = path.with_file_name(format!(
        "{}-{stamp}-{}.tmp", target_name.to_string_lossy(), std::process::id()
    ));
    let mut handle = OpenOptions::new().create_new(true).write(true).open(&temp)?;
    let written = handle.write_all(contents).and_then(|_| handle.sync_all());
    drop(handle);
    if let Err(error) = written {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    let mut result = fs::rename(&temp, path);
    // Antivirus/HTML viewer can hold a short-lived open handle on Windows.
    for _ in 0..10 {
        if result.is_ok() { break; }
        sleep(Duration::from_millis(100));
        refuses_reparse(path)?;
        result = fs::rename(&temp, path);
    }
    if result.is_err() { let _ = fs::remove_file(temp); }
    result
}
fn persist(report: &Report, dir: &Path, working: bool) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    refuses_reparse(dir)?;
    replace_file_safely(&dir.join("Pravda-Production-Report.txt"), report.text().as_bytes())?;
    replace_file_safely(&dir.join("Pravda-Production-Report.html"), report.html(working).as_bytes())
}

fn extract(dir: &Path, name: &str, data: &[u8]) -> io::Result<PathBuf> {
    let path = dir.join(name);
    let mut f = OpenOptions::new().create_new(true).write(true).open(&path)?;
    f.write_all(data)?;
    f.sync_all()?;
    Ok(path)
}

fn run_capture(path: &Path, args: &[&str], limit: Duration) -> Result<String, &'static str> {
    const MAX_OUTPUT: usize = 96_000;
    let mut command = Command::new(path);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::piped())
        .stderr(Stdio::null());
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW, including Doctor and Harness.
    let mut child = command.spawn().map_err(|_| "START_FAILED")?;
    let mut stdout = child.stdout.take().ok_or("STDOUT_UNAVAILABLE")?;
    // Drain concurrently so a verbose child cannot deadlock the parent.
    // Bounded take() also prevents unbounded report memory consumption.
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = stdout.by_ref().take(MAX_OUTPUT as u64 + 1).read_to_end(&mut bytes);
        (bytes, read.is_ok())
    });
    let began = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if began.elapsed() < limit => sleep(Duration::from_millis(100)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let (bytes, readable) = reader.join().map_err(|_| "STDOUT_READER_FAILED")?;
    if bytes.len() > MAX_OUTPUT { return Err("OUTPUT_EXCEEDS_LIMIT"); }
    if !readable { return Err("READ_FAILED"); }
    let status = status.ok_or("TIMEOUT_OR_PROCESS_STATUS_FAILED")?;
    if !status.success() { return Err("PROCESS_EXIT_NONZERO"); }
    String::from_utf8(bytes).map_err(|_| "NON_UTF8_OUTPUT")
}

fn smoke_desktop(path: &Path, data_dir: &Path, ci: bool) -> Result<&'static str, &'static str> {
    let profile = data_dir.join("isolated-webview-profile");
    fs::create_dir(&profile).map_err(|_| "WEBVIEW_PROFILE_UNAVAILABLE")?;
    let mut cmd = Command::new(path);
    cmd.env("PRAVDA_DB_PATH", data_dir.join("market-test.db"))
        .env("PRAVDA_AUTH_PATH", data_dir.join("auth-test.json"))
        .env("PRAVDA_MARKET_FEEDS_ENABLED", "false")
        .env("PRAVDA_OFFLINE_TEST", "true")
        .env("PRAVDA_DIAGNOSTIC_WEBVIEW_DIR", &profile)
        .env_remove("PRAVDA_LEGACY_DB_PATH")
        .env_remove("MARKET_HISTORY_DB")
        .current_dir(data_dir)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    cmd.creation_flags(0x0800_0000); // Never open a console during the probe.
    let mut child = cmd.spawn().map_err(|_| "DESKTOP_START_FAILED")?;
    let duration = if ci { Duration::from_secs(4) } else { Duration::from_secs(10) };
    let began = Instant::now();
    let mut visible = false;
    let mut premature = false;
    while began.elapsed() < duration {
        match child.try_wait() {
            Ok(Some(_)) => { premature = true; break; }
            Ok(None) => { visible |= desktop_window_visible(child.id()); sleep(Duration::from_millis(200)); }
            Err(_) => { premature = true; break; }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    if premature { Err("DESKTOP_EXITED_EARLY") }
    else if visible { Ok("PROCESS_AND_VISIBLE_WINDOW_OK__LOGIN_NOT_AUTOMATED") }
    else { Err("DESKTOP_WINDOW_NOT_OBSERVED") }
}

fn doctor_allowed(raw: &str) -> String {
    let mut result = String::new();
    for line in raw.lines().take(125) {
        if ["PASS |", "WARN |", "FAIL |", "SKIP |", "SUMMARY |", "PRAVDA-DOCTOR REPORT"]
            .iter().any(|prefix| line.trim_start().starts_with(prefix)) {
            let clean = line.trim().chars().filter(|c| !c.is_control()).take(240).collect::<String>();
            let _ = writeln!(result, "{clean}");
        }
    }
    result
}

fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let ci = args.iter().any(|a| a == "--ci");
    let skip_desktop = args.iter().any(|a| a == "--skip-desktop");
    let custom_dir = args.windows(2).find(|w| w[0] == "--report-dir").map(|w| PathBuf::from(&w[1]));
    let report_dir = output_dir(custom_dir);
    let mut report = Report::new();
    report.add("Build provenance", "PASS", format!("Embedded binaries from commit {BUILD_COMMIT}"));
    if persist(&report, &report_dir, true).is_err() {
        return Err("REPORT_DIRECTORY_UNAVAILABLE");
    }
    if !ci {
        if !open_report(&report_dir.join("Pravda-Production-Report.html")) {
            let _ = Command::new("notepad.exe")
                .arg(report_dir.join("Pravda-Production-Report.txt")).spawn();
        }
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|_| "INVALID_SYSTEM_TIME")?.as_nanos();
    let staging = std::env::temp_dir().join(format!("pravda-center-{}-{stamp}", std::process::id()));
    if fs::create_dir(&staging).is_err() {
        report.add("Temporary sandbox", "FAIL", "Unable to allocate a private test directory");
        let _ = persist(&report, &report_dir, false);
        return Err("TEMP_DIRECTORY_UNAVAILABLE");
    }
    let binaries = [
        ("pravda-app.exe", APP), ("pravda-doctor.exe", DOCTOR),
        ("pravda-harness.exe", HARNESS)
    ];
    let mut embedded = Vec::new();
    for (name, bytes) in binaries {
        match extract(&staging, name, bytes) {
            Ok(path) => embedded.push(path),
            Err(_) => {
                report.add("Embedded binaries", "FAIL", format!("{name}: cannot extract"));
                let _ = persist(&report, &report_dir, false);
                let _ = fs::remove_dir_all(&staging);
                return Err("EMBEDDED_BINARY_EXTRACT_FAILED");
            }
        }
    }
    report.add("Embedded binaries", "PASS",
        "Three compiled Windows EXEs extracted inside a temporary directory");
    let _ = persist(&report, &report_dir, true);

    match run_capture(&embedded[1], &["--no-open"], Duration::from_secs(20)) {
        Ok(output) if output.contains("PRAVDA-DOCTOR REPORT v2") => {
            report.doctor = doctor_allowed(&output);
            report.add("Doctor diagnostic", "PASS", "Local passive report captured; see its individual PASS/WARN/FAIL/SKIP entries");
        }
        Ok(_) => report.add("Doctor diagnostic", "FAIL", "Expected report header is missing"),
        Err(reason) => report.add("Doctor diagnostic", "FAIL", reason),
    }
    let _ = persist(&report, &report_dir, true);

    match run_capture(&embedded[2], &["--smoke"], Duration::from_secs(35)) {
        Ok(output) => {
            let last = output.lines().last().unwrap_or("");
            if last.len() <= 24_000 && last.contains("\"build_accepted\":false")
                && last.contains("\"pass\":1")
                && last.contains("\"rejected\":1")
                && last.contains("\"crash\":1")
                && last.contains("\"timeout\":1") {
                report.harness = last.to_owned();
                report.add("Harness deterministic smoke", "PASS",
                    "Synthetic pass/reject/crash/timeout scenarios executed; build_accepted=false is intentional");
            } else {
                report.add("Harness deterministic smoke", "WARN",
                    "Execution returned but the expected JSON aggregate could not be validated");
            }
        }
        Err(reason) => report.add("Harness deterministic smoke", "FAIL", reason),
    }
    let _ = persist(&report, &report_dir, true);

    if skip_desktop {
        report.add("Desktop smoke", "SKIP", "Skipped explicitly for CI packaging validation");
    } else {
        match smoke_desktop(&embedded[0], &staging, ci) {
            Ok(message) => report.add("Desktop smoke", "PASS", message),
            Err(reason) => report.add("Desktop smoke", "WARN", reason),
        }
    }
    report.add("Installer/air-gap/signature", "SKIP",
        "Not tested here: MSI, NSIS, VM without WebView2, full GUI onboarding and Authenticode");
    let _ = fs::remove_dir_all(&staging);
    persist(&report, &report_dir, false).map_err(|_| "FINAL_REPORT_WRITE_FAILED")?;
    if !ci && !report_dir.join("Pravda-Production-Report.html").exists() {
        let _ = Command::new("notepad.exe")
            .arg(report_dir.join("Pravda-Production-Report.txt")).spawn();
    }
    Ok(())
}

fn main() {
    if let Err(problem) = run() {
        // A GUI-subsystem process must never vanish without a diagnostic.
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        let ci = args.iter().any(|a| a == "--ci");
        let custom = args.windows(2).find(|w| w[0] == "--report-dir")
            .map(|w| PathBuf::from(&w[1]));
        let dir = output_dir(custom);
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("Pravda-Production-Report.txt");
        let _ = fs::write(&file, format!(
            "PRAVDA PRODUCTION CENTER / STARTUP ERROR\n\
             Commit: {BUILD_COMMIT}\n\
             FAIL | launcher | {problem}\n\
             No credentials or production data were changed by this launcher.\n"
        ));
        if ci {
            std::process::exit(1);
        }
        let _ = Command::new("notepad.exe").arg(file).spawn();
    }
}


#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn scratch() -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pravda-center-selftest-{}-{stamp}", std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn report_publishing_does_not_truncate_existing_hardlink_target() {
        let base = scratch();
        let canary = base.join("canary.txt");
        fs::write(&canary, "SENTINEL_UNCHANGED").unwrap();
        let reports = base.join("reports");
        fs::create_dir(&reports).unwrap();
        fs::hard_link(&canary, reports.join("Pravda-Production-Report.txt")).unwrap();
        persist(&Report::new(), &reports, false).unwrap();
        assert_eq!(fs::read_to_string(&canary).unwrap(), "SENTINEL_UNCHANGED");
        assert!(fs::read_to_string(reports.join("Pravda-Production-Report.html"))
            .unwrap().contains("Centre de recette"));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn report_publishing_rejects_symlink_if_windows_can_create_it() {
        let base = scratch();
        let canary = base.join("canary.txt");
        fs::write(&canary, "SENTINEL_UNCHANGED").unwrap();
        let reports = base.join("reports");
        fs::create_dir(&reports).unwrap();
        let target = reports.join("Pravda-Production-Report.txt");
        if std::os::windows::fs::symlink_file(&canary, &target).is_ok() {
            assert!(persist(&Report::new(), &reports, false).is_err());
            assert_eq!(fs::read_to_string(&canary).unwrap(), "SENTINEL_UNCHANGED");
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn bounded_capture_rejects_child_excessive_stdout_without_pipe_deadlock() {
        let result = run_capture(
            Path::new("powershell.exe"),
            &["-NoProfile", "-NonInteractive", "-Command",
              "[Console]::Out.Write('X' * 150000)"],
            Duration::from_secs(20),
        );
        assert_eq!(result.unwrap_err(), "OUTPUT_EXCEEDS_LIMIT");
    }

    #[test]
    fn html_report_escapes_user_controlled_text() {
        let mut report = Report::new();
        report.add("Output encoding", "WARN", "<script>alert(1)</script>");
        let html = report.html(false);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
