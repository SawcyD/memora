/** Mirrors `MemorySnapshot` in src-tauri/src/system/memory.rs. */
export interface MemorySnapshot {
  physicalTotal: number;
  physicalAvailable: number;
  physicalInUse: number;
  percentInUse: number;
  commitTotal: number;
  commitLimit: number;
  systemCache: number;
  kernelPaged: number;
  kernelNonpaged: number;
  pageSize: number;
  timestampMs: number;
}

/** Mirrors `Accent` in src-tauri/src/system/accent.rs. */
export interface Accent {
  accent: string;
  accentLight1: string;
  accentLight2: string;
  accentDark1: string;
  highContrast: boolean;
}

/** Mirrors `ProcessInfo` in src-tauri/src/system/process.rs. */
export interface ProcessInfo {
  pid: number;
  name: string;
  workingSet: number;
  privateWorkingSet: number;
  commit: number;
  threads: number;
  accessible: boolean;
}

/** Mirrors `Method` in src-tauri/src/system/clean.rs. */
export type CleanMethod = "trimWorkingSets" | "purgeStandbyList" | "flushModifiedList";

export type Outcome = "trimmed" | "skipped" | "failed";

export interface ProcessResult {
  pid: number;
  name: string;
  workingSetBefore: number;
  workingSetAfter: number;
  outcome: Outcome;
}

export interface CleanProgress {
  current: string;
  completed: number;
  total: number;
  skipped: number;
  workingSetReduced: number;
}

export interface CleanReport {
  availableBefore: number;
  availableAfter: number;
  recovered: number;
  processesTrimmed: number;
  processesSkipped: number;
  errors: number;
  durationMs: number;
  cancelled: boolean;
  details: ProcessResult[];
  unavailable: string[];
}

export type PageId =
  | "home"
  | "memory"
  | "processes"
  | "cleaner"
  | "automation"
  | "history"
  | "settings"
  | "about";
