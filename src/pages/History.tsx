import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContentDialog } from "@/components/ContentDialog";
import { Button, InfoBar, SectionHeader } from "@/components/primitives";
import { formatBytes } from "@/system/format";
import type { HistoryRecord } from "@/system/types";

function when(ms: number): string {
  const d = new Date(ms);
  const today = new Date();
  const sameDay = d.toDateString() === today.toDateString();
  return sameDay
    ? d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleString(undefined, {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
}

function sourceLabel(s: HistoryRecord["source"]): string {
  switch (s.kind) {
    case "manual":
      return "Manual";
    case "tray":
      return "Tray";
    case "automation":
      return `Automation · ${s.rule}`;
  }
}

/** Signed, with the direction carried by a sign rather than colour alone. */
function signed(v: number): string {
  return v < 0 ? `−${formatBytes(-v)}` : `+${formatBytes(v)}`;
}

export function HistoryPage() {
  const [records, setRecords] = useState<HistoryRecord[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  const load = useCallback(() => {
    invoke<HistoryRecord[]>("list_history")
      .then((r) => {
        setRecords(r);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(load, [load]);

  if (error) {
    return (
      <div className="max-w-[820px]">
        <SectionHeader>History</SectionHeader>
        <InfoBar title="Could not read history" message={error} />
      </div>
    );
  }

  return (
    <div className="flex h-full max-w-[820px] flex-col">
      <SectionHeader>History</SectionHeader>

      <div className="mb-3 flex items-center gap-3">
        <span className="tabular text-[12px] text-[var(--text-secondary)]">
          {records ? `${records.length} recorded` : "Loading…"}
        </span>
        <div className="flex-1" />
        <Button onClick={load}>Refresh</Button>
        <Button disabled={!records?.length} onClick={() => setConfirmClear(true)}>
          Clear history
        </Button>
      </div>

      {records && records.length === 0 && (
        <InfoBar
          title="No optimizations recorded yet"
          message="Runs started from the Cleaner page, the tray, or automation are recorded here, including the memory that was still available 30 seconds afterwards."
        />
      )}

      {records && records.length > 0 && (
        <div className="min-h-0 flex-1 overflow-auto rounded-[var(--radius-md)] border border-[var(--stroke-control)]">
          <table className="w-full border-collapse text-[12px]">
            <thead className="sticky top-0 z-10 bg-[var(--acrylic-fill)] backdrop-blur">
              <tr className="text-[var(--text-secondary)]">
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-left font-normal">
                  When
                </th>
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-left font-normal">
                  Source
                </th>
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-left font-normal">
                  Result
                </th>
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-right font-normal">
                  Immediate
                </th>
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-right font-normal">
                  After 30s
                </th>
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-right font-normal">
                  Trimmed
                </th>
                <th scope="col" className="border-b border-[var(--stroke-divider)] px-2 py-1.5 text-right font-normal">
                  Duration
                </th>
              </tr>
            </thead>

            <tbody>
              {records.map((r) => (
                <tr key={`${r.at}-${r.processesTrimmed}`} className="border-b border-[var(--stroke-divider)] last:border-b-0">
                  <td className="tabular whitespace-nowrap px-2 py-1.5">{when(r.at)}</td>
                  <td className="px-2 py-1.5 text-[var(--text-secondary)]">{sourceLabel(r.source)}</td>
                  <td className="px-2 py-1.5">
                    {r.outcome.kind === "completed" && "Completed"}
                    {r.outcome.kind === "cancelled" && "Cancelled"}
                    {r.outcome.kind === "failed" && (
                      <span title={r.outcome.error}>Failed</span>
                    )}
                    {r.outcome.kind === "blocked" && (
                      <span title={r.outcome.gate}>Blocked</span>
                    )}
                  </td>
                  <td className="tabular px-2 py-1.5 text-right">
                    {r.outcome.kind === "completed" || r.outcome.kind === "cancelled"
                      ? signed(r.recoveredImmediate)
                      : "—"}
                  </td>
                  <td className="tabular px-2 py-1.5 text-right">
                    {/* Null is genuinely unknown: Memora may have exited before
                        the delayed measurement landed. */}
                    {r.recoveredSettled === null ? (
                      <span className="text-[var(--text-tertiary)]">Not measured</span>
                    ) : (
                      signed(r.recoveredSettled)
                    )}
                  </td>
                  <td className="tabular px-2 py-1.5 text-right">{r.processesTrimmed}</td>
                  <td className="tabular px-2 py-1.5 text-right">
                    {(r.durationMs / 1000).toFixed(1)} s
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <p className="mt-3 text-[12px] leading-4 text-[var(--text-secondary)]">
        The 30-second column is the figure that matters. Trimming moves pages to the standby list
        where they remain in RAM, so the immediate increase decays as processes resume.
      </p>

      {confirmClear && (
        <ContentDialog
          title="Clear optimization history?"
          primaryText="Clear history"
          destructive
          onCancel={() => setConfirmClear(false)}
          onPrimary={async () => {
            setConfirmClear(false);
            try {
              await invoke("clear_history");
              load();
            } catch (e) {
              setError(String(e));
            }
          }}
        >
          <p>
            All {records?.length ?? 0} records will be deleted. This is the only record of what
            Memora has done to your system, including anything automation ran unattended. This
            cannot be undone.
          </p>
        </ContentDialog>
      )}
    </div>
  );
}
