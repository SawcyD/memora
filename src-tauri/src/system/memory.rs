//! Physical + commit memory counters, read straight from the Win32 performance
//! APIs. Everything here is a real measurement; nothing is estimated.

use serde::Serialize;

/// A snapshot of system memory. All byte counts are absolute bytes.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshot {
    /// Total physical memory visible to the OS (excludes hardware-reserved).
    pub physical_total: u64,
    pub physical_available: u64,
    pub physical_in_use: u64,
    /// 0.0–100.0, physical in use as a share of physical total.
    pub percent_in_use: f64,

    pub commit_total: u64,
    pub commit_limit: u64,

    /// Working sets the cache manager currently holds. This is *not* memory
    /// that can be reported as freeable — see the Cleaner results rules.
    pub system_cache: u64,
    pub kernel_paged: u64,
    pub kernel_nonpaged: u64,

    pub page_size: u64,
    /// Milliseconds since the Unix epoch, for graph plotting on the frontend.
    pub timestamp_ms: u64,
}

/// The deeper breakdown shown on the Memory page.
///
/// Every field is optional because each comes from a source that can refuse:
/// the memory-list query is undocumented and can fail, and compressed memory is
/// derived from a process that may not exist. `None` means "not measured" and
/// the UI shows a dash — it never becomes a zero.
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDetail {
    /// Cached pages the OS can reclaim without writing anything to disk.
    pub standby: Option<u64>,
    /// Dirty pages that must be written to disk before reuse.
    pub modified: Option<u64>,
    /// Genuinely unused pages, including zeroed ones.
    pub free: Option<u64>,
    /// Physical memory held by the compression store.
    pub compressed: Option<u64>,
    /// Installed RAM the firmware withheld from Windows.
    pub hardware_reserved: Option<u64>,
    /// Installed RAM according to SMBIOS, which includes the reserved part.
    pub physical_installed: Option<u64>,
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(windows)]
pub fn snapshot() -> Result<MemorySnapshot, String> {
    use windows::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    let mut perf = PERFORMANCE_INFORMATION {
        cb: std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
        ..Default::default()
    };

    // SAFETY: both structs are zeroed, correctly sized, and their size field is
    // set as the API requires. Neither call retains the pointer.
    unsafe {
        GlobalMemoryStatusEx(&mut status).map_err(|e| format!("GlobalMemoryStatusEx failed: {e}"))?;
        GetPerformanceInfo(&mut perf, perf.cb)
            .map_err(|e| format!("GetPerformanceInfo failed: {e}"))?;
    }

    let page = perf.PageSize as u64;
    let pages = |n: usize| (n as u64).saturating_mul(page);

    let physical_total = status.ullTotalPhys;
    let physical_available = status.ullAvailPhys;

    Ok(MemorySnapshot {
        physical_total,
        physical_available,
        physical_in_use: physical_total.saturating_sub(physical_available),
        percent_in_use: if physical_total == 0 {
            0.0
        } else {
            (physical_total - physical_available) as f64 / physical_total as f64 * 100.0
        },
        commit_total: pages(perf.CommitTotal),
        commit_limit: pages(perf.CommitLimit),
        system_cache: pages(perf.SystemCache),
        kernel_paged: pages(perf.KernelPaged),
        kernel_nonpaged: pages(perf.KernelNonpaged),
        page_size: page,
        timestamp_ms: now_ms(),
    })
}

/// Non-Windows builds exist only so the crate still type-checks in CI/editors;
/// Memora is a Windows application and this path is never shipped.
#[cfg(not(windows))]
pub fn snapshot() -> Result<MemorySnapshot, String> {
    Err("Memory telemetry is only available on Windows".into())
}

#[cfg(windows)]
mod detail_imp {
    /// `SYSTEM_MEMORY_LIST_INFORMATION`, which is undocumented and therefore not
    /// bound by the `windows` crate. Field order matches ntexapi.h.
    #[repr(C)]
    #[derive(Default)]
    pub struct SystemMemoryListInformation {
        pub zero_page_count: usize,
        pub free_page_count: usize,
        pub modified_page_count: usize,
        pub modified_no_write_page_count: usize,
        pub bad_page_count: usize,
        /// Standby pages, split across the eight cache priorities.
        pub page_count_by_priority: [usize; 8],
        pub repurposed_pages_by_priority: [usize; 8],
        pub modified_page_count_page_file: usize,
    }

    #[link(name = "ntdll")]
    extern "system" {
        fn NtQuerySystemInformation(
            class: i32,
            info: *mut std::ffi::c_void,
            len: u32,
            return_len: *mut u32,
        ) -> i32;
    }

    const SYSTEM_MEMORY_LIST_INFORMATION_CLASS: i32 = 80;

