import type { MemorySnapshot } from "./types";

export type PagingState = "collecting" | "quiet" | "brief" | "sustained";

export interface PagingActivity {
  state: PagingState;
  seconds: number;
  faultsPerSecond: number | null;
  pagesReadPerSecond: number | null;
  readOperationsPerSecond: number | null;
}

/** Windows exposes these as ULONG counters, so long uptimes can wrap them. */
export function counterDelta(current: number, previous: number): number {
  return current >= previous ? current - previous : 0x1_0000_0000 - previous + current;
}

/**
 * Summarizes the last 15 seconds without treating ordinary soft faults as disk
 * pressure. "Sustained" means page-read I/O occurred in most sampled seconds;
 * it is an observation, not a health score or an invented severity threshold.
 */
export function pagingActivity(history: MemorySnapshot[], windowSeconds = 15): PagingActivity {
  const recent = history.slice(-(windowSeconds + 1));
  const first = recent[0];
  const last = recent[recent.length - 1];
  if (
    recent.length < 3 ||
    !first ||
    !last ||
    first.pageFaultCount === null ||
    first.pageReadCount === null ||
    first.pageReadIoCount === null ||
    last.pageFaultCount === null ||
    last.pageReadCount === null ||
    last.pageReadIoCount === null
  ) {
    return {
      state: "collecting",
      seconds: 0,
      faultsPerSecond: null,
      pagesReadPerSecond: null,
      readOperationsPerSecond: null,
    };
  }

  const seconds = Math.max(0, (last.timestampMs - first.timestampMs) / 1000);
  if (seconds < 1) {
    return {
      state: "collecting",
      seconds,
      faultsPerSecond: null,
      pagesReadPerSecond: null,
      readOperationsPerSecond: null,
    };
  }

  const faults = counterDelta(last.pageFaultCount, first.pageFaultCount);
  const pagesRead = counterDelta(last.pageReadCount, first.pageReadCount);
  const readOperations = counterDelta(last.pageReadIoCount, first.pageReadIoCount);

  let validIntervals = 0;
  let intervalsWithDiskReads = 0;
  for (let i = 1; i < recent.length; i++) {
    const previous = recent[i - 1].pageReadIoCount;
    const current = recent[i].pageReadIoCount;
    if (previous === null || current === null) continue;
    validIntervals++;
    if (counterDelta(current, previous) > 0) intervalsWithDiskReads++;
  }

  const sustained = validIntervals >= 5 && intervalsWithDiskReads / validIntervals >= 0.6;
  return {
    state: sustained ? "sustained" : readOperations > 0 ? "brief" : "quiet",
    seconds,
    faultsPerSecond: faults / seconds,
    pagesReadPerSecond: pagesRead / seconds,
    readOperationsPerSecond: readOperations / seconds,
  };
}

export function formatRate(value: number | null): string {
  if (value === null) return "—";
  if (value < 10) return `${value.toFixed(1)}/s`;
  return `${Math.round(value).toLocaleString()}/s`;
}
