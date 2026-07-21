import { MemoryGraph } from "@/components/MemoryGraph";
import { Button, InfoBar, InfoRow, SectionHeader } from "@/components/primitives";
import { formatBytes, formatBytesPair, formatPercent } from "@/system/format";
import { formatRate, pagingActivity } from "@/system/paging";
import type { MemoryState } from "@/system/useMemory";

export function HomePage({
  memory,
  onOptimize,
}: {
  memory: MemoryState;
  onOptimize: () => void;
}) {
  const { current, history, error } = memory;
  const paging = pagingActivity(history);

  if (error) {
    return (
      <p className="text-[13px] text-[var(--text-secondary)]">
        Memory counters are unavailable: {error}
      </p>
    );
  }

  return (
    <div className="max-w-[720px]">
      <SectionHeader>Memory</SectionHeader>

      <div className="mb-4 flex items-end justify-between gap-6">
        <div>
          <div className="tabular text-[28px] leading-[36px] text-[var(--text-primary)]">
            {current ? `${formatPercent(current.percentInUse)} in use` : "—"}
          </div>
          <div className="tabular mt-0.5 text-[13px] text-[var(--text-secondary)]">
            {current
              ? `${formatBytes(current.physicalInUse)} of ${formatBytes(current.physicalTotal)}`
              : "Reading memory counters…"}
          </div>
          <div className="tabular text-[13px] text-[var(--text-secondary)]">
            {current ? `${formatBytes(current.physicalAvailable)} available` : " "}
          </div>
        </div>

        <Button accent disabled={!current} onClick={onOptimize}>
          Optimize memory
        </Button>
      </div>

      {/* Commit charge nearing the limit causes allocation failures that
          surface as unexplained application crashes. The data is already
          sampled every second; saying so costs nothing. */}
      {current && current.commitLimit > 0 &&
        current.commitTotal / current.commitLimit >= 0.9 && (
          <div className="mb-4">
            <InfoBar
              title="Committed memory is close to the system limit"
              message={`${formatBytesPair(current.commitTotal, current.commitLimit)} committed. When this limit is reached, applications fail to allocate memory and may close unexpectedly. Increasing the paging file size, or closing some applications, avoids it. Trimming working sets does not help: it moves pages within RAM and does not reduce commit charge.`}
            />
          </div>
        )}

      {current && current.percentInUse >= 80 && paging.state === "sustained" && (
        <div className="mb-4">
          <InfoBar
            title="High memory use is causing disk-backed paging"
            message={`Windows has repeatedly read memory from storage during the recent sample (${formatRate(paging.readOperationsPerSecond)} page-read operations). Closing an unused memory-heavy application can help. Working-set trimming may make the paging worse.`}
          />
        </div>
      )}

      <MemoryGraph history={history} seconds={60} className="mb-5" />

      {/* Two columns of counters, Task Manager's density. */}
      <div className="grid grid-cols-1 gap-x-10 sm:grid-cols-2">
        <div>
          <InfoRow label="In use" value={current ? formatBytes(current.physicalInUse) : "—"} />
          <InfoRow
            label="Available"
            value={current ? formatBytes(current.physicalAvailable) : "—"}
          />
          <InfoRow
            label="Committed"
            value={current ? formatBytesPair(current.commitTotal, current.commitLimit) : "—"}
          />
          <InfoRow label="Cached" value={current ? formatBytes(current.systemCache) : "—"} />
        </div>
        <div>
          <InfoRow label="Paged pool" value={current ? formatBytes(current.kernelPaged) : "—"} />
          <InfoRow
            label="Non-paged pool"
            value={current ? formatBytes(current.kernelNonpaged) : "—"}
          />
          {/* Speed and slot count come from SMBIOS, which the shell does not
              read yet. Showing a placeholder beats inventing a number. */}
          <InfoRow label="Memory speed" value="—" />
          <InfoRow label="Slots used" value="—" />
        </div>
      </div>
    </div>
  );
}
