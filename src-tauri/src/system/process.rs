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
    /// Working set that cannot be shared with other processes. This is the
    /// number Task Manager's "Memory" column shows.
    pub private_working_set: u64,
    pub commit: u64,
    pub threads: u32,
    /// False when the process could not be opened for query. Its counters are
    /// then best-effort and trimming it will be skipped.
    pub accessible: bool,
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

    /// Returns (working set, private working set, commit).
    fn memory_counters(h: HANDLE) -> Option<(u64, u64, u64)> {
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

        Some((
            ex.WorkingSetSize as u64,
            // PrivateUsage is the closest counter to Task Manager's private
            // working set that is available without a perf-counter query.
            ex.PrivateUsage as u64,
            ex.PagefileUsage as u64,
        ))
    }

    /// Current working set of an already-open process, for measuring the effect
    /// of a trim without re-enumerating.
    pub fn working_set_of(h: HANDLE) -> Option<u64> {
        memory_counters(h).map(|(ws, _, _)| ws)
    }

    pub fn enumerate() -> Result<Vec<ProcessInfo>, String> {
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
            return Ok(out);
        }

        loop {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);

            let (ws, private, commit, accessible) = match open_for_query(entry.th32ProcessID) {
                Some(h) => match memory_counters(h.0) {
                    Some((ws, p, c)) => (ws, p, c, true),
                    // Opened but refused the query: still not actionable.
                    None => (0, 0, 0, false),
                },
                None => (0, 0, 0, false),
            };

            out.push(ProcessInfo {
                pid: entry.th32ProcessID,
                name,
                working_set: ws,
                private_working_set: private,
                commit,
                threads: entry.cntThreads,
                accessible,
            });

            // SAFETY: same invariants as Process32FirstW.
            if unsafe { Process32NextW(snapshot.0, &mut entry) }.is_err() {
                break;
            }
        }

        Ok(out)
    }
}

#[cfg(windows)]
pub use imp::{enumerate, open_for_query, working_set_of, Handle};

#[cfg(not(windows))]
pub fn enumerate() -> Result<Vec<ProcessInfo>, String> {
    Err("Process enumeration is only available on Windows".into())
}
