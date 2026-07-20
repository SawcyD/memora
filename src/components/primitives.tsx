/** Shared Fluent primitives. Windows naming, Windows metrics. */

export function Button({
  children,
  accent = false,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { accent?: boolean }) {
  return (
    <button
      type="button"
      {...rest}
      className={[
        "h-8 rounded-[var(--radius-md)] border px-3 text-sm",
        "transition-colors duration-75 disabled:opacity-40",
        accent
          ? "border-transparent bg-[var(--accent-usable)] text-[var(--text-on-accent)] hover:opacity-90 active:opacity-80"
          : "border-[var(--stroke-control)] bg-[var(--control-fill)] hover:bg-[var(--control-fill-secondary)] active:bg-[var(--control-fill-tertiary)]",
      ].join(" ")}
    >
      {children}
    </button>
  );
}

/** A label/value pair in the compact two-column layout Task Manager uses. */
export function InfoRow({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-[3px]">
      <span className="text-[13px] text-[var(--text-secondary)]">{label}</span>
      <span className="tabular text-[13px] text-[var(--text-primary)]">{value}</span>
    </div>
  );
}

export function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="mb-2 text-[14px] font-semibold text-[var(--text-primary)]">{children}</h2>
  );
}

/** WinUI InfoBar — used here for the not-yet-built pages. */
export function InfoBar({ title, message }: { title: string; message: string }) {
  return (
    <div
      role="status"
      className="flex gap-3 rounded-[var(--radius-lg)] border border-[var(--stroke-control)] bg-[var(--card-fill)] p-3"
    >
      <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" className="mt-0.5 shrink-0">
        <circle cx="8" cy="8" r="6.5" fill="none" stroke="var(--accent-usable)" strokeWidth="1.1" />
        <path
          d="M8 7.2v4M8 4.9v.1"
          stroke="var(--accent-usable)"
          strokeWidth="1.1"
          strokeLinecap="round"
        />
      </svg>
      <div>
        <div className="text-[13px] font-semibold">{title}</div>
        <p className="mt-0.5 text-[13px] text-[var(--text-secondary)]">{message}</p>
      </div>
    </div>
  );
}
