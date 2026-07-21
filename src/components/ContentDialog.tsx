import { useEffect, useRef } from "react";
import { Button } from "./primitives";

/**
 * WinUI ContentDialog: modal, dimmed backdrop, actions bottom-right with the
 * primary on the left. Used for confirming destructive actions.
 */
export function ContentDialog({
  title,
  children,
  primaryText,
  onPrimary,
  onCancel,
  destructive = false,
}: {
  title: string;
  children: React.ReactNode;
  primaryText: string;
  onPrimary: () => void;
  onCancel: () => void;
  destructive?: boolean;
}) {
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Focus moves into the dialog so keyboard users are not left behind it.
    panel.current?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
      if (e.key !== "Tab") return;

      // Trap focus: a modal that lets Tab escape is not modal.
      const focusable = panel.current?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable?.length) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-[var(--smoke-fill)]">
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={[
          "w-[min(440px,calc(100vw-48px))] rounded-[var(--radius-lg)]",
          "border border-[var(--stroke-surface)] bg-[var(--acrylic-fill)] backdrop-blur-xl",
          "shadow-[var(--shadow-dialog)]",
        ].join(" ")}
      >
        <div className="p-6">
          <h2 className="mb-3 text-[20px] font-semibold leading-6">{title}</h2>
          <div className="text-[13px] leading-5 text-[var(--text-secondary)]">{children}</div>
        </div>

        <div className="flex gap-2 border-t border-[var(--stroke-divider)] p-4">
          <div className="flex-1" />
          <Button accent={!destructive} onClick={onPrimary}>
            {primaryText}
          </Button>
          <Button onClick={onCancel}>Cancel</Button>
        </div>
      </div>
    </div>
  );
}
