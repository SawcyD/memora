import { MemoryGraph } from "@/components/MemoryGraph";
import { Button, InfoRow, SectionHeader } from "@/components/primitives";
import { formatBytes, formatBytesPair, formatPercent } from "@/system/format";
import type { MemoryState } from "@/system/useMemory";

export function HomePage({
  memory,
  onOptimize,
}: {
  memory: MemoryState;
  onOptimize: () => void;
}) {
  const { current, history, error } = memory;

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
