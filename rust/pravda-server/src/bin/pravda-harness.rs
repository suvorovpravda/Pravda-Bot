//! Native Tokio orchestration for deterministic offline Rust replay binaries.
//! No shell, no Python/Node, no data upload, no exchange/API HTTP calls.
//!
//! A compiler/runtime cannot enforce air-gap isolation by itself. The host
//! MUST deny all outbound network traffic at the OS/hypervisor boundary.
//! Every child binary is SHA-256 allowlisted and started with a minimal env.
#[path = "../harness_metrics.rs"]
mod harness_metrics;

use harness_metrics::{
    validate_build, validate_with_limits, Aggregate, GateLimits, Metrics, ReasonCount, Summary,
    SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{mpsc, Arc},
    thread,
    time::{Duration, Instant},
};
use tokio::{sync::Semaphore, task::JoinSet};

const MAX_MANIFEST_BYTES: u64 = 65_536;
const MAX_STDOUT_BYTES: usize = 16_384;
const MAX_STDERR_BYTES: usize = 16_384;
const MAX_TOTAL_RUNS: u32 = 5_000;
const MAX_CONCURRENCY: usize = 64;
const READER_FINISH_MS: u64 = 300;
const MAX_CHILD_RSS_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TestCase {
    id: String,
    binary: String,
    expected_sha256: String,
    repetitions: u32,
    timeout_ms: u64,
    args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u16,
    concurrency: usize,
    gates: GateLimits,
    cases: Vec<TestCase>,
}

#[derive(Clone, Copy)]
enum ResultKind {
    Pass,
    Rejected,
    Timeout,
    Crash,
    MemoryExceeded,
    Invalid,
}

struct ResultRow {
    kind: ResultKind,
    reason: &'static str,
    metric: Option<Metrics>,
}

impl ResultRow {
    fn invalid(reason: &'static str) -> Self {
        Self {
            kind: ResultKind::Invalid,
            reason,
            metric: None,
        }
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut handle = fs::File::open(path)?;
    let mut state = Sha256::new();
    let mut buf = [0_u8; 65_536];
    loop {
        let len = handle.read(&mut buf)?;
        if len == 0 {
            break;
        }
        state.update(&buf[..len]);
    }
    let digest = state.finalize();
    let mut result = String::with_capacity(64);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &byte in digest.iter() {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(result)
}

fn check_manifest(manifest: &Manifest, bin_dir: &Path) -> Result<Vec<PathBuf>, &'static str> {
    if manifest.schema_version != SCHEMA_VERSION
        || manifest.concurrency == 0
        || manifest.concurrency > MAX_CONCURRENCY
        || manifest.cases.is_empty()
    {
        return Err("invalid_manifest");
    }
    let total = manifest
        .cases
        .iter()
        .try_fold(0_u32, |sum, case| sum.checked_add(case.repetitions));
    if total.is_none_or(|n| n == 0 || n > MAX_TOTAL_RUNS) {
        return Err("invalid_run_count");
    }
    // Hard-coded desk envelope is a safety FLOOR. A manifest may tighten
    // requirements but cannot dilute the mandatory production gate.
    let hard = GateLimits::default();
    if manifest.gates.min_trades < hard.min_trades
        || manifest.gates.min_net_pnl_cents < hard.min_net_pnl_cents
        || manifest.gates.min_sharpe < hard.min_sharpe
        || manifest.gates.min_sortino < hard.min_sortino
        || manifest.gates.min_calmar < hard.min_calmar
        || manifest.gates.max_drawdown_bps > hard.max_drawdown_bps
        || manifest.gates.max_var_99_bps > hard.max_var_99_bps
        || manifest.gates.max_slippage_bps > hard.max_slippage_bps
        || manifest.gates.max_latency_p99_us > hard.max_latency_p99_us
        || ![
            manifest.gates.min_sharpe,
            manifest.gates.min_sortino,
            manifest.gates.min_calmar,
            manifest.gates.max_slippage_bps,
        ]
        .iter()
        .all(|n| n.is_finite())
        || manifest.gates.min_trades == 0
        || manifest.gates.max_latency_p99_us == 0
        || manifest.gates.max_slippage_bps < 0.0
    {
        return Err("invalid_gate_profile");
    }

    let base = fs::canonicalize(bin_dir).map_err(|_| "binary_directory_unavailable")?;
    let mut paths = Vec::with_capacity(manifest.cases.len());
    for case in &manifest.cases {
        if !valid_identifier(&case.id)
            || case.repetitions == 0
            || case.timeout_ms < 100
            || case.timeout_ms > 300_000
            || case.args.len() > 16
            || case
                .args
                .iter()
                .any(|s| s.len() > 128 || s.chars().any(char::is_control))
            || case.expected_sha256.len() != 64
            || !case.expected_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("unsafe_case_configuration");
        }
        let name = Path::new(&case.binary);
        if name.is_absolute()
            || name.file_name().is_none()
            || name.components().count() != 1
            || (cfg!(windows) && name.extension().is_none_or(|e| e != "exe"))
        {
            return Err("unsafe_binary_name");
        }
        let candidate = fs::canonicalize(base.join(name)).map_err(|_| "test_binary_unavailable")?;
        if candidate.parent() != Some(base.as_path()) || !candidate.is_file() {
            return Err("binary_outside_allowlist");
        }
        let digest = hash_file(&candidate).map_err(|_| "binary_unreadable")?;
        if !digest.eq_ignore_ascii_case(&case.expected_sha256) {
            return Err("binary_sha256_mismatch");
        }
        paths.push(candidate);
    }
    Ok(paths)
}

