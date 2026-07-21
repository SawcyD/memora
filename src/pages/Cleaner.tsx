import { useState } from "react";
import {
  Button,
  InfoBar,
  InfoRow,
  ProgressBar,
  SectionHeader,
  SettingsRow,
  SettingsSection,
  ToggleSwitch,
} from "@/components/primitives";
import { formatBytes } from "@/system/format";
import type { CleanMethod } from "@/system/types";
import type { CleanState } from "@/system/useClean";

interface MethodSpec {
  id: CleanMethod;
  name: string;
  description: string;
  risk: "Low" | "Moderate" | "High";
  /** Experimental or disruptive methods start off and say why. */
  defaultOn: boolean;
  needsElevation: boolean;
}

const METHODS: MethodSpec[] = [
  {
    id: "trimWorkingSets",
    name: "Trim inactive process working sets",
    description:
      "Asks Windows to page out memory each process is not actively using. Pages stay in RAM on the standby list, so processes reload them quickly if needed.",
    risk: "Low",
    defaultOn: true,
    needsElevation: false,
  },
  {
    id: "purgeLowPriorityStandbyList",
    name: "Clear low-priority cached memory",
    description:
      "Discards only the cached data Windows itself ranked as least worth keeping. Reclaims memory without throwing away the cache that is making your PC fast.",
    risk: "Low",
    defaultOn: false,
    needsElevation: true,
  },
  {
    id: "purgeStandbyList",
    name: "Clear standby memory",
    description:
      "Discards cached file and application data Windows is keeping for fast reuse. Genuinely increases free memory, but the next read of that data goes to disk.",
    risk: "Moderate",
    defaultOn: false,
    needsElevation: true,
  },
  {
    id: "flushModifiedList",
    name: "Clear modified page list",
    description:
      "Writes modified pages to disk so their memory can be reused. Causes a burst of disk activity.",
    risk: "Moderate",
    defaultOn: false,
    needsElevation: true,
  },
];

