import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContentDialog } from "@/components/ContentDialog";
import { ContextMenu, type MenuAction } from "@/components/ContextMenu";
import { SearchBox } from "@/components/SearchBox";
import { InfoBar, SectionHeader } from "@/components/primitives";
import { formatBytes } from "@/system/format";
import type { ProcessInfo } from "@/system/types";

type ColumnId = keyof Pick<
  ProcessInfo,
  "name" | "pid" | "workingSet" | "commit" | "cpuPercent" | "threads" | "handles"
> | "status";

interface Column {
  id: ColumnId;
  label: string;
  /** Numeric columns are right-aligned and sort descending first. */
  numeric: boolean;
  width: number;
}

const COLUMNS: Column[] = [
  { id: "name", label: "Name", numeric: false, width: 220 },
  { id: "pid", label: "PID", numeric: true, width: 70 },
  { id: "status", label: "Status", numeric: false, width: 90 },
  { id: "workingSet", label: "Memory", numeric: true, width: 100 },
  { id: "commit", label: "Commit", numeric: true, width: 100 },
  { id: "cpuPercent", label: "CPU", numeric: true, width: 64 },
  { id: "threads", label: "Threads", numeric: true, width: 70 },
  { id: "handles", label: "Handles", numeric: true, width: 76 },
];

const REFRESH_MS = 2000;