/// Drains a pipe without unbounded allocation, even for a noisy child.
/// A bounded mailbox avoids blocking the supervisor if a descendant retains
/// an inherited pipe after the original process is killed.
fn drain_pipe<R: Read + Send + 'static>(mut source: R, max: usize) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut data = Vec::with_capacity(max.min(4_096));
        let mut chunk = [0_u8; 8_192];
        loop {
            match source.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let remaining = max.saturating_sub(data.len());
                    data.extend_from_slice(&chunk[..remaining.min(n)]);
                    // Continue draining after hitting the cap to prevent the
                    // subprocess from deadlocking on a full OS pipe.
                }
                Err(_) => break,
            }
        }
        let _ = tx.send(data);
    });
    rx
}

#[cfg(target_os = "linux")]
fn current_rss_bytes(pid: u32) -> Option<u64> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kb = status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
    })?;
    kb.checked_mul(1024)
}

#[cfg(windows)]
fn current_rss_bytes(pid: u32) -> Option<u64> {
    use std::mem;
    use windows_sys::Win32::{
        System::{
            ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
            Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
        },
        Foundation::CloseHandle,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut counters: PROCESS_MEMORY_COUNTERS = mem::zeroed();
        let ok = GetProcessMemoryInfo(
            handle,
            &mut counters as *mut PROCESS_MEMORY_COUNTERS,
            mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        );
        CloseHandle(handle);
        if ok == 0 {
            None
        } else {
            Some(counters.WorkingSetSize as u64)
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn current_rss_bytes(_pid: u32) -> Option<u64> {
    None
}

fn run_one(case: &TestCase, path: &Path, index: u32, limits: GateLimits) -> ResultRow {
    let run_id = format!("{}_{}", case.id, index);
    let mut cmd = Command::new(path);
    cmd.args(&case.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("PRAVDA_RUN_ID", &run_id)
        .env("PRAVDA_OFFLINE_TEST", "true")
        .env("PRAVDA_MARKET_FEEDS_ENABLED", "false")
        .env("PRAVDA_LIVE_TRADING_ENABLED", "false")
        .env("PRAVDA_REVOLUT_X_ENABLED", "false")
        .env("RUST_BACKTRACE", "0");
    // Preserve only the Windows loader's minimal OS requirement; do not
    // inherit USERPROFILE, API keys, proxies or exchange configuration.
    #[cfg(windows)]
    if let Some(root) = env::var_os("SystemRoot") {
        cmd.env("SystemRoot", root);
    }
    if let Some(parent) = path.parent() {
        cmd.current_dir(parent);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(_) => return ResultRow::invalid("spawn_failed"),
    };
    let stdout = match child.stdout.take() {
        Some(out) => drain_pipe(out, MAX_STDOUT_BYTES + 1),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return ResultRow::invalid("stdout_unavailable");
        }
    };
    let stderr = match child.stderr.take() {
        Some(err) => drain_pipe(err, MAX_STDERR_BYTES + 1),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return ResultRow::invalid("stderr_unavailable");
        }
    };

    let started = Instant::now();
    let deadline = Duration::from_millis(case.timeout_ms);
    let mut memory_exceeded = false;
    let outcome = loop {
        if current_rss_bytes(child.id()).is_some_and(|rss| rss > MAX_CHILD_RSS_BYTES) {
            memory_exceeded = true;
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status.success()),
            Ok(None) if started.elapsed() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break Some(false);
            }
        }
    };
    let output = stdout.recv_timeout(Duration::from_millis(READER_FINISH_MS));
    let _ = stderr.recv_timeout(Duration::from_millis(READER_FINISH_MS));
    if memory_exceeded {
        return ResultRow {
            kind: ResultKind::MemoryExceeded,
            reason: "child_memory_cap_exceeded",
            metric: None,
        };
    }
    match outcome {
        None => ResultRow {
            kind: ResultKind::Timeout,
            reason: "deadline_exceeded",
            metric: None,
        },
        Some(false) => ResultRow {
            kind: ResultKind::Crash,
            reason: "child_failed_or_panicked",
            metric: None,
        },
        Some(true) => {
            let bytes = match output {
                Ok(bytes) if bytes.len() <= MAX_STDOUT_BYTES => bytes,
                _ => return ResultRow::invalid("missing_or_excessive_telemetry"),
            };
            let metric: Metrics = match serde_json::from_slice(&bytes) {
                Ok(parsed) => parsed,
                Err(_) => return ResultRow::invalid("invalid_metrics_json"),
            };
            if metric.run_id != run_id {
                return ResultRow::invalid("metric_run_identity_mismatch");
            }
            match validate_build(&metric).and_then(|()| validate_with_limits(&metric, &limits)) {
                Ok(()) => ResultRow {
                    kind: ResultKind::Pass,
                    reason: "accepted",
                    metric: Some(metric),
                },
                Err(e) => ResultRow {
                    kind: ResultKind::Rejected,
                    reason: e.code(),
                    metric: Some(metric),
                },
            }
        }
    }
}

