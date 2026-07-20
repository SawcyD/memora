import { useRef } from "react";
import type { PageId } from "@/system/types";
import {
  AboutIcon,
  AutomationIcon,
  CleanerIcon,
  HistoryIcon,
  HomeIcon,
  MemoryIcon,
  MenuIcon,
  ProcessesIcon,
  SettingsIcon,
} from "./Icons";

export interface NavItem {
  id: PageId;
  label: string;
  Icon: (p: { className?: string }) => React.ReactElement;
}

export const NAV_ITEMS: NavItem[] = [
  { id: "home", label: "Home", Icon: HomeIcon },
  { id: "memory", label: "Memory", Icon: MemoryIcon },
  { id: "processes", label: "Processes", Icon: ProcessesIcon },
  { id: "cleaner", label: "Cleaner", Icon: CleanerIcon },
  { id: "automation", label: "Automation", Icon: AutomationIcon },
  { id: "history", label: "History", Icon: HistoryIcon },
  { id: "settings", label: "Settings", Icon: SettingsIcon },
];

export const FOOTER_ITEMS: NavItem[] = [{ id: "about", label: "About Memora", Icon: AboutIcon }];

function NavButton({
  item,
  selected,
  collapsed,
  onSelect,
}: {
  item: NavItem;
  selected: boolean;
  collapsed: boolean;
  onSelect: (id: PageId) => void;
}) {
  const { Icon, label, id } = item;
  return (
    <button
      type="button"
      role="tab"
      aria-selected={selected}
      aria-controls="memora-content"
      // Collapsed rail is icon-only, so the name has to come from the label.
      aria-label={collapsed ? label : undefined}
      title={collapsed ? label : undefined}
      tabIndex={selected ? 0 : -1}
      data-nav-item
      onClick={() => onSelect(id)}
      className={[
        "relative flex h-9 w-full items-center rounded-[var(--radius-md)]",
        "text-left text-[var(--text-primary)] transition-colors duration-75",
        "hover:bg-[var(--subtle-hover)] active:bg-[var(--subtle-pressed)]",
        selected && "bg-[var(--control-fill)]",
        collapsed ? "justify-center px-0" : "gap-3 pl-3 pr-2",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {/* Windows 11 selection indicator: a 3px accent pill on the leading edge. */}
      <span
        aria-hidden="true"
        className={[
          "absolute left-0 w-[3px] rounded-full bg-[var(--accent-usable)]",
          "transition-[height,opacity] duration-150",
          selected ? "h-4 opacity-100" : "h-0 opacity-0",
        ].join(" ")}
      />
      <Icon className="shrink-0 text-[var(--text-secondary)]" />
      {!collapsed && <span className="truncate text-sm">{label}</span>}
    </button>
  );
}

export function NavigationView({
  selected,
  onSelect,
  collapsed,
  onToggleCollapse,
}: {
  selected: PageId;
  onSelect: (id: PageId) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const all = [...NAV_ITEMS, ...FOOTER_ITEMS];

  /** Roving tabstop: arrows move selection, matching the Settings app. */
  const onKeyDown = (e: React.KeyboardEvent) => {
    const delta = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
    if (!delta && e.key !== "Home" && e.key !== "End") return;
    e.preventDefault();

    const i = all.findIndex((n) => n.id === selected);
    const next =
      e.key === "Home"
        ? 0
        : e.key === "End"
          ? all.length - 1
          : Math.min(all.length - 1, Math.max(0, i + delta));

    onSelect(all[next].id);
    requestAnimationFrame(() => {
      listRef.current
        ?.querySelectorAll<HTMLElement>("[data-nav-item]")
        [next]?.focus();
    });
  };

  return (
    <nav
      ref={listRef}
      role="tablist"
      aria-orientation="vertical"
      aria-label="Main"
      onKeyDown={onKeyDown}
      className={[
        "flex shrink-0 flex-col gap-1 px-1 pb-2",
        "transition-[width] duration-150",
        collapsed ? "w-12" : "w-[248px]",
      ].join(" ")}
    >
      <button
        type="button"
        aria-label={collapsed ? "Expand navigation" : "Collapse navigation"}
        aria-expanded={!collapsed}
        onClick={onToggleCollapse}
        className={[
          "flex h-9 shrink-0 items-center rounded-[var(--radius-md)]",
          "text-[var(--text-secondary)] transition-colors duration-75",
          "hover:bg-[var(--subtle-hover)] active:bg-[var(--subtle-pressed)]",
          collapsed ? "justify-center" : "pl-3",
        ].join(" ")}
      >
        <MenuIcon />
      </button>

      <div className="flex flex-col gap-0.5">
        {NAV_ITEMS.map((item) => (
          <NavButton
            key={item.id}
            item={item}
            selected={selected === item.id}
            collapsed={collapsed}
            onSelect={onSelect}
          />
        ))}
      </div>

      <div className="mt-auto flex flex-col gap-0.5">
        {FOOTER_ITEMS.map((item) => (
          <NavButton
            key={item.id}
            item={item}
            selected={selected === item.id}
            collapsed={collapsed}
            onSelect={onSelect}
          />
        ))}
      </div>
    </nav>
  );
}
