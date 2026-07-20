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
}
