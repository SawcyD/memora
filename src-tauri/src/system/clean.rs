//! Memory optimization.
//!
//! Two very different things happen here and the UI must not conflate them:
//!
//! * **Working-set trimming** pushes a process's pages out to the standby list.
//!   Nothing is destroyed; the pages are still in RAM and the process will fault
//!   them back in on next use. Available memory rises, but the effect decays,
//!   which is why results are measured again after a delay.
//! * **Standby/modified list purging** genuinely returns pages to the free list,
//!   at the cost of discarding cache that Windows chose to keep. It needs
//!   elevation and is not something to run casually.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::memory;
use super::process::{self, ProcessInfo};

/// Cleaning methods, ordered from safest to most disruptive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    /// Empty the working set of every process Memora can open.
    TrimWorkingSets,
    /// Discard the standby (cached) page list. Requires elevation.
    PurgeStandbyList,
    /// Flush modified pages to disk so they can be reused. Requires elevation.
    FlushModifiedList,
}

impl Method {
    pub fn requires_elevation(self) -> bool {
        !matches!(self, Method::TrimWorkingSets)
    }
}

/// Whether Memora holds the privilege the memory-list commands need. The UI
/// uses this to disable those methods instead of letting them fail on click.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: token is closed on both paths; TOKEN_ELEVATION is the documented
    // output type for TokenElevation.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// What happened to one process during a trim.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessResult {
    pub pid: u32,
    pub name: String,
    pub working_set_before: u64,
    pub working_set_after: u64,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Trimmed,
    /// Protected, inaccessible, or explicitly excluded. Not a failure.
    Skipped,
    Failed,
}

/// Progress pushed to the UI while a run is in flight.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub current: String,
    pub completed: u32,
    pub total: u32,
    pub skipped: u32,
    /// Sum of observed working-set reduction so far. Explicitly *not* called
    /// "freed" — see the module note.
    pub working_set_reduced: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub available_before: u64,
    pub available_after: u64,
    /// available_after - available_before. May be negative if the system
    /// allocated during the run, so this is signed.
    pub recovered: i64,
    pub processes_trimmed: u32,
    pub processes_skipped: u32,
    pub errors: u32,
    pub duration_ms: u64,
    pub cancelled: bool,
    pub details: Vec<ProcessResult>,
    /// Methods that were requested but could not run, with the reason.
    pub unavailable: Vec<String>,
}