fn aggregate(rows: Vec<ResultRow>) -> Summary {
    let runs = rows.len() as u32;
    let mut a = Aggregate {
        runs,
        pass: 0,
        rejected: 0,
        timeout: 0,
        crash: 0,
        memory_exceeded: 0,
        invalid: 0,
        gross_pnl_cents_total: 0,
        net_pnl_cents_total: 0,
        worst_max_drawdown_bps: 0,
        worst_var_99_bps: 0,
        worst_latency_p99_us: 0,
        mean_latency_p99_us: 0.0,
        mean_sharpe: 0.0,
        mean_sortino: 0.0,
        mean_calmar: 0.0,
        mean_slippage_bps: 0.0,
    };
    let mut reasons = BTreeMap::<String, u32>::new();
    let mut observed = 0_u32;
    for row in rows {
        match row.kind {
            ResultKind::Pass => a.pass += 1,
            ResultKind::Rejected => a.rejected += 1,
            ResultKind::Timeout => a.timeout += 1,
            ResultKind::Crash => a.crash += 1,
            ResultKind::MemoryExceeded => a.memory_exceeded += 1,
            ResultKind::Invalid => a.invalid += 1,
        }
        if !matches!(row.kind, ResultKind::Pass) {
            *reasons.entry(row.reason.into()).or_insert(0) += 1;
        }
        if let Some(m) = row.metric {
            observed += 1;
            a.gross_pnl_cents_total += i128::from(m.gross_pnl_cents);
            a.net_pnl_cents_total += i128::from(m.net_pnl_cents);
            a.worst_max_drawdown_bps = a.worst_max_drawdown_bps.max(m.max_drawdown_bps);
            a.worst_var_99_bps = a.worst_var_99_bps.max(m.var_99_bps);
            a.worst_latency_p99_us = a.worst_latency_p99_us.max(m.latency_p99_us);
            a.mean_latency_p99_us += m.latency_p99_us as f64;
            a.mean_sharpe += m.sharpe;
            a.mean_sortino += m.sortino;
            a.mean_calmar += m.calmar;
            a.mean_slippage_bps += m.slippage_bps;
        }
    }
    if observed > 0 {
        let denominator = f64::from(observed);
        a.mean_latency_p99_us /= denominator;
        a.mean_sharpe /= denominator;
        a.mean_sortino /= denominator;
        a.mean_calmar /= denominator;
        a.mean_slippage_bps /= denominator;
    }
    Summary {
        schema_version: SCHEMA_VERSION,
        build_accepted: runs > 0 && a.pass == runs,
        aggregate: a,
        rejected_reasons: reasons
            .into_iter()
            .map(|(code, count)| ReasonCount { code, count })
            .collect(),
    }
}

