//! Process-local CPU and memory gauges. Unsupported OS => None; never invent telemetry.
//! Avoid third-party dependencies in the Rust-only production boundary.
#[cfg(target_os = "linux")]
pub fn resident_memory_mb() -> Option<f64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    let value = text.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
    })?;
    Some(value as f64 / 1024.0)
}

// /proc/self/schedstat accounts only for the calling thread. Linux
// /proc/self/stat fields 14 and 15 aggregate user + system CPU for the
// entire process, including Tokio worker threads.
#[cfg(target_os = "linux")]
fn linux_process_cpu_ticks(stat: &str) -> Option<u64> {
    // comm (field 2) may contain spaces or parentheses; split after its
    // final closing parenthesis, not at whitespace inside the process name.
    let (_, fields) = stat.rsplit_once(") ")?;
    let mut fields = fields.split_whitespace();
    // After comm, index 0 is state (field 3), so utime is index 11.
    let user = fields.nth(11)?.parse::<u64>().ok()?;
    let system = fields.next()?.parse::<u64>().ok()?;
    user.checked_add(system)
}

#[cfg(target_os = "linux")]
pub fn process_cpu_seconds() -> Option<f64> {
    use std::ffi::{c_int, c_long};
    unsafe extern "C" {
        fn sysconf(name: c_int) -> c_long;
    }
    // Linux _SC_CLK_TCK is 2 (glibc and musl). Query the real clock
    // tick rate instead of assuming 100 Hz on every Linux system.
    const SC_CLK_TCK: c_int = 2;
    // SAFETY: sysconf is called with a valid constant and no pointers.
    let ticks_per_second = unsafe { sysconf(SC_CLK_TCK) };
    if ticks_per_second <= 0 {
        return None;
    }
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let ticks = linux_process_cpu_ticks(&stat)?;
    Some(ticks as f64 / ticks_per_second as f64)
}

#[cfg(target_os = "windows")]
mod native {
    use std::ffi::c_void;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        cb: u32,
        faults: u32,
        peak_working_set: usize,
        working_set: usize,
        quota_peak_paged: usize,
        quota_paged: usize,
        quota_peak_nonpaged: usize,
        quota_nonpaged: usize,
        pagefile: usize,
        peak_pagefile: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn GetProcessTimes(
            process: *mut c_void,
            created: *mut FileTime,
            exited: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(process: *mut c_void, counters: *mut Counters, cb: u32) -> i32;
    }
    fn seconds(value: FileTime) -> f64 {
        (u64::from(value.high) << 32 | u64::from(value.low)) as f64 / 10_000_000.0
    }
    pub fn resident_memory_mb() -> Option<f64> {
        let mut counters = Counters {
            cb: std::mem::size_of::<Counters>() as u32,
            ..Counters::default()
        };
        // SAFETY: GetCurrentProcess returns a pseudo-handle valid in this process;
        // counters points to an initialized Windows PROCESS_MEMORY_COUNTERS buffer.
        let size = counters.cb;
        let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
        (ok != 0).then_some(counters.working_set as f64 / 1_048_576.0)
    }
    pub fn process_cpu_seconds() -> Option<f64> {
        let mut a = FileTime::default();
        let mut b = FileTime::default();
        let mut c = FileTime::default();
        let mut d = FileTime::default();
        // SAFETY: All four FILETIME pointers are valid writable stack allocations.
        let ok = unsafe { GetProcessTimes(GetCurrentProcess(), &mut a, &mut b, &mut c, &mut d) };
        (ok != 0).then_some(seconds(c) + seconds(d))
    }
}
#[cfg(target_os = "windows")]
pub use native::{process_cpu_seconds, resident_memory_mb};

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn resident_memory_mb() -> Option<f64> {
    None
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn process_cpu_seconds() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn parses_process_wide_ticks_with_spaced_comm() {
        let stat = format!("42 (my worker thread) S {}500 250 0 0", "0 ".repeat(10));
        assert_eq!(super::linux_process_cpu_ticks(&stat), Some(750));
        assert_eq!(super::linux_process_cpu_ticks("invalid stat"), None);
    }

    #[test]
    fn process_gauges_are_finite_or_unavailable() {
        if let Some(memory) = super::resident_memory_mb() {
            assert!(memory.is_finite() && memory >= 0.0);
        }
        if let Some(seconds) = super::process_cpu_seconds() {
            assert!(seconds.is_finite() && seconds >= 0.0);
        }
    }
}
