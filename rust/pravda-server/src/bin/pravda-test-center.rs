#![cfg_attr(windows, windows_subsystem = "windows")]
// One Windows executable containing the desktop, Doctor and offline Harness.
// No installation, shell script, network connection or development runtime.
#[cfg(windows)]
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
#[cfg(windows)]
use std::{
    io,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
const DESKTOP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pravda-app.exe"));
#[cfg(windows)]
const DOCTOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pravda-doctor.exe"));
#[cfg(windows)]
const HARNESS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pravda-harness.exe"));

#[cfg(any(windows, test))]
#[derive(serde::Serialize)]
struct Finding {
    check: &'static str,
    status: &'static str,
    detail: String,
}
#[cfg(windows)]
#[derive(serde::Serialize)]
struct Report {
    schema_version: u32,
    product: &'static str,
    commit: &'static str,
    generated_unix: u64,
    signed_release: bool,
    findings: Vec<Finding>,
}

#[cfg(windows)]
fn checksum(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
#[cfg(any(windows, test))]
fn is_pe(bytes: &[u8]) -> bool {
    bytes.len() > 100_000 && bytes.starts_with(b"MZ")
}
#[cfg(any(windows, test))]
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
#[cfg(windows)]
fn save_binary(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()?;
    // Verify what will actually be executed, not just the embedded constant.
    if fs::read(path)?.as_slice() != bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Extracted executable does not match embedded payload",
        ));
    }
    Ok(())
}

