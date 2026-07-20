/**
 * Byte formatting that matches Task Manager's conventions: binary units shown
 * with decimal-style labels ("GB" meaning GiB), one decimal place for GB, none
 * for MB and below.
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";

  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;
  const TB = GB * 1024;

  if (bytes >= TB) return `${(bytes / TB).toFixed(1)} TB`;
  if (bytes >= GB) return `${(bytes / GB).toFixed(1)} GB`;
  if (bytes >= MB) return `${Math.round(bytes / MB)} MB`;
  if (bytes >= KB) return `${Math.round(bytes / KB)} KB`;
  return `${bytes} B`;
}

/** "18.4 / 31.8 GB" — used for committed memory, which Task Manager pairs. */
export function formatBytesPair(a: number, b: number): string {
  const GB = 1024 ** 3;
  return `${(a / GB).toFixed(1)} / ${(b / GB).toFixed(1)} GB`;
}

export function formatPercent(value: number): string {
  return `${Math.round(value)}%`;
}