export function ProcessesPage({
  excludedNames,
  onToggleExcluded,
}: {
  excludedNames: string[];
  onToggleExcluded: (name: string) => void;
}) {
  const [rows, setRows] = useState<ProcessInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<{ col: ColumnId; desc: boolean }>({
    col: "workingSet",
    desc: true,
  });
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [menu, setMenu] = useState<{ x: number; y: number; pid: number } | null>(null);
  const [confirmEnd, setConfirmEnd] = useState<ProcessInfo | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const lastClicked = useRef<number | null>(null);

  const refresh = useCallback(() => {
    invoke<ProcessInfo[]>("list_processes")
      .then((list) => {
        setRows(list);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, REFRESH_MS);
    return () => clearInterval(id);
  }, [refresh]);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    const filtered = q
      ? rows.filter((r) => r.name.toLowerCase().includes(q) || String(r.pid).includes(q))
      : rows;

    const dir = sort.desc ? -1 : 1;
    return [...filtered].sort((a, b) => {
      if (sort.col === "status") {
        return dir * (Number(a.accessible) - Number(b.accessible));
      }
      const av = a[sort.col as keyof ProcessInfo];
      const bv = b[sort.col as keyof ProcessInfo];

      if (typeof av === "string" && typeof bv === "string") {
        return dir * av.localeCompare(bv, undefined, { sensitivity: "base" });
      }
      // Unknown CPU sorts as lowest rather than being treated as zero.
      const an = typeof av === "number" ? av : -1;
      const bn = typeof bv === "number" ? bv : -1;
      return dir * (an - bn);
    });
  }, [rows, query, sort]);

  const toggleSort = (col: Column) =>
    setSort((s) =>
      s.col === col.id ? { col: col.id, desc: !s.desc } : { col: col.id, desc: col.numeric },
    );

  const onRowClick = (e: React.MouseEvent, pid: number, index: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (e.shiftKey && lastClicked.current !== null) {
        const from = visible.findIndex((r) => r.pid === lastClicked.current);
        if (from >= 0) {
          const [lo, hi] = from < index ? [from, index] : [index, from];
          for (let i = lo; i <= hi; i++) next.add(visible[i].pid);
          return next;
        }
      }
      if (e.ctrlKey) {
        next.has(pid) ? next.delete(pid) : next.add(pid);
        return next;
      }
      return new Set([pid]);
    });
    lastClicked.current = pid;
  };

  const target = menu ? rows.find((r) => r.pid === menu.pid) : null;

  const actions: MenuAction[] = target
    ? [
        { id: "trim", label: "Trim memory", disabled: !target.accessible },
        {
          id: "exclude",
          // Excluding by name covers every instance of the program, which is
          // what a user picking "this app" means.
          label: excludedNames.includes(target.name.toLowerCase())
            ? `Include ${target.name} in cleaning`
            : `Exclude ${target.name} from cleaning`,
        },
        { id: "copy", label: "Copy details" },
        { id: "end", label: "End task", danger: true, disabled: !target.accessible },
      ]
    : [];

  const onAction = async (id: string) => {
    if (!target) return;
    switch (id) {
      case "trim":
        try {
          const after = await invoke<number>("trim_process", { pid: target.pid });
          setNotice(
            `Trimmed ${target.name}: working set now ${formatBytes(after)}. Pages moved to the standby list and may be reloaded.`,
          );
          refresh();
        } catch (e) {
          setNotice(String(e));
        }
        break;
      case "exclude":
        onToggleExcluded(target.name);
        break;
      case "copy":
        await navigator.clipboard
          .writeText(
            [
              `Name: ${target.name}`,
              `PID: ${target.pid}`,
              `Memory: ${formatBytes(target.workingSet)}`,
              `Commit: ${formatBytes(target.commit)}`,
              `Threads: ${target.threads}`,
              `Handles: ${target.handles}`,
            ].join("\n"),
          )
          .catch(() => setNotice("Could not copy to the clipboard."));
        break;
      case "end":
        // Irreversible: always confirmed, never run straight from the menu.
        setConfirmEnd(target);
        break;
    }
  };

  return (
    <div className="flex h-full flex-col">
      <SectionHeader>Processes</SectionHeader>

      <div className="mb-3 flex items-center gap-3">
        <SearchBox label="Search processes" value={query} onChange={setQuery} />
        <span className="tabular text-[12px] text-[var(--text-secondary)]">
          {visible.length} of {rows.length}
        </span>
      </div>

      {error && (
        <div className="mb-3">
          <InfoBar title="Could not read the process list" message={error} />
        </div>
      )}
      {notice && (
        <div className="mb-3">
          <InfoBar title="Process" message={notice} />
        </div>
      )}

      <div className="min-h-0 flex-1 overflow-auto rounded-[var(--radius-md)] border border-[var(--stroke-control)]">
        <table className="w-full border-collapse text-[12px]">
          <thead className="sticky top-0 z-10 bg-[var(--acrylic-fill)] backdrop-blur">
            <tr>
              {COLUMNS.map((c) => (
                <th
                  key={c.id}
                  scope="col"
                  aria-sort={
                    sort.col === c.id ? (sort.desc ? "descending" : "ascending") : "none"
                  }
                  style={{ width: c.width }}
                  className="border-b border-[var(--stroke-divider)] p-0 font-normal"
                >
                  <button
                    type="button"
                    onClick={() => toggleSort(c)}
                    className={[
                      "flex h-7 w-full items-center gap-1 px-2",
                      "text-[var(--text-secondary)] hover:bg-[var(--subtle-hover)]",
                      c.numeric ? "justify-end" : "justify-start",
                    ].join(" ")}
                  >
                    {c.label}
                    {sort.col === c.id && (
                      <span aria-hidden="true" className="text-[9px]">
                        {sort.desc ? "▼" : "▲"}
                      </span>
                    )}
                  </button>
                </th>
              ))}
            </tr>
          </thead>

          <tbody>
            {visible.map((p, i) => {
              const isSelected = selected.has(p.pid);
              const isExcluded = excludedNames.includes(p.name.toLowerCase());
              return (
                <tr
                  key={p.pid}
                  aria-selected={isSelected}
                  tabIndex={0}
                  onClick={(e) => onRowClick(e, p.pid, i)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setSelected((s) => (s.has(p.pid) ? s : new Set([p.pid])));
                    setMenu({ x: e.clientX, y: e.clientY, pid: p.pid });
                  }}
                  className={[
                    "cursor-default border-b border-[var(--stroke-divider)]",
                    isSelected
                      ? "bg-[var(--accent-usable)]/18"
                      : "hover:bg-[var(--subtle-hover)]",
                  ].join(" ")}
                >
                  <td className="truncate px-2 py-1">
                    <span className={isExcluded ? "text-[var(--text-tertiary)]" : undefined}>
                      {p.name}
                    </span>
                    {isExcluded && (
                      <span className="ml-1.5 text-[10px] text-[var(--text-tertiary)]">
                        Excluded
                      </span>
                    )}
                  </td>
                  <td className="tabular px-2 py-1 text-right text-[var(--text-secondary)]">
                    {p.pid}
                  </td>
                  <td className="px-2 py-1 text-[var(--text-secondary)]">
                    {/* Marked, not hidden: protected processes are a normal
                        state, not an error. */}
                    {p.accessible ? "Running" : "Protected"}
                  </td>
                  <td className="tabular px-2 py-1 text-right">
                    {p.accessible ? formatBytes(p.workingSet) : "—"}
                  </td>
                  <td className="tabular px-2 py-1 text-right">
                    {p.accessible ? formatBytes(p.commit) : "—"}
                  </td>
                  <td className="tabular px-2 py-1 text-right">
                    {p.cpuPercent === null ? "—" : `${p.cpuPercent.toFixed(1)}%`}
                  </td>
                  <td className="tabular px-2 py-1 text-right">{p.threads}</td>
                  <td className="tabular px-2 py-1 text-right">
                    {p.handles > 0 ? p.handles : "—"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          actions={actions}
          onSelect={onAction}
          onDismiss={() => setMenu(null)}
        />
      )}

      {confirmEnd && (
        <ContentDialog
          title={`End ${confirmEnd.name}?`}
          primaryText="End task"
          destructive
          onCancel={() => setConfirmEnd(null)}
          onPrimary={async () => {
            const p = confirmEnd;
            setConfirmEnd(null);
            try {
              await invoke("end_process", { pid: p.pid });
              setNotice(`Ended ${p.name} (PID ${p.pid}).`);
              refresh();
            } catch (e) {
              setNotice(String(e));
            }
          }}
        >
          <p>
            Unsaved data in {confirmEnd.name} will be lost, and ending a system process can make
            Windows unstable. This cannot be undone.
          </p>
        </ContentDialog>
      )}
    </div>
  );
}