    pub fn memory_list() -> Option<SystemMemoryListInformation> {
        let mut info = SystemMemoryListInformation::default();
        let mut returned = 0u32;

        // SAFETY: the struct is repr(C) and matches the layout the class writes;
        // its size is passed so the kernel cannot overrun it.
        let status = unsafe {
            NtQuerySystemInformation(
                SYSTEM_MEMORY_LIST_INFORMATION_CLASS,
                &mut info as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<SystemMemoryListInformation>() as u32,
                &mut returned,
            )
        };

        // Negative NTSTATUS is a failure; the caller reports "not measured".
        (status >= 0).then_some(info)
    }

    pub fn physically_installed() -> Option<u64> {
        use windows::Win32::System::SystemInformation::GetPhysicallyInstalledSystemMemory;

        let mut kb = 0u64;
        // SAFETY: kb is an owned local valid for the call.
        unsafe { GetPhysicallyInstalledSystemMemory(&mut kb) }
            .ok()
            .map(|_| kb * 1024)
    }

    /// Compressed memory is held by the Memory Compression process; its working
    /// set is how Task Manager derives the figure.
    pub fn compressed() -> Option<u64> {
        use super::super::process;

        process::enumerate()
            .ok()?
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case("MemCompression"))
            .and_then(|p| {
                process::open_for_query(p.pid).and_then(|h| process::working_set_of(h.0))
            })
    }
}

#[cfg(windows)]
pub fn detail() -> Result<MemoryDetail, String> {
    let snap = snapshot()?;
    let page = snap.page_size;
    let bytes = |pages: usize| (pages as u64).saturating_mul(page);

    let list = detail_imp::memory_list();
    let installed = detail_imp::physically_installed();

    Ok(MemoryDetail {
        standby: list
            .as_ref()
            .map(|l| bytes(l.page_count_by_priority.iter().sum::<usize>())),
        modified: list.as_ref().map(|l| bytes(l.modified_page_count)),
        // Free and zeroed pages are both unused; Task Manager reports them
        // together.
        free: list
            .as_ref()
            .map(|l| bytes(l.free_page_count + l.zero_page_count)),
        compressed: detail_imp::compressed(),
        // Only meaningful if SMBIOS reported more than Windows can address.
        hardware_reserved: installed.map(|i| i.saturating_sub(snap.physical_total)),
        physical_installed: installed,
    })
}

#[cfg(not(windows))]
pub fn detail() -> Result<MemoryDetail, String> {
    Err("Memory telemetry is only available on Windows".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_internally_consistent() {
        let s = snapshot().expect("snapshot");
        assert!(s.physical_total > 0, "total physical must be non-zero");
        assert!(s.physical_available <= s.physical_total);
        assert_eq!(s.physical_in_use, s.physical_total - s.physical_available);
        assert!((0.0..=100.0).contains(&s.percent_in_use));
        assert!(s.commit_limit >= s.commit_total);
        assert_eq!(s.page_size, 4096, "x64 Windows uses 4 KiB pages");
        println!(
            "total={} in_use={} ({:.1}%) commit={}/{} cache={}",
            s.physical_total,
            s.physical_in_use,
            s.percent_in_use,
            s.commit_total,
            s.commit_limit,
            s.system_cache
        );
    }

    /// The breakdown must add up: standby, modified and free are disjoint
    /// subsets of physical memory, so together they cannot exceed the total.
    #[test]
    fn detail_is_consistent_with_the_snapshot() {
        let snap = snapshot().expect("snapshot");
        let d = detail().expect("detail");

        let standby = d.standby.expect("standby should be readable");
        let modified = d.modified.expect("modified should be readable");
        let free = d.free.expect("free should be readable");

        assert!(
            standby + modified + free <= snap.physical_total,
            "standby {standby} + modified {modified} + free {free} exceeds total {}",
            snap.physical_total
        );

        // Available memory is essentially standby plus free; it should not be
        // wildly out of step with them.
        assert!(
            standby + free <= snap.physical_available + 512 * 1024 * 1024,
            "standby + free ({}) far exceeds available ({})",
            standby + free,
            snap.physical_available
        );

        if let Some(installed) = d.physical_installed {
            assert!(installed >= snap.physical_total, "installed must cover usable");
            let reserved = d.hardware_reserved.unwrap();
            assert_eq!(reserved, installed - snap.physical_total);
            assert!(reserved < 2 * 1024 * 1024 * 1024, "reserved looks implausible");
        }

        println!(
            "standby={} modified={} free={} compressed={:?} installed={:?} reserved={:?}",
            standby, modified, free, d.compressed, d.physical_installed, d.hardware_reserved
        );
    }
}
