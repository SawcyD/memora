//! Process enumeration.
//!
//! Access to other processes is a privilege question, not an error condition:
//! system and protected processes will refuse to open no matter how Memora
//! asks. Those are reported as `accessible: false` and carry whatever counters
//! could still be read, rather than being dropped or surfaced as failures.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    /// Physical memory currently backing the process.
    pub working_set: u64,
    /// Private commit charge — Task Manager's "Commit size".
    ///
    /// Task Manager's separate "private working set" column is deliberately
    /// absent: it comes from `SystemProcessInformation`'s
    /// `WorkingSetPrivateSize`, which the `windows` crate does not expose, and
    /// `GetProcessMemoryInfo` cannot distinguish it from commit. Showing commit
    /// twice under two headings would invent a distinction that is not measured.
    pub commit: u64,
    pub threads: u32,
    pub handles: u32,
    /// Share of one wall-clock second spent on CPU, across all cores, since the
    /// previous sample. Null on the first sample, when there is no baseline to
    /// difference against — an unknown value, not zero.
    pub cpu_percent: Option<f64>,
    /// All page faults per second (soft and hard). Disk-backed paging is
    /// reported separately at system level; this must not be labelled as hard
    /// faults.
    pub page_faults_per_sec: Option<f64>,
    /// False when the process could not be opened for query. Its counters are
    /// then best-effort and trimming it will be skipped.
    pub accessible: bool,
    /// True while Windows still reports the window minimized after an
    /// automatic minimize rule trimmed this process.
    pub minimized_trimmed: bool,
}

#[cfg(windows)]
mod imp {
    use super::ProcessInfo;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };

    /// RAII wrapper so every early return still closes the handle.
    pub struct Handle(pub HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: the handle came from OpenProcess/CreateToolhelp32Snapshot
                // and is closed exactly once, here.
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
    }

    /// Opens a process for reading counters. Falls back to the limited access
    /// right, which succeeds for more processes than the full one.
    pub fn open_for_query(pid: u32) -> Option<Handle> {
        // SAFETY: pid is a plain value; a failed open returns Err rather than
        // an invalid handle we might use.
        unsafe {
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .or_else(|_| OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid))
                .ok()
                .map(Handle)
        }
    }

    /// Process creation time and total CPU time, in 100ns ticks. Creation time
    /// prevents a quickly reused pid from inheriting another process's rate.
    pub fn process_times_of(h: HANDLE) -> Option<(u64, u64)> {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::GetProcessTimes;

        let (mut creation, mut exit, mut kernel, mut user) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );

        // SAFETY: all four out-params are owned locals valid for the call.
        let ok =
            unsafe { GetProcessTimes(h, &mut creation, &mut exit, &mut kernel, &mut user).is_ok() };
        if !ok {
            return None;
        }

        let ticks = |f: FILETIME| ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64;
        Some((ticks(creation), ticks(kernel) + ticks(user)))
    }

    pub fn handle_count_of(h: HANDLE) -> Option<u32> {
        use windows::Win32::System::Threading::GetProcessHandleCount;

        let mut count = 0u32;
        // SAFETY: count is an owned local valid for the call.
        if unsafe { GetProcessHandleCount(h, &mut count) }.is_ok() {
            Some(count)
        } else {
            None
        }
    }

    pub fn processor_count() -> u32 {
        use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};

        let mut info = SYSTEM_INFO::default();
        // SAFETY: info is an owned local; GetSystemInfo only writes to it.
        unsafe { GetSystemInfo(&mut info) };
        info.dwNumberOfProcessors.max(1)
    }

    /// Returns (working set, commit charge, cumulative page faults).
    fn memory_counters(h: HANDLE) -> Option<(u64, u64, u32)> {
        let mut ex = PROCESS_MEMORY_COUNTERS_EX::default();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

        // SAFETY: PROCESS_MEMORY_COUNTERS_EX is layout-compatible with the base
        // struct the API expects, and `size` tells it which one it received.
        let ok = unsafe {
            GetProcessMemoryInfo(h, &mut ex as *mut _ as *mut PROCESS_MEMORY_COUNTERS, size).is_ok()
        };
        if !ok {
            return None;
        }

        // PrivateUsage and PagefileUsage are the same underlying counter; only
        // one of them is reported, as commit.
        Some((
            ex.WorkingSetSize as u64,
            ex.PagefileUsage as u64,
            ex.PageFaultCount,
        ))
    }

    /// Current working set of an already-open process, for measuring the effect
    /// of a trim without re-enumerating.
    pub fn working_set_of(h: HANDLE) -> Option<u64> {
        memory_counters(h).map(|(ws, _, _)| ws)
    }

    #[derive(Clone, Copy)]
    pub struct RawCounters {
        pub pid: u32,
        pub created: u64,
        pub cpu_ticks: u64,
        pub page_faults: Option<u32>,
    }

    /// One pass over the process list plus cumulative counters that only
    /// become rates after a second sample — see `Sampler`.
    pub fn enumerate_with_cpu() -> Result<(Vec<ProcessInfo>, Vec<RawCounters>), String> {
        let mut counters: Vec<RawCounters> = Vec::with_capacity(256);
        // SAFETY: snapshot handle is checked and wrapped for close-on-drop.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|e| format!("CreateToolhelp32Snapshot failed: {e}"))?;
        let snapshot = Handle(snapshot);

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut out = Vec::with_capacity(256);
        // SAFETY: dwSize is set as the API requires; iteration stops on Err.
        if unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_err() {
            return Ok((out, counters));
        }

        loop {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);

            let mut info = ProcessInfo {
                pid: entry.th32ProcessID,
                name,
                working_set: 0,
                commit: 0,
                threads: entry.cntThreads,
                handles: 0,
                cpu_percent: None,
                page_faults_per_sec: None,
                accessible: false,
                minimized_trimmed: false,
            };

            if let Some(h) = open_for_query(entry.th32ProcessID) {
                let memory = memory_counters(h.0);
                if let Some((ws, commit, _)) = memory {
                    info.working_set = ws;
                    info.commit = commit;
                    info.accessible = true;
                }
                info.handles = handle_count_of(h.0).unwrap_or(0);
                if let Some((created, cpu_ticks)) = process_times_of(h.0) {
                    counters.push(RawCounters {
                        pid: info.pid,
                        created,
                        cpu_ticks,
                        page_faults: memory.map(|(_, _, faults)| faults),
                    });
                }
            }

            out.push(info);

            // SAFETY: same invariants as Process32FirstW.
            if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
                break;
            }
        }

        Ok((out, counters))
    }

    /// Plain enumeration, for callers that do not need CPU (the cleaner).
    pub fn enumerate() -> Result<Vec<ProcessInfo>, String> {
        enumerate_with_cpu().map(|(p, _)| p)
    }
}

