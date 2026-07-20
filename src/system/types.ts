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

/** Mirrors `MemoryDetail` in src-tauri/src/system/memory.rs.
 *  Null means the counter was not measured, never zero. */
export interface MemoryDetail {
  standby: number | null;
  modified: number | null;
  free: number | null;
  compressed: number | null;
  hardwareReserved: number | null;
  physicalInstalled: number | null;
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
  /** Private commit charge. Private working set is not exposed by the API
   *  Memora uses, so it is deliberately not shown — see process.rs. */
  commit: number;
  threads: number;
  handles: number;
  /** Null until a second sample exists to difference against — unknown, not 0. */
  cpuPercent: number | null;
  accessible: boolean;
}

/** Mirrors `Method` in src-tauri/src/system/clean.rs. */
export type CleanMethod =
  | "trimWorkingSets"
  | "purgeLowPriorityStandbyList"
  | "purgeStandbyList"
  | "flushModifiedList";

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

/** Mirrors `ClickAction` in src-tauri/src/system/settings.rs. */
export type ClickAction = "none" | "openMemora" | "openMemoryPage" | "optimize";

/** Mirrors `Settings` in src-tauri/src/system/settings.rs. */
export interface Settings {
  showTrayPercentage: boolean;
  trayIntervalSecs: number;
  warningThreshold: number;
  highThreshold: number;
  criticalThreshold: number;
  singleClick: ClickAction;
  doubleClick: ClickAction;
  middleClick: ClickAction;
  minimizeToTray: boolean;
  closeToTray: boolean;
  startWithWindows: boolean;
  showOptimizationNotifications: boolean;
  /** Process names, lowercased. Names not pids: pids change across reboots. */
  excludedProcesses: string[];
  automation: AutomationConfig;
}

/** Mirrors `Source` in src-tauri/src/system/history.rs. */
export type HistorySource =
  | { kind: "manual" }
  | { kind: "tray" }
  | { kind: "automation"; rule: string };

/** Mirrors `RunOutcome`. */
export type RunOutcome =
  | { kind: "completed" }
  | { kind: "cancelled" }
  | { kind: "failed"; error: string }
  | { kind: "blocked"; gate: string };

/** Mirrors `Record` in src-tauri/src/system/history.rs. */
export interface HistoryRecord {
  at: number;
  source: HistorySource;
  outcome: RunOutcome;
  methods: CleanMethod[];
  availableBefore: number;
  recoveredImmediate: number;
  /** Null means not measured — never coerce to zero. */
  recoveredSettled: number | null;
  processesTrimmed: number;
  processesSkipped: number;
  errors: number;
  durationMs: number;
  unavailable: string[];
}

/** Mirrors `Trigger` in src-tauri/src/system/automation.rs. */
export type Trigger =
  | { kind: "usageAbove"; percent: number; sustainedSecs: number }
  | { kind: "scheduled"; everyMins: number }
  | { kind: "systemIdle"; idleMins: number };

export interface AutomationRule {
  id: string;
  enabled: boolean;
  trigger: Trigger;
  ineffectiveLimit: number;
}

export interface AutomationProfile {
  name: string;
  methods: CleanMethod[];
  rules: AutomationRule[];
  minIntervalSecs: number;
}

export interface AutomationConfig {
  enabled: boolean;
  pausedUntil: number | null;
  activeProfile: string;
  profiles: AutomationProfile[];
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