/// Bounded, timeout-enforced capture: no transient console, no unbounded
/// stdout allocation or hung child when the output pipe fills up.
#[cfg(windows)]
fn run_bounded_capture(
    path: &Path,
    args: &[&str],
    folder: &Path,
) -> Result<std::process::Output, &'static str> {
    use std::io::Read;
    use std::os::windows::process::CommandExt;
    use std::sync::mpsc;
    use std::time::Instant;
    const MAX_OUTPUT: usize = 128 * 1024;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new(path);
    command
        .args(args)
        .current_dir(folder)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|_| "child_spawn_failed")?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("stdout_capture_unavailable");
    };
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout
            .take((MAX_OUTPUT + 1) as u64)
            .read_to_end(&mut output);
        let _ = tx.send((result, output));
    });
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut captured = None;
    loop {
        if captured.is_none() {
            match rx.try_recv() {
                Ok((Ok(_), bytes)) if bytes.len() <= MAX_OUTPUT => captured = Some(bytes),
                Ok((Ok(_), _)) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("child_output_limit_exceeded");
                }
                Ok((Err(_), _)) | Err(mpsc::TryRecvError::Disconnected) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("child_stdout_read_failed");
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = match captured {
                    Some(bytes) => bytes,
                    None => match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok((Ok(_), bytes)) if bytes.len() <= MAX_OUTPUT => bytes,
                        Ok((Ok(_), _)) => return Err("child_output_limit_exceeded"),
                        _ => return Err("child_stdout_read_failed"),
                    },
                };
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr: Vec::new(),
                });
            }
            Ok(None) => {}
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("child_wait_failed");
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("child_timeout");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
#[cfg(any(windows, test))]
fn finding(check: &'static str, status: &'static str, detail: impl Into<String>) -> Finding {
    Finding {
        check,
        status,
        detail: detail.into(),
    }
}
#[cfg(any(windows, test))]
fn safe_doctor_results(output: &str) -> Vec<Finding> {
    // Never copy arbitrary diagnostic output to the report: status and
    // check identifiers only. No paths, user names, secrets, or auth content.
    output
        .lines()
        .filter_map(|line| {
            let mut pieces = line.splitn(3, " | ");
            let status = match pieces.next()?.trim() {
                "PASS" => "PASS",
                "WARN" => "WARN",
                "FAIL" => "FAIL",
                "SKIP" => "SKIP",
                _ => return None,
            };
            let check = pieces.next()?.trim();
            if check.is_empty()
                || check.len() > 64
                || !check
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                return None;
            }
            Some(finding(
                "doctor_check",
                status,
                format!("{check}: see Doctor classification"),
            ))
        })
        .take(100)
        .collect()
}
#[cfg(windows)]
fn render_html(report: &Report) -> String {
    let mut rows = String::new();
    for f in &report.findings {
        rows.push_str(&format!(
            "<tr><td>{}</td><td><span class=\"{}\">{}</span></td><td>{}</td></tr>",
            html_escape(f.check),
            f.status.to_ascii_lowercase(),
            f.status,
            html_escape(&f.detail)
        ));
    }
    format!(
        r#"<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Pravda-Bot — Centre de test</title>
<style>body{{background:#0c1321;color:#e9f0fa;font:16px Segoe UI,Arial,sans-serif;max-width:1100px;margin:3rem auto;padding:0 1rem}}
h1{{color:#6ab9ff}}p{{line-height:1.6}}.tag{{background:#243b5c;padding:.5rem 1rem;border-radius:20px}}
table{{width:100%;border-collapse:collapse;background:#162338;border-radius:12px;overflow:hidden}}
td,th{{text-align:left;padding:1rem;border-bottom:1px solid #33445b}}.pass{{color:#5ee3a5}}.warn,.skip{{color:#ffcf76}}.fail{{color:#ff8494}}
a{{color:#8fc9ff}}code{{word-break:break-all}}</style></head><body>
<h1>PRAVDA / Centre de test</h1>
<p><span class="tag">Diagnostic local</span> <span class="tag">Aucun ordre réel</span> <span class="tag">Tests sans transmission</span></p>
<p>Commit source : <code>{}</code><br>Date Unix : {}<br>Signature Enterprise : <strong>non revendiquée</strong>.</p>
<table><thead><tr><th>Contrôle</th><th>État</th><th>Conclusion</th></tr></thead><tbody>{}</tbody></table>
<p><strong>Limites :</strong> un démarrage surveillé ne certifie pas les parcours GUI, la sélection MSI/NSIS,
l'absence de WebView2 sur VM neuve ni la signature Authenticode. Un résultat SKIP n'est jamais PASS.</p>
<p>Le rapport JSON voisin contient les mêmes résultats structurés. Il ne contient aucun mot de passe
ni donnée financière. Vérifiez-le avant de le partager avec un assistant.</p></body></html>"#,
        html_escape(report.commit),
        report.generated_unix,
        rows
    )
}

#[cfg(windows)]
fn message(text: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let title: Vec<u16> = "Pravda-Bot — Centre de test"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
#[cfg(not(windows))]
fn message(text: &str) {
    eprintln!("{text}");
}

#[cfg(windows)]
fn run() -> Result<PathBuf, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let base = if std::env::args().any(|arg| arg == "--ci") {
        std::env::var_os("PRAVDA_TEST_CENTER_REPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
    } else {
        std::env::temp_dir()
    };
    let folder = base.join(format!("pravda-test-center-{}-{now}", std::process::id()));
    fs::create_dir(&folder).map_err(|_| "Cannot create isolated test folder")?;
    let mut report = Report {
        schema_version: 1,
        product: "Pravda-Bot",
        commit: option_env!("PRAVDA_BUILD_COMMIT").unwrap_or("UNKNOWN"),
        generated_unix: now,
        signed_release: false,
        findings: Vec::new(),
    };
    // The tester only accepts embedded Windows PE payloads. It must not
    // silently pass when built by a plain cargo check without release inputs.
    if ![DESKTOP, DOCTOR, HARNESS].iter().all(|bytes| is_pe(bytes)) {
        report.findings.push(finding(
            "bundle_integrity",
            "FAIL",
            "One or more release executables are absent or invalid",
        ));
    } else {
        let app = folder.join("pravda-app.exe");
        let doctor = folder.join("pravda-doctor.exe");
        let harness = folder.join("pravda-harness.exe");
        let payloads = [(&app, DESKTOP), (&doctor, DOCTOR), (&harness, HARNESS)];
        for (file, bytes) in payloads {
            match save_binary(file, bytes) {
                Ok(()) => report.findings.push(finding(
                    "embedded_sha256",
                    "PASS",
                    format!(
                        "{}: {}",
                        file.file_name().unwrap_or_default().to_string_lossy(),
                        checksum(bytes)
                    ),
                )),
                Err(_) => {
                    let _ = fs::remove_file(file);
                    report.findings.push(finding(
                        "payload_extract",
                        "FAIL",
                        "Cannot verify the extracted executable in the isolated test folder",
                    ));
                }
            }
        }
        if doctor.is_file() {
            match run_bounded_capture(&doctor, &["--no-open"], &folder) {
                Ok(output) => {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let items = safe_doctor_results(&text);
                    let has_header = text.contains("PRAVDA-DOCTOR REPORT v2");
                    report.findings.push(finding(
                        "doctor_startup",
                        if output.status.success() && has_header {
                            "PASS"
                        } else {
                            "FAIL"
                        },
                        if has_header {
                            "Doctor completed passive checks"
                        } else {
                            "Doctor did not return expected report"
                        },
                    ));
                    report.findings.extend(items);
                }
                Err(reason) => report.findings.push(finding(
                    "doctor_startup",
                    "FAIL",
                    format!("Doctor execution failed: {reason}"),
                )),
            }
        }
        if harness.is_file() {
            match run_bounded_capture(&harness, &["--smoke"], &folder) {
                Ok(output) => {
                    let value = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .last()
                        .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok());
                    let count = value
                        .as_ref()
                        .and_then(|v| v.pointer("/aggregate/pass"))
                        .and_then(|v| v.as_u64())
                        == Some(1);
                    let rejected = value
                        .as_ref()
                        .and_then(|v| v.pointer("/aggregate/rejected"))
                        .and_then(|v| v.as_u64())
                        == Some(1);
                    let timeout = value
                        .as_ref()
                        .and_then(|v| v.pointer("/aggregate/timeout"))
                        .and_then(|v| v.as_u64())
                        == Some(1);
                    let crash = value
                        .as_ref()
                        .and_then(|v| v.pointer("/aggregate/crash"))
                        .and_then(|v| v.as_u64())
                        == Some(1);
                    let accepted = value
                        .as_ref()
                        .and_then(|v| v.get("build_accepted"))
                        .and_then(|v| v.as_bool())
                        == Some(false);
                    report.findings.push(finding(
                        "harness_smoke",
                        if output.status.success()
                            && count
                            && rejected
                            && timeout
                            && crash
                            && accepted
                        {
                            "PASS"
                        } else {
                            "FAIL"
                        },
                        "Synthetic pass/refusal/timeout/crash; build_accepted=false is expected",
                    ));
                }
                Err(reason) => report.findings.push(finding(
                    "harness_smoke",
                    "FAIL",
                    format!("Harness execution failed: {reason}"),
                )),
            }
        }
        // Always run under synthetic isolated profiles. Never touch existing
        // user databases or enable live network feeds/financial operations.
        if app.is_file() {
            let db = folder.join("synthetic.db");
            let auth = folder.join("synthetic-auth.json");
            // Inherit only Windows runtime prerequisites, never credentials
            // or legacy migration paths from the user's real desktop profile.
            let mut desktop_command = Command::new(&app);
            desktop_command.env_clear();
            for key in [
                "SystemRoot",
                "WINDIR",
                "PATH",
                "COMSPEC",
                "PATHEXT",
                "ProgramFiles",
                "ProgramFiles(x86)",
                "PROGRAMDATA",
            ] {
                if let Some(value) = std::env::var_os(key) {
                    desktop_command.env(key, value);
                }
            }
            let local = folder.join("AppData").join("Local");
            let roaming = folder.join("AppData").join("Roaming");
            let _ = fs::create_dir_all(&local);
            let _ = fs::create_dir_all(&roaming);
            let child = desktop_command
                .current_dir(&folder)
                .env("LOCALAPPDATA", &local)
                .env("APPDATA", &roaming)
                .env("USERPROFILE", &folder)
                .env("TEMP", &folder)
                .env("TMP", &folder)
                .env("PRAVDA_DB_PATH", &db)
                .env("PRAVDA_AUTH_PATH", &auth)
                .env("PRAVDA_MARKET_FEEDS_ENABLED", "false")
                .env("PRAVDA_LIVE_TRADING_ENABLED", "false")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            match child {
                Ok(mut process) => {
                    let mut early_exit = false;
                    for _ in 0..12 {
                        std::thread::sleep(Duration::from_millis(500));
                        match process.try_wait() {
                            Ok(Some(_)) => {
                                early_exit = true;
                                break;
                            }
                            Ok(None) => {}
                            Err(_) => {
                                early_exit = true;
                                break;
                            }
                        }
                    }
                    if !early_exit {
                        let _ = process.kill();
                    }
                    let _ = process.wait();
                    report.findings.push(finding(
                        "desktop_process",
                        if early_exit { "FAIL" } else { "PASS" },
                        if early_exit {
                            "Desktop process terminated before 6-second startup window"
                        } else {
                            "Desktop stayed alive for 6 seconds; GUI interaction not certified"
                        },
                    ));
                    report.findings.push(finding("desktop_gui","SKIP",
                        "No automated interactive login or WebView2 absence assertion in this local test center"));
                }
                Err(_) => report.findings.push(finding(
                    "desktop_process",
                    "FAIL",
                    "Windows could not launch the bundled desktop",
                )),
            }
        }
        for file in [&app, &doctor, &harness] {
            let _ = fs::remove_file(file);
        }
    }
    report.findings.push(finding(
        "msi_nsis_clean_vm",
        "SKIP",
        "Requires independent air-gapped Windows VM, not the user's physical machine",
    ));
    report.findings.push(finding(
        "enterprise_signature",
        "SKIP",
        "Unsigned test build; no code-signing certificate assumed",
    ));
    let json = folder.join("PRAVDA-TEST-REPORT.json");
    let html = folder.join("PRAVDA-TEST-REPORT.html");
    fs::write(
        &json,
        serde_json::to_vec_pretty(&report).map_err(|_| "Cannot serialize report")?,
    )
    .map_err(|_| "Cannot save report JSON")?;
    fs::write(&html, render_html(&report)).map_err(|_| "Cannot save report HTML")?;
    if !std::env::args().any(|arg| arg == "--ci") {
        let _ = Command::new("explorer.exe").arg(&html).spawn();
    }
    Ok(folder)
}
#[cfg(not(windows))]
fn run() -> Result<PathBuf, String> {
    Err("Windows x64 only".into())
}

fn main() {
    let ci = std::env::args().any(|arg| arg == "--ci");
    match run() {
        Ok(folder) => {
            if ci {
                let path = folder.join("PRAVDA-TEST-REPORT.json");
                println!("PRAVDA_TEST_REPORT={}", path.display());
                let failed = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                    .and_then(|report| {
                        report["findings"]
                            .as_array()
                            .map(|rows| rows.iter().any(|row| row["status"] == "FAIL"))
                    })
                    .unwrap_or(true);
                if failed {
                    eprintln!("PRAVDA_TEST_FAILED: see structured JSON report");
                    std::process::exit(2);
                }
            } else {
                message(&format!("Tests terminés.\n\nRapport HTML et JSON :\n{}\n\nTu peux partager uniquement le JSON après vérification.",folder.display()));
            }
        }
        Err(error) => {
            if ci {
                eprintln!("PRAVDA_TEST_ERROR={error}");
                std::process::exit(1);
            }
            message(&format!("Les tests n'ont pas démarré : {error}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_escapes_html_and_rejects_untrusted_doctor_lines() {
        assert_eq!(html_escape("<>&"), "&lt;&gt;&amp;");
        let r = safe_doctor_results(
            "PASS | sqlite_health | C:\\Users\\Private\nPRIVATE=SECRET\nFAIL | bad/name | path",
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].detail, "sqlite_health: see Doctor classification");
        assert!(!r[0].detail.contains("Private"));
    }
    #[test]
    fn rejects_incomplete_portable_payloads() {
        assert!(!is_pe(b""));
        assert!(!is_pe(b"MZ"));
    }
}
