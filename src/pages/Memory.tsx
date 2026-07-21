import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MemoryGraph } from "@/components/MemoryGraph";
import { InfoBar, InfoRow, SectionHeader } from "@/components/primitives";
import { formatBytes, formatBytesPair, formatPercent } from "@/system/format";
import { formatRate, pagingActivity } from "@/system/paging";
import type { MemoryDetail } from "@/system/types";
import type { MemoryState } from "@/system/useMemory";

/** The spec's ranges. History holds one hour of 1 Hz samples. */
const RANGES = [
  { secs: 60, label: "60 seconds" },
  { secs: 300, label: "5 minutes" },
  { secs: 1800, label: "30 minutes" },
  { secs: 3600, label: "1 hour" },
];

const DETAIL_REFRESH_MS = 2000;

export function MemoryPage({ memory }: { memory: MemoryState }) {
  const { current, history, error } = memory;
  const [range, setRange] = useState(60);
  const [detail, setDetail] = useState<MemoryDetail | null>(null);
  const paging = useMemo(() => pagingActivity(history), [history]);

  const commitTrend = useMemo(() => {
    const last = history[history.length - 1];
    if (!last) return "Collecting commit trend data…";
    const first = history.find((sample) => sample.timestampMs >= last.timestampMs - 60_000);
    if (!first || last.timestampMs - first.timestampMs < 15_000) {
      return "Collecting commit trend data…";
    }
    const elapsedMinutes = (last.timestampMs - first.timestampMs) / 60_000;
    const perMinute = (last.commitTotal - first.commitTotal) / elapsedMinutes;
    const pressure = last.commitLimit > 0 ? (last.commitTotal / last.commitLimit) * 100 : 0;
    const noiseFloor = 32 * 1024 * 1024;
    let direction = "stable";
    if (perMinute > noiseFloor) direction = `rising ${formatBytes(perMinute)}/min`;
    if (perMinute < -noiseFloor) direction = `falling ${formatBytes(-perMinute)}/min`;

    let estimate = "";
    const target = last.commitLimit * 0.9;
    if (perMinute > noiseFloor && last.commitTotal < target) {
      const minutes = (target - last.commitTotal) / perMinute;
      if (minutes >= 1 && minutes <= 24 * 60) {
        estimate = ` · about ${Math.ceil(minutes)} min to 90% at this rate`;
      }
    }
    return `Commit ${direction} · ${pressure.toFixed(0)}% of limit${estimate}`;
  }, [history]);

  useEffect(() => {
    const load = () =>
      invoke<MemoryDetail>("memory_detail")
        .then(setDetail)
        .catch(() => setDetail(null));

    load();
    // Slower than the graph: this call enumerates processes.
    const id = setInterval(load, DETAIL_REFRESH_MS);
    return () => clearInterval(id);
  }, []);

  if (error) {
    return (
      <div className="max-w-[820px]">
        <SectionHeader>Memory</SectionHeader>
        <p className="text-[13px] text-[var(--text-secondary)]">
          Memory counters are unavailable: {error}
        </p>
      </div>
    );
  }

  /** Optional counters render as a dash when the source did not report them. */
  const opt = (v: number | null | undefined) =>
    v === null || v === undefined ? "—" : formatBytes(v);

  return (
    <div className="max-w-[820px]">
      <SectionHeader>Memory</SectionHeader>

      <div className="mb-1 flex items-baseline justify-between gap-6">
        <div className="tabular text-[20px] leading-7">
          {current
            ? `${formatPercent(current.percentInUse)} in use · ${formatBytes(
                current.physicalInUse,
              )} of ${formatBytes(current.physicalTotal)}`
            : "Reading memory counters…"}
        </div>

        {/* Range selector, styled as a Windows segmented control. */}
        <div
          role="radiogroup"
          aria-label="Graph range"
          className="flex shrink-0 overflow-hidden rounded-[var(--radius-md)] border border-[var(--stroke-control)]"
        >
          {RANGES.map((r) => (
            <button
              key={r.secs}
              type="button"
              role="radio"
              aria-checked={range === r.secs}
              onClick={() => setRange(r.secs)}
              className={[
                "h-7 px-2.5 text-[12px] transition-colors duration-75",
                "border-r border-[var(--stroke-divider)] last:border-r-0",
                range === r.secs
                  ? "bg-[var(--accent-usable)] text-[var(--text-on-accent)]"
                  : "bg-[var(--control-fill)] hover:bg-[var(--subtle-hover)]",
              ].join(" ")}
            >
              {r.label}
            </button>
          ))}
        </div>
      </div>

      <p className="mb-3 text-[12px] text-[var(--text-tertiary)]">
        {/* Say so rather than showing a misleadingly short line. */}
        {history.length < range
          ? `Collecting — ${history.length} of ${range} seconds recorded since Memora started.`
          : `Showing the last ${RANGES.find((r) => r.secs === range)?.label}.`}
      </p>

      <p
        className="mb-3 text-[12px] text-[var(--text-secondary)]"
        title="Commit includes memory promised to applications, backed by RAM or the page file. The estimate is a trend, not a prediction."
      >
        {commitTrend}
      </p>

      {current && current.percentInUse >= 80 && paging.state === "sustained" && (
        <div className="mb-4">
          <InfoBar
            title="Windows is repeatedly reading memory pages from disk"
            message={`${formatRate(paging.readOperationsPerSecond)} page-read operations and ${formatRate(paging.pagesReadPerSecond)} pages read over the recent sample. This can cause pauses. Trimming working sets may increase this activity because applications must reload pages.`}
          />
        </div>
      )}

      <MemoryGraph history={history} seconds={range} className="mb-5" />

      <div className="grid grid-cols-1 gap-x-10 sm:grid-cols-2">
        <div>
          <h3 className="mb-1 text-[13px] font-semibold">Physical memory</h3>
          <InfoRow label="Total" value={current ? formatBytes(current.physicalTotal) : "—"} />
          <InfoRow label="In use" value={current ? formatBytes(current.physicalInUse) : "—"} />
          <InfoRow
            label="Available"
            value={current ? formatBytes(current.physicalAvailable) : "—"}
          />
          <InfoRow label="Cached" value={current ? formatBytes(current.systemCache) : "—"} />
          <InfoRow label="Standby" value={opt(detail?.standby)} />
          <InfoRow label="Modified" value={opt(detail?.modified)} />
          <InfoRow label="Free" value={opt(detail?.free)} />
        </div>

        <div>
          <h3 className="mb-1 text-[13px] font-semibold">Commit and kernel</h3>
          <InfoRow
            label="Committed"
            value={current ? formatBytesPair(current.commitTotal, current.commitLimit) : "—"}
            help="Memory Windows has promised to processes compared with the RAM plus page-file limit."
          />
          <InfoRow
            label="Paged pool"
            value={current ? formatBytes(current.kernelPaged) : "—"}
            help="Kernel memory that Windows may move to disk."
          />
          <InfoRow
            label="Non-paged pool"
            value={current ? formatBytes(current.kernelNonpaged) : "—"}
            help="Kernel memory that must remain in physical RAM."
          />
          <InfoRow label="Compressed" value={opt(detail?.compressed)} />
          <InfoRow label="Installed" value={opt(detail?.physicalInstalled)} />
          <InfoRow label="Hardware reserved" value={opt(detail?.hardwareReserved)} />
        </div>
      </div>

      <div className="mt-5 max-w-[620px]">
        <h3 className="mb-1 text-[13px] font-semibold">Paging activity</h3>
        <div className="grid grid-cols-1 gap-x-10 sm:grid-cols-2">
          <div>
            <InfoRow
              label="Status"
              value={
                paging.state === "collecting"
                  ? "Collecting…"
                  : paging.state === "quiet"
                    ? "No disk paging detected"
                    : paging.state === "brief"
                      ? "Brief disk paging"
                      : "Sustained disk paging"
              }
              help="Sustained means page-read I/O occurred in most sampled seconds. It is not based on RAM usage alone."
            />
            <InfoRow
              label="Page faults"
              value={formatRate(paging.faultsPerSecond)}
              help="All faults, including ordinary soft faults resolved from RAM. A high value alone does not mean storage is slow."
            />
          </div>
          <div>
            <InfoRow
              label="Pages read from disk"
              value={formatRate(paging.pagesReadPerSecond)}
              help="Memory pages Windows read from storage to satisfy faults. This is the disk-backed signal."
            />
            <InfoRow
              label="Page-read operations"
              value={formatRate(paging.readOperationsPerSecond)}
              help="Physical I/O operations used for page reads; one operation may retrieve several pages."
            />
          </div>
        </div>
      </div>

      {detail && detail.compressed === null && (
        <p className="mt-4 text-[12px] leading-4 text-[var(--text-tertiary)]">
          Compressed memory is not shown: it is held by the Memory Compression process, which
          Windows protects from being opened for query.
        </p>
      )}
    </div>
  );
}
