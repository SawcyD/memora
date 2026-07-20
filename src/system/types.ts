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

export type PageId =
  | "home"
  | "memory"
  | "processes"
  | "cleaner"
  | "automation"
  | "history"
  | "settings"
  | "about";