export function CleanerPage({
  clean,
  excludedNames,
}: {
  clean: CleanState;
  /** Process names excluded from the Processes page; skipped by every method. */
  excludedNames: string[];
}) {
  const [enabled, setEnabled] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(METHODS.map((m) => [m.id, m.defaultOn])),
  );

  if (clean.phase === "done" && clean.report) {
    return <Results clean={clean} />;
  }

  const selected = METHODS.filter((m) => enabled[m.id]).map((m) => m.id);
  const running = clean.phase === "running";

  return (
    <div className="max-w-[720px]">
      <SectionHeader>Cleaner</SectionHeader>

      {!clean.elevated && (
        <div className="mb-3">
          <InfoBar
            title="Some methods need administrator rights"
            message="Memora is running without elevation, so clearing standby and modified memory is unavailable. Restart Memora as administrator to enable them."
          />
        </div>
      )}

      {clean.error && (
        <div className="mb-3">
          <InfoBar title="Optimization failed" message={clean.error} />
        </div>
      )}

      <SettingsSection>
        {METHODS.map((m) => {
          const blocked = m.needsElevation && !clean.elevated;
          return (
            <SettingsRow
              key={m.id}
              title={m.name}
              description={m.description}
              note={
                <span>
                  Risk: {m.risk}
                  {blocked && " · Requires administrator"}
                  {!m.defaultOn && !blocked && " · Off by default"}
                </span>
              }
              control={
                <ToggleSwitch
                  label={m.name}
                  checked={!blocked && !!enabled[m.id]}
                  disabled={blocked || running}
                  onChange={(v) => setEnabled((s) => ({ ...s, [m.id]: v }))}
                />
              }
            />
          );
        })}
      </SettingsSection>

      {running ? (
        <Running clean={clean} />
      ) : (
        <div className="mt-4">
          <Button
            accent
            disabled={selected.length === 0}
            onClick={() => clean.start(selected)}
          >
            Optimize now
          </Button>
          {excludedNames.length > 0 && (
            <p className="mt-2 text-[12px] text-[var(--text-secondary)]">
              {excludedNames.length} process name{excludedNames.length === 1 ? "" : "s"} excluded
              from cleaning.
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function Running({ clean }: { clean: CleanState }) {
  const p = clean.progress;
  return (
    <div className="mt-4">
      <div className="mb-2 flex items-baseline justify-between">
        <span className="truncate text-[13px] text-[var(--text-primary)]">
          {p ? p.current : "Starting…"}
        </span>
        <span className="tabular text-[12px] text-[var(--text-secondary)]">
          {p ? `${p.completed} of ${p.total}` : ""}
        </span>
      </div>

      <ProgressBar value={p?.completed ?? 0} max={p?.total ?? 1} />

      <div className="mt-2 flex gap-6 text-[12px] text-[var(--text-secondary)]">
        <span className="tabular">Skipped: {p?.skipped ?? 0}</span>
        <span className="tabular">
          Working set reduced: {formatBytes(p?.workingSetReduced ?? 0)}
        </span>
      </div>

      <div className="mt-4">
        <Button onClick={clean.cancel}>Cancel</Button>
      </div>
    </div>
  );
}

function Results({ clean }: { clean: CleanState }) {
  const [expanded, setExpanded] = useState(false);
  const r = clean.report!;
  const affected = r.details.filter((d) => d.outcome !== "skipped");

  return (
    <div className="max-w-[720px]">
      <SectionHeader>
        {r.cancelled ? "Memory optimization cancelled" : "Memory optimization completed"}
      </SectionHeader>

      <div className="mb-4 grid grid-cols-1 gap-x-10 sm:grid-cols-2">
        <div>
          <InfoRow label="Available before" value={formatBytes(r.availableBefore)} />
          <InfoRow label="Available after" value={formatBytes(r.availableAfter)} />
          <InfoRow
            label="Immediately recovered"
            value={signedBytes(r.recovered)}
          />
          <InfoRow
            label="Still available after 30 seconds"
            value={clean.settled === null ? "Measuring…" : signedBytes(clean.settled)}
          />
        </div>
        <div>
          <InfoRow label="Processes trimmed" value={r.processesTrimmed} />
          <InfoRow label="Processes skipped" value={r.processesSkipped} />
          <InfoRow label="Errors" value={r.errors} />
          <InfoRow label="Duration" value={`${(r.durationMs / 1000).toFixed(1)} s`} />
        </div>
      </div>

      {/* The spec's central honesty requirement: trimming relocates pages, it
          does not free them. The settled figure above is the real one. */}
      <p className="mb-4 text-[12px] leading-4 text-[var(--text-secondary)]">
        Trimming working sets moves pages to the standby list, where they remain in RAM and are
        reloaded as processes resume. The figure measured after 30 seconds reflects how much of the
        increase actually persisted.
      </p>

      {r.unavailable.length > 0 && (
        <div className="mb-4">
          <InfoBar
            title="Some methods did not run"
            message={r.unavailable.join("; ")}
          />
        </div>
      )}

      <button
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((v) => !v)}
        className="mb-2 text-[13px] text-[var(--accent-usable)] hover:underline"
      >
        {expanded ? "Hide details" : `Show details (${affected.length} processes)`}
      </button>

      {expanded && (
        <SettingsSection>
          <div className="max-h-[280px] overflow-y-auto">
            {affected.map((d) => (
              <div
                key={d.pid}
                className="flex items-center gap-4 border-b border-[var(--stroke-divider)] px-4 py-1.5 text-[12px] last:border-b-0"
              >
                <span className="min-w-0 flex-1 truncate">{d.name}</span>
                <span className="tabular w-14 text-right text-[var(--text-tertiary)]">{d.pid}</span>
                <span className="tabular w-40 text-right text-[var(--text-secondary)]">
                  {formatBytes(d.workingSetBefore)} → {formatBytes(d.workingSetAfter)}
                </span>
                <span className="w-14 text-right text-[var(--text-tertiary)]">
                  {d.outcome === "failed" ? "Failed" : "Trimmed"}
                </span>
              </div>
            ))}
          </div>
        </SettingsSection>
      )}

      <div className="mt-4">
        <Button accent onClick={clean.dismiss}>
          Done
        </Button>
      </div>
    </div>
  );
}

/** Recovery can be negative when the system allocates during the run. */
function signedBytes(v: number): string {
  if (v < 0) return `−${formatBytes(-v)}`;
  return formatBytes(v);
}