/// Cancellation flag shared with the command layer.
pub type Cancel = Arc<AtomicBool>;

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
    use windows::Win32::Security::{
        AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SET_QUOTA,
    };
    use windows::core::w;

    /// Enables a privilege on Memora's own token. Returns false when the
    /// process is not elevated, which is expected rather than exceptional.
    fn enable_privilege(name: windows::core::PCWSTR) -> bool {
        unsafe {
            let mut token = HANDLE::default();
            if OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )
            .is_err()
            {
                return false;
            }

            let mut luid = LUID::default();
            if LookupPrivilegeValueW(None, name, &mut luid).is_err() {
                let _ = CloseHandle(token);
                return false;
            }

            let tp = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };

            let ok = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None).is_ok()
                // AdjustTokenPrivileges reports success even when it did not
                // assign every privilege; GetLastError is the real answer.
                && windows::Win32::Foundation::GetLastError().is_ok();

            let _ = CloseHandle(token);
            ok
        }
    }

    /// Trims one process. `Skipped` means Memora could not obtain the rights,
    /// which is normal for system processes.
    fn trim_one(p: &ProcessInfo) -> (Outcome, u64) {
        if !p.accessible {
            return (Outcome::Skipped, p.working_set);
        }

        // SAFETY: pid is a value; the handle is closed on every path below.
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
                false,
                p.pid,
            )
        };
        let Ok(h) = handle else {
            return (Outcome::Skipped, p.working_set);
        };
        let h = process::Handle(h);

        // SAFETY: h is a live handle with PROCESS_SET_QUOTA.
        let trimmed = unsafe { EmptyWorkingSet(h.0) }.is_ok();
        if !trimmed {
            return (Outcome::Failed, p.working_set);
        }

        // Re-read rather than assuming the working set went to zero; the
        // process keeps whatever it touches during the call.
        let after = process::open_for_query(p.pid)
            .and_then(|q| process::working_set_of(q.0))
            .unwrap_or(0);

        (Outcome::Trimmed, after)
    }

    pub fn trim_working_sets(
        processes: &[ProcessInfo],
        excluded: &[u32],
        cancel: &Cancel,
        mut on_progress: impl FnMut(Progress),
    ) -> Vec<ProcessResult> {
        let total = processes.len() as u32;
        let mut results = Vec::with_capacity(processes.len());
        let (mut completed, mut skipped, mut reduced) = (0u32, 0u32, 0u64);

        for p in processes {
            if cancel.load(Ordering::Relaxed) {
                break;
            }

            let (outcome, after) = if excluded.contains(&p.pid) {
                (Outcome::Skipped, p.working_set)
            } else {
                trim_one(p)
            };

            match outcome {
                Outcome::Trimmed => {
                    completed += 1;
                    reduced += p.working_set.saturating_sub(after);
                }
                Outcome::Skipped => skipped += 1,
                Outcome::Failed => {}
            }

            results.push(ProcessResult {
                pid: p.pid,
                name: p.name.clone(),
                working_set_before: p.working_set,
                working_set_after: after,
                outcome,
            });

            on_progress(Progress {
                current: p.name.clone(),
                completed,
                total,
                skipped,
                working_set_reduced: reduced,
            });
        }

        results
    }

    /// `SYSTEM_MEMORY_LIST_COMMAND` values accepted by
    /// `SystemMemoryListInformation`.
    const MEMORY_FLUSH_MODIFIED_LIST: i32 = 3;
    const MEMORY_PURGE_STANDBY_LIST: i32 = 4;
    /// `SystemMemoryListInformation`
    const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;

    // NtSetSystemInformation is undocumented, so the `windows` crate does not
    // bind it (it binds only the Query counterpart). Declared here directly.
    #[link(name = "ntdll")]
    extern "system" {
        fn NtSetSystemInformation(
            system_information_class: i32,
            system_information: *mut std::ffi::c_void,
            system_information_length: u32,
        ) -> i32;
    }

    /// Issues a memory-list command. Requires SeProfileSingleProcessPrivilege,
    /// which is only obtainable when running elevated.
    fn memory_list_command(command: i32) -> Result<(), String> {
        if !enable_privilege(w!("SeProfileSingleProcessPrivilege")) {
            return Err("requires running Memora as administrator".into());
        }

        let mut cmd = command;
        // SAFETY: SystemMemoryListInformation takes a single command word; the
        // pointer and length describe exactly that i32.
        let status = unsafe {
            NtSetSystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut cmd as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            )
        };

        // NTSTATUS: negative values are errors.
        if status >= 0 {
            Ok(())
        } else {
            Err(format!("NtSetSystemInformation failed (NTSTATUS {status:#010x})"))
        }
    }

    pub fn purge_standby() -> Result<(), String> {
        memory_list_command(MEMORY_PURGE_STANDBY_LIST)
    }

    pub fn flush_modified() -> Result<(), String> {
        memory_list_command(MEMORY_FLUSH_MODIFIED_LIST)
    }
}

/// Runs an optimization pass.
///
/// `on_progress` is called from the calling thread; the command layer runs this
/// off the UI thread so the window stays responsive.
#[cfg(windows)]
pub fn run(
    methods: &[Method],
    excluded: &[u32],
    cancel: Cancel,
    on_progress: impl FnMut(Progress),
) -> Result<CleanReport, String> {
    let started = std::time::Instant::now();
    let before = memory::snapshot()?;

    let mut details = Vec::new();
    let mut unavailable = Vec::new();

    if methods.contains(&Method::TrimWorkingSets) {
        let processes = process::enumerate()?;
        details = imp::trim_working_sets(&processes, excluded, &cancel, on_progress);
    }

    // Privileged methods run after the trim so the standby list they purge
    // already contains everything the trim pushed out.
    for (method, label, action) in [
        (
            Method::PurgeStandbyList,
            "Clear standby memory",
            imp::purge_standby as fn() -> Result<(), String>,
        ),
        (
            Method::FlushModifiedList,
            "Clear modified page list",
            imp::flush_modified as fn() -> Result<(), String>,
        ),
    ] {
        if !methods.contains(&method) || cancel.load(Ordering::Relaxed) {
            continue;
        }

        // Checked here as well as in the UI: the command layer is reachable
        // regardless of what the UI chose to show.
        if method.requires_elevation() && !is_elevated() {
            unavailable.push(format!("{label}: requires running Memora as administrator"));
            continue;
        }

        if let Err(e) = action() {
            unavailable.push(format!("{label}: {e}"));
        }
    }

    let after = memory::snapshot()?;
    let trimmed = details.iter().filter(|d| d.outcome == Outcome::Trimmed).count() as u32;
    let skipped = details.iter().filter(|d| d.outcome == Outcome::Skipped).count() as u32;
    let errors = details.iter().filter(|d| d.outcome == Outcome::Failed).count() as u32;

    Ok(CleanReport {
        available_before: before.physical_available,
        available_after: after.physical_available,
        recovered: after.physical_available as i64 - before.physical_available as i64,
        processes_trimmed: trimmed,
        processes_skipped: skipped,
        errors,
        duration_ms: started.elapsed().as_millis() as u64,
        cancelled: cancel.load(Ordering::Relaxed),
        details,
        unavailable,
    })
}

