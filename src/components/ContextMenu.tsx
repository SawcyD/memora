import { useEffect, useLayoutEffect, useRef, useState } from "react";

export interface MenuAction {
  id: string;
  label: string;
  /** Renders in the destructive style and is separated from the safe actions. */
  danger?: boolean;
  disabled?: boolean;
}

/**
 * A Windows-style context menu on an acrylic surface — one of the few places
 * the design rules allow blur, since it is a transient surface.
 */
export function ContextMenu({
  x,
  y,
  actions,
  onSelect,
  onDismiss,
}: {
  x: number;
  y: number;
  actions: MenuAction[];
  onSelect: (id: string) => void;
  onDismiss: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  // Flip the menu back inside the window when it would overflow, the way the
  // shell does rather than letting it clip.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      x: x + width > window.innerWidth ? Math.max(0, x - width) : x,
      y: y + height > window.innerHeight ? Math.max(0, y - height) : y,
    });
  }, [x, y]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onDismiss();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onDismiss]);

  const items = actions.filter((a) => !a.danger);
  const dangerous = actions.filter((a) => a.danger);

  const render = (a: MenuAction) => (
    <button
      key={a.id}
      type="button"
      role="menuitem"
      disabled={a.disabled}
      onClick={() => {
        onSelect(a.id);
        onDismiss();
      }}
      className={[
        "flex h-8 w-full items-center rounded-[var(--radius-md)] px-3 text-left text-[13px]",
        "transition-colors duration-75 disabled:opacity-40",
        "hover:bg-[var(--subtle-hover)] active:bg-[var(--subtle-pressed)]",
        a.danger ? "text-[#c42b1c] dark:text-[#ff99a4]" : "text-[var(--text-primary)]",
      ].join(" ")}
    >
      {a.label}
    </button>
  );

  return (
    <>
      {/* Dismiss layer: a click anywhere else closes the menu. */}
      <div
        className="fixed inset-0 z-40"
        onMouseDown={onDismiss}
        onContextMenu={(e) => {
          e.preventDefault();
          onDismiss();
        }}
      />
      <div
        ref={ref}
        role="menu"
        style={{ left: pos.x, top: pos.y }}
        className={[
          "fixed z-50 min-w-[180px] rounded-[var(--radius-lg)] p-1",
          "border border-[var(--stroke-surface)] bg-[var(--acrylic-fill)] backdrop-blur-xl",
          "shadow-[var(--shadow-flyout)]",
        ].join(" ")}
      >
        {items.map(render)}
        {dangerous.length > 0 && items.length > 0 && (
          <div className="my-1 h-px bg-[var(--stroke-divider)]" role="separator" />
        )}
        {dangerous.map(render)}
      </div>
    </>
  );
}