async fn orchestrate(manifest: Manifest, paths: Vec<PathBuf>) -> Summary {
    let count: usize = manifest.cases.iter().map(|c| c.repetitions as usize).sum();
    let permits = Arc::new(Semaphore::new(manifest.concurrency));
    let mut tasks = JoinSet::new();
    for (case, path) in manifest.cases.into_iter().zip(paths) {
        for index in 0..case.repetitions {
            let permit = permits
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore open");
            let one = case.clone();
            let binary = path.clone();
            let gates = manifest.gates;
            tasks.spawn(async move {
                let _permit = permit;
                tokio::task::spawn_blocking(move || run_one(&one, &binary, index, gates)).await
            });
        }
    }
    let mut rows = Vec::with_capacity(count);
    while let Some(outcome) = tasks.join_next().await {
        rows.push(match outcome {
            Ok(Ok(row)) => row,
            _ => ResultRow::invalid("harness_task_failed"),
        });
    }
    aggregate(rows)
}

fn sample_metric() -> Metrics {
    let run_id = env::var("PRAVDA_RUN_ID").unwrap_or_else(|_| "smoke_0".into());
    Metrics {
        schema_version: SCHEMA_VERSION,
        run_id,
        gross_pnl_cents: 12_000,
        fees_cents: 2_000,
        net_pnl_cents: 10_000,
        max_drawdown_bps: 250,
        sharpe: 1.2,
        sortino: 1.4,
        calmar: 1.1,
        wins: 120,
        losses: 80,
        trades: 200,
        slippage_bps: 2.0,
        latency_p50_us: 500,
        latency_p99_us: 2_000,
        var_99_bps: 120,
    }
}

fn fixture(mode: &str) -> ! {
    match mode {
        "--fixture-ok" | "--fixture-bad-risk" => {
            let mut m = sample_metric();
            if mode == "--fixture-bad-risk" {
                m.max_drawdown_bps = 2_000;
            }
            println!("{}", serde_json::to_string(&m).expect("serialize fixture"));
            std::process::exit(0);
        }
        "--fixture-crash" => std::process::exit(17),
        "--fixture-timeout" => {
            thread::sleep(Duration::from_secs(5));
            std::process::exit(0);
        }
        _ => std::process::exit(64),
    }
}

fn load_manifest(path: &Path) -> Result<Manifest, &'static str> {
    if fs::metadata(path)
        .map_err(|_| "manifest_unavailable")?
        .len()
        > MAX_MANIFEST_BYTES
    {
        return Err("manifest_too_large");
    }
    let bytes = fs::read(path).map_err(|_| "manifest_unreadable")?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid_manifest_json")
}

async fn smoke() -> Result<(), &'static str> {
    let own = env::current_exe().map_err(|_| "harness_executable_missing")?;
    let parent = own.parent().ok_or("harness_executable_directory_missing")?;
    let binary = own
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid_binary_name")?;
    let hash = hash_file(&own).map_err(|_| "hash_failure")?;
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        concurrency: 3,
        gates: GateLimits::default(),
        cases: ["--fixture-ok", "--fixture-bad-risk", "--fixture-timeout"]
            .iter()
            .enumerate()
            .map(|(n, arg)| TestCase {
                id: format!("smoke{n}"),
                binary: binary.into(),
                expected_sha256: hash.clone(),
                repetitions: 1,
                timeout_ms: if *arg == "--fixture-timeout" {
                    150
                } else {
                    5_000
                },
                args: vec![(*arg).into()],
            })
            .collect(),
    };
    let binaries = check_manifest(&manifest, parent)?;
    let summary = orchestrate(manifest, binaries).await;
    println!(
        "{}",
        serde_json::to_string(&summary).map_err(|_| "serialization_failed")?
    );
    if summary.aggregate.pass == 1
        && summary.aggregate.rejected == 1
        && summary.aggregate.timeout == 1
        && !summary.build_accepted
    {
        Ok(())
    } else {
        Err("smoke_assertion_failed")
    }
}