#[cfg(not(windows))]
pub fn run(
    _methods: &[Method],
    _excluded: &[u32],
    _cancel: Cancel,
    _on_progress: impl FnMut(Progress),
) -> Result<CleanReport, String> {
    Err("Memory optimization is only available on Windows".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    /// Trims only this test process, so the suite never disturbs the machine it
    /// runs on. Proves the OpenProcess -> EmptyWorkingSet -> re-measure path.
    #[test]
    fn trims_its_own_working_set() {
        // Touch a few MB so there is a working set worth reclaiming.
        let ballast: Vec<u8> = (0..8 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        assert_eq!(ballast[4096], (4096 % 251) as u8);

        let pid = std::process::id();
        let me = process::enumerate()
            .expect("enumerate")
            .into_iter()
            .find(|p| p.pid == pid)
            .expect("test process must appear in its own enumeration");
        assert!(me.accessible, "a process can always open itself");
        assert!(me.working_set > 0);

        let cancel: Cancel = Arc::new(AtomicBool::new(false));
        let mut seen = Vec::new();
        let results = imp::trim_working_sets(&[me.clone()], &[], &cancel, |p| seen.push(p));

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, Outcome::Trimmed, "self-trim must succeed");
        assert!(
            results[0].working_set_after < results[0].working_set_before,
            "working set should shrink: {} -> {}",
            results[0].working_set_before,
            results[0].working_set_after
        );
        assert_eq!(seen.len(), 1, "one progress event per process");
        assert_eq!(seen[0].completed, 1);
        println!(
            "self trim: {} -> {} bytes",
            results[0].working_set_before, results[0].working_set_after
        );
    }

    #[test]
    fn excluded_processes_are_skipped_not_trimmed() {
        let pid = std::process::id();
        let me = process::enumerate()
            .expect("enumerate")
            .into_iter()
            .find(|p| p.pid == pid)
            .expect("self");

        let cancel: Cancel = Arc::new(AtomicBool::new(false));
        let results = imp::trim_working_sets(&[me], &[pid], &cancel, |_| {});
        assert_eq!(results[0].outcome, Outcome::Skipped);
    }

    #[test]
    fn cancellation_stops_before_any_work() {
        let cancel: Cancel = Arc::new(AtomicBool::new(true));
        let processes = process::enumerate().expect("enumerate");
        let results = imp::trim_working_sets(&processes, &[], &cancel, |_| {});
        assert!(results.is_empty(), "a pre-cancelled run must trim nothing");
    }

    /// The privileged path must report a clear reason rather than silently
    /// doing nothing when Memora is not elevated.
    #[test]
    fn standby_purge_reports_elevation_requirement() {
        match imp::purge_standby() {
            Ok(()) => assert!(is_elevated(), "purge only succeeds when elevated"),
            Err(e) => assert!(!e.is_empty(), "failure must carry a reason"),
        }
    }

    /// Exercises the measurement and reporting path without trimming anything
    /// on the host. A system-wide trim is a real (if reversible) side effect,
    /// so it is left to an explicit user action rather than the test suite.
    #[test]
    fn empty_run_still_reports_measurements() {
        let cancel: Cancel = Arc::new(AtomicBool::new(false));
        let report = run(&[], &[], cancel, |_| {}).expect("run");

        assert!(report.available_before > 0);
        assert!(report.available_after > 0);
        assert_eq!(report.processes_trimmed, 0);
        assert_eq!(report.errors, 0);
        assert!(report.details.is_empty());
        assert!(!report.cancelled);
        assert_eq!(
            report.recovered,
            report.available_after as i64 - report.available_before as i64
        );
    }

    /// Requesting a privileged method without elevation must be reported, not
    /// silently dropped.
    #[test]
    fn unelevated_privileged_method_is_reported_unavailable() {
        if is_elevated() {
            return; // Nothing to assert when the privilege is actually held.
        }
        let cancel: Cancel = Arc::new(AtomicBool::new(false));
        let report = run(&[Method::PurgeStandbyList], &[], cancel, |_| {}).expect("run");
        assert_eq!(report.unavailable.len(), 1);
        assert!(
            report.unavailable[0].contains("administrator"),
            "reason should name the requirement: {}",
            report.unavailable[0]
        );
    }
}
