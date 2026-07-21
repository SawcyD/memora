//! Durable record of optimization runs.
//!
//! Append-only JSON Lines. One record per line means a truncated or corrupt
//! line costs exactly one entry rather than the whole file — history is
//! evidence, and evidence should degrade gracefully.
//!
//! Automation depends on this: unattended action that leaves no trail cannot
//! be audited, and "why did it not run?" has to be answerable.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use super::clean::{CleanReport, Method};

/// What caused a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Source {
    /// The Cleaner page button.
    Manual,
    /// A tray click action.
    Tray,
    Automation {
        rule: String,
    },
    /// One selected application was trimmed after remaining minimized.
    Minimize {
        process: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RunOutcome {
    Completed,
    Cancelled,
    Failed {
        error: String,
    },
    /// A rule matched but a gate stopped it. Recorded so a user whose
    /// automation never fires can find out why.
    Blocked {
        gate: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Record {
    /// Milliseconds since the Unix epoch.
    pub at: u64,
    pub source: Source,
    pub outcome: RunOutcome,
    pub methods: Vec<Method>,
    pub available_before: u64,
    pub recovered_immediate: i64,
    /// The figure measured 30 seconds later, once decay has happened.
    ///
    /// `None` means not measured — Memora may exit before the delayed
    /// measurement lands. It is never coerced to zero, which would read as a
    /// real measurement showing no benefit.
    pub recovered_settled: Option<i64>,
    pub processes_trimmed: u32,
    pub processes_skipped: u32,
    pub errors: u32,
    pub duration_ms: u64,
    pub unavailable: Vec<String>,
    /// Per-process telemetry for minimize rules. `None` for full clean runs.
    pub target_pid: Option<u32>,
    pub working_set_before: Option<u64>,
    pub working_set_after: Option<u64>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            at: 0,
            source: Source::Manual,
            outcome: RunOutcome::Completed,
            methods: Vec::new(),
            available_before: 0,
            recovered_immediate: 0,
            recovered_settled: None,
            processes_trimmed: 0,
            processes_skipped: 0,
            errors: 0,
            duration_ms: 0,
            unavailable: Vec::new(),
            target_pid: None,
            working_set_before: None,
            working_set_after: None,
        }
    }
}

impl Record {
    pub fn from_report(source: Source, methods: &[Method], report: &CleanReport) -> Self {
        Self {
            at: now_ms(),
            source,
            outcome: if report.cancelled {
                RunOutcome::Cancelled
            } else {
                RunOutcome::Completed
            },
            methods: methods.to_vec(),
            available_before: report.available_before,
            recovered_immediate: report.recovered,
            recovered_settled: None,
            processes_trimmed: report.processes_trimmed,
            processes_skipped: report.processes_skipped,
            errors: report.errors,
            duration_ms: report.duration_ms,
            unavailable: report.unavailable.clone(),
            target_pid: None,
            working_set_before: None,
            working_set_after: None,
        }
    }

    pub fn from_minimize(
        process: String,
        pid: u32,
        working_set_before: u64,
        result: Result<u64, String>,
        duration_ms: u64,
    ) -> Self {
        let (outcome, working_set_after, processes_trimmed, errors) = match result {
            Ok(after) => (RunOutcome::Completed, Some(after), 1, 0),
            Err(error) => (RunOutcome::Failed { error }, None, 0, 1),
        };
        Self {
            at: now_ms(),
            source: Source::Minimize { process },
            outcome,
            duration_ms,
            target_pid: Some(pid),
            working_set_before: Some(working_set_before),
            working_set_after,
            processes_trimmed,
            errors,
            ..Default::default()
        }
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Retention. Old records are dropped on write rather than by a timer.
const MAX_RECORDS: usize = 500;
const MAX_AGE_MS: u64 = 90 * 24 * 60 * 60 * 1000;

pub struct Store {
    path: PathBuf,
    /// Serializes read-modify-write; the single-instance lock covers processes.
    lock: Mutex<()>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    /// Reads every record, newest first.
    ///
    /// Unparseable lines are skipped, not fatal: a partial write from a crash
    /// should cost one entry, never the whole history.
    pub fn list(&self) -> Vec<Record> {
        let _guard = self.lock.lock().unwrap();
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };

        let mut records: Vec<Record> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();

        records.sort_by(|a, b| b.at.cmp(&a.at));
        records
    }

    pub fn append(&self, record: &Record) -> Result<(), String> {
        let _guard = self.lock.lock().unwrap();

        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Could not create {dir:?}: {e}"))?;
        }

        let line = serde_json::to_string(record).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Could not open history: {e}"))?;
        writeln!(file, "{line}").map_err(|e| format!("Could not write history: {e}"))?;
        drop(file);

        self.prune_locked();
        Ok(())
    }

    /// Attaches the delayed measurement to the most recent matching record.
    ///
    /// Matched by timestamp because a run is identified by when it started;
    /// nothing else is unique and stable across the 30-second gap.
    pub fn set_settled(&self, at: u64, settled: i64) -> Result<(), String> {
        let _guard = self.lock.lock().unwrap();
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Ok(());
        };

        let mut found = false;
        let mut out = String::with_capacity(text.len() + 32);
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Record>(line) {
                Ok(mut r) if !found && r.at == at => {
                    r.recovered_settled = Some(settled);
                    found = true;
                    out.push_str(&serde_json::to_string(&r).map_err(|e| e.to_string())?);
                }
                // Unparseable lines are carried through untouched rather than
                // dropped: rewriting is not a licence to discard evidence.
                _ => out.push_str(line),
            }
            out.push('\n');
        }

        std::fs::write(&self.path, out).map_err(|e| format!("Could not update history: {e}"))
    }

    pub fn clear(&self) -> Result<(), String> {
        let _guard = self.lock.lock().unwrap();
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            // Already absent is the desired state.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Could not clear history: {e}")),
        }
    }

    /// Caller must hold `lock`.
    fn prune_locked(&self) {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return;
        };
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();

        let cutoff = now_ms().saturating_sub(MAX_AGE_MS);
        let keep: Vec<&str> = lines
            .iter()
            .filter(|l| {
                // A line that will not parse has no age; keep it rather than
                // using pruning as a excuse to silently delete it.
                serde_json::from_str::<Record>(l).is_ok_and(|r| r.at >= cutoff)
                    || serde_json::from_str::<Record>(l).is_err()
            })
            .copied()
            .collect();

        let start = keep.len().saturating_sub(MAX_RECORDS);
        let keep = &keep[start..];

        if keep.len() != lines.len() {
            let _ = std::fs::write(&self.path, keep.join("\n") + "\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!("memora-history-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("history.jsonl");
        (Store::new(path.clone()), path)
    }

    /// Recent, so retention does not legitimately drop it. Offsets are
    /// relative to now for the same reason.
    fn recent(offset_ms: u64) -> u64 {
        now_ms() - offset_ms
    }

    fn record(at: u64) -> Record {
        Record {
            at,
            processes_trimmed: 3,
            ..Default::default()
        }
    }

    #[test]
    fn appends_and_lists_newest_first() {
        let (store, _p) = temp_store("order");
        let (old, mid, new) = (recent(3000), recent(2000), recent(1000));
        store.append(&record(old)).unwrap();
        store.append(&record(new)).unwrap();
        store.append(&record(mid)).unwrap();

        let all = store.list();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].at, new, "newest first");
        assert_eq!(all[2].at, old);
    }

    #[test]
    fn missing_file_is_empty_not_an_error() {
        let (store, _p) = temp_store("missing");
        assert!(store.list().is_empty());
    }

    /// A crash mid-write leaves a partial line. That must cost one record.
    #[test]
    fn corrupt_lines_are_skipped_not_fatal() {
        let (store, path) = temp_store("corrupt");
        store.append(&record(recent(2000))).unwrap();
        store.append(&record(recent(1000))).unwrap();

        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("{\"at\": 3000, \"truncated\"\n");
        std::fs::write(&path, text).unwrap();

        let all = store.list();
        assert_eq!(all.len(), 2, "the two good records survive");
    }

    #[test]
    fn settled_attaches_to_the_right_record() {
        let (store, _p) = temp_store("settled");
        let (a, b) = (recent(2000), recent(1000));
        store.append(&record(a)).unwrap();
        store.append(&record(b)).unwrap();

        store.set_settled(a, 777).unwrap();

        let all = store.list();
        let target = all.iter().find(|r| r.at == a).unwrap();
        let other = all.iter().find(|r| r.at == b).unwrap();
        assert_eq!(target.recovered_settled, Some(777));
        assert_eq!(other.recovered_settled, None, "unknown stays unknown");
    }

    /// Rewriting the file to attach a measurement must not drop the bad lines
    /// it passes over.
    #[test]
    fn settled_rewrite_preserves_corrupt_lines() {
        let (store, path) = temp_store("preserve");
        store.append(&record(recent(1000))).unwrap();
        std::fs::write(
            &path,
            std::fs::read_to_string(&path).unwrap() + "not json at all\n",
        )
        .unwrap();

        store.set_settled(store.list()[0].at, 5).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("not json at all"),
            "evidence must survive a rewrite"
        );
    }

    #[test]
    fn retention_caps_the_record_count() {
        let (store, _p) = temp_store("cap");
        let base = now_ms() - 100_000;
        for i in 0..(MAX_RECORDS + 25) {
            store.append(&record(base + i as u64)).unwrap();
        }
        assert_eq!(store.list().len(), MAX_RECORDS);
    }

    #[test]
    fn records_older_than_the_window_are_dropped() {
        let (store, _p) = temp_store("age");
        store.append(&record(1)).unwrap(); // 1970
        store.append(&record(now_ms())).unwrap();
        let all = store.list();
        assert_eq!(all.len(), 1, "the 1970 record is far outside the window");
    }

    #[test]
    fn unknown_fields_from_a_newer_build_still_load() {
        let (store, path) = temp_store("forward");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "{\"at\":42,\"processesTrimmed\":2,\"somethingNew\":true}\n",
        )
        .unwrap();

        let all = store.list();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].processes_trimmed, 2);
    }

    #[test]
    fn clear_removes_everything() {
        let (store, _p) = temp_store("clear");
        store.append(&record(recent(1000))).unwrap();
        store.clear().unwrap();
        assert!(store.list().is_empty());
        // Clearing twice is not an error.
        store.clear().unwrap();
    }

    #[test]
    fn minimize_record_keeps_process_telemetry() {
        let r = Record::from_minimize("editor.exe".into(), 42, 500, Ok(125), 9);
        assert_eq!(
            r.source,
            Source::Minimize {
                process: "editor.exe".into()
            }
        );
        assert_eq!(r.target_pid, Some(42));
        assert_eq!(r.working_set_before, Some(500));
        assert_eq!(r.working_set_after, Some(125));
        assert_eq!(r.processes_trimmed, 1);
    }
}