#[cfg(windows)]
pub use imp::{enumerate, open_for_query, working_set_of, Handle};

/// Terminates a process.
///
/// Irreversible and capable of losing the user's unsaved work, so the UI is
/// required to confirm first; this function does not prompt.
#[cfg(windows)]
pub fn terminate(pid: u32) -> Result<(), String> {
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    // SAFETY: handle is wrapped for close-on-drop; exit code 1 marks an
    // externally terminated process, as Task Manager does.
    unsafe {
        let h = OpenProcess(PROCESS_TERMINATE, false, pid)
            .map_err(|e| format!("Cannot end this process: {e}"))?;
        let h = Handle(h);
        TerminateProcess(h.0, 1).map_err(|e| format!("Failed to end process: {e}"))
    }
}

#[cfg(not(windows))]
pub fn terminate(_pid: u32) -> Result<(), String> {
    Err("Only available on Windows".into())
}

/// Turns successive enumerations into CPU and page-fault rates.
///
/// CPU usage is a rate, so it does not exist until there are two readings to
/// difference. The first sample therefore reports `None` rather than 0, which
/// would be a claim the data does not support.
#[cfg(windows)]
pub struct Sampler {
    previous: std::collections::HashMap<u32, imp::RawCounters>,
    last_at: Option<std::time::Instant>,
    processors: u32,
}

#[cfg(windows)]
impl Sampler {
    pub fn new() -> Self {
        Self {
            previous: std::collections::HashMap::new(),
            last_at: None,
            processors: imp::processor_count(),
        }
    }