/// Generate the fixture manifest with an exact SHA-256 allowlist.
/// No shell, network, or external runtime; never overwrite an existing file.
fn prepare_manifest(
    bin_dir: &Path,
    binary: &str,
    repetitions: u32,
    output: &Path,
) -> Result<(), &'static str> {
    let root = fs::canonicalize(bin_dir).map_err(|_| "binary_directory_unavailable")?;
    let name = Path::new(binary);
    if name.components().count() != 1 || name.file_name().is_none() {
        return Err("unsafe_binary_name");
    }
    let candidate = fs::canonicalize(root.join(name)).map_err(|_| "test_binary_unavailable")?;
    if candidate.parent() != Some(root.as_path()) || !candidate.is_file() {
        return Err("binary_outside_allowlist");
    }
    let hash = hash_file(&candidate).map_err(|_| "hash_failure")?;
    let concurrency = thread::available_parallelism().map_or(4, |n| n.get().clamp(1, 16));
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        concurrency,
        gates: GateLimits::default(),
        cases: vec![TestCase {
            id: "replay".into(),
            binary: binary.into(),
            expected_sha256: hash,
            repetitions,
            timeout_ms: 30_000,
            args: Vec::new(),
        }],
    };
    let _ = check_manifest(&manifest, &root)?;
    let data = serde_json::to_vec_pretty(&manifest).map_err(|_| "serialization_failed")?;
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|_| "manifest_output_unavailable")?;
    file.write_all(&data).map_err(|_| "manifest_write_failed")?;
    file.flush().map_err(|_| "manifest_write_failed")?;
    Ok(())
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && args[0].starts_with("--fixture-") {
        fixture(&args[0]);
    }
    let result = if args.len() == 1 && args[0] == "--smoke" {
        smoke().await
    } else if args.len() == 5 && args[0] == "prepare" {
        match args[3].parse::<u32>() {
            Ok(repetitions) => {
                let created = prepare_manifest(
                    Path::new(&args[1]),
                    &args[2],
                    repetitions,
                    Path::new(&args[4]),
                );
                if created.is_ok() {
                    println!("MANIFEST_READY");
                }
                created
            }
            Err(_) => Err("invalid_run_count"),
        }
    } else if args.len() == 3 && args[0] == "run" {
        let parsed = load_manifest(Path::new(&args[1]));
        match parsed {
            Ok(manifest) => match check_manifest(&manifest, Path::new(&args[2])) {
                Ok(paths) => {
                    let summary = orchestrate(manifest, paths).await;
                    match serde_json::to_string(&summary) {
                        Ok(serialized) => {
                            println!("{serialized}");
                            if summary.build_accepted {
                                Ok(())
                            } else {
                                Err("build_rejected")
                            }
                        }
                        Err(_) => Err("serialization_failed"),
                    }
                }
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    } else {
        Err("usage: pravda-harness prepare <bin_dir> <exe> <N> <manifest.json> | run <manifest.json> <bin_dir> | --smoke")
    };
    if let Err(reason) = result {
        eprintln!("HARNESS_ERROR:{reason}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_all_types_without_exposing_raw_stderr() {
        let m = sample_metric();
        let summary = aggregate(vec![
            ResultRow {
                kind: ResultKind::Pass,
                reason: "accepted",
                metric: Some(m),
            },
            ResultRow {
                kind: ResultKind::Crash,
                reason: "child_failed_or_panicked",
                metric: None,
            },
            ResultRow {
                kind: ResultKind::Timeout,
                reason: "deadline_exceeded",
                metric: None,
            },
        ]);
        assert_eq!(summary.aggregate.runs, 3);
        assert_eq!(summary.aggregate.pass, 1);
        assert_eq!(summary.aggregate.crash, 1);
        assert_eq!(summary.aggregate.timeout, 1);
        assert!(!summary.build_accepted);
        assert_eq!(summary.rejected_reasons.len(), 2);
    }
}