    pub fn sample(&mut self) -> Result<Vec<ProcessInfo>, String> {
        let (mut processes, counters) = imp::enumerate_with_cpu()?;
        let now = std::time::Instant::now();

        if let Some(previous_at) = self.last_at {
            // 100ns ticks per wall-clock nanosecond, times the number of cores,
            // is the total CPU time the machine could have spent in the gap.
            let elapsed_ns = now.duration_since(previous_at).as_nanos() as f64;
            let elapsed_secs = now.duration_since(previous_at).as_secs_f64();
            let capacity_ticks = (elapsed_ns / 100.0) * self.processors as f64;

            if capacity_ticks > 0.0 && elapsed_secs > 0.0 {
                let current: std::collections::HashMap<u32, imp::RawCounters> = counters
                    .iter()
                    .copied()
                    .map(|sample| (sample.pid, sample))
                    .collect();
                for p in &mut processes {
                    let Some((now_c, then_c)) = current
                        .get(&p.pid)
                        .zip(self.previous.get(&p.pid))
                        .filter(|(now_c, then_c)| now_c.created == then_c.created)
                    else {
                        continue;
                    };

                    let used = now_c.cpu_ticks.saturating_sub(then_c.cpu_ticks) as f64;
                    p.cpu_percent = Some((used / capacity_ticks * 100.0).clamp(0.0, 100.0));

                    if let (Some(now_faults), Some(then_faults)) =
                        (now_c.page_faults, then_c.page_faults)
                    {
                        // This 32-bit Windows counter wraps during long
                        // uptimes; wrapping subtraction preserves the rate.
                        p.page_faults_per_sec =
                            Some(now_faults.wrapping_sub(then_faults) as f64 / elapsed_secs);
                    }
                }
            }
        }

        self.previous = counters
            .into_iter()
            .map(|sample| (sample.pid, sample))
            .collect();
        self.last_at = Some(now);
        Ok(processes)
    }
}

#[cfg(windows)]
impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(windows))]
pub fn enumerate() -> Result<Vec<ProcessInfo>, String> {
    Err("Process enumeration is only available on Windows".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn enumeration_finds_this_process() {
        let list = enumerate().expect("enumerate");
        assert!(list.len() > 10, "a live system has many processes");

        let me = list
            .iter()
            .find(|p| p.pid == std::process::id())
            .expect("self must appear");
        assert!(me.accessible);
        assert!(me.working_set > 0);
        assert!(me.threads > 0);
        assert!(me.handles > 0, "a running process holds handles");
    }

    /// Inaccessible processes must be marked, not dropped or errored.
    #[test]
    fn protected_processes_are_marked_not_omitted() {
        let list = enumerate().expect("enumerate");
        // pid 4 is the System process, which never opens for query.
        let system = list.iter().find(|p| p.pid == 4);
        if let Some(p) = system {
            assert!(!p.accessible, "System should report as inaccessible");
        }
        assert!(
            list.iter().any(|p| !p.accessible),
            "some processes are always protected"
        );
    }

    /// The first sample has no baseline, so CPU must be unknown rather than 0.
    #[test]
    fn first_sample_reports_unknown_cpu() {
        let mut s = Sampler::new();
        let first = s.sample().expect("first sample");
        assert!(
            first.iter().all(|p| p.cpu_percent.is_none()),
            "no process can have a rate before a second reading"
        );
        assert!(first.iter().all(|p| p.page_faults_per_sec.is_none()));
    }

    #[test]
    fn second_sample_reports_bounded_cpu() {
        let mut s = Sampler::new();
        s.sample().expect("first");
        // Burn a little CPU so at least one process has measurable usage.
        let mut acc = 0u64;
        let spin = std::time::Instant::now();
        while spin.elapsed() < std::time::Duration::from_millis(120) {
            acc = acc.wrapping_add(spin.elapsed().as_nanos() as u64);
        }
        // Commit and touch fresh pages so this process has a measurable page
        // fault delta as well as CPU activity.
        let mut allocation = vec![0u8; 16 * 1024 * 1024];
        for byte in allocation.iter_mut().step_by(4096) {
            *byte = 1;
        }
        assert!(acc > 0);

        let second = s.sample().expect("second");
        let measured: Vec<_> = second.iter().filter(|p| p.cpu_percent.is_some()).collect();
        assert!(!measured.is_empty(), "second sample must produce rates");
        for p in &measured {
            let c = p.cpu_percent.unwrap();
            assert!((0.0..=100.0).contains(&c), "{} reported {c}%", p.name);
        }

        let me = second
            .iter()
            .find(|p| p.pid == std::process::id())
            .expect("self");
        assert!(
            me.cpu_percent.unwrap_or(0.0) > 0.0,
            "the spinning test process should show CPU usage"
        );
        assert!(
            me.page_faults_per_sec.unwrap_or(0.0) > 0.0,
            "touching fresh pages should produce page faults"
        );
        assert_eq!(allocation[0], 1);
    }
}
