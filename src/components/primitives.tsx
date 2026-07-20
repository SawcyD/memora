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

/** WinUI ToggleSwitch: 40x20 track with a sliding knob. */
export function ToggleSwitch({
  checked,
  onChange,
  disabled = false,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={[
        "relative h-5 w-10 shrink-0 rounded-full border transition-colors duration-150",
        "disabled:opacity-40",
        checked
          ? "border-transparent bg-[var(--accent-usable)]"
          : "border-[var(--stroke-control-strong)] bg-[var(--control-fill-tertiary)]",
      ].join(" ")}
    >
      <span
        aria-hidden="true"
        className={[
          "absolute top-1/2 block size-3 -translate-y-1/2 rounded-full transition-[left] duration-150",
          checked ? "left-[22px] bg-[var(--text-on-accent)]" : "left-[4px] bg-[var(--text-secondary)]",
        ].join(" ")}
      />
    </button>
  );
}

/**
 * A row in a Windows settings list: icon-free, one line of title, one of
 * description, and a control on the trailing edge.
 */
export function SettingsRow({
  title,
  description,
  note,
  control,
}: {
  title: string;
  description?: string;
  note?: React.ReactNode;
  control: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-4 border-b border-[var(--stroke-divider)] px-4 py-3 last:border-b-0">
      <div className="min-w-0 flex-1">
        <div className="text-[13px] text-[var(--text-primary)]">{title}</div>
        {description && (
          <p className="mt-0.5 text-[12px] leading-4 text-[var(--text-secondary)]">{description}</p>
        )}
        {note && <div className="mt-1 text-[12px] text-[var(--text-tertiary)]">{note}</div>}
      </div>
      {control}
    </div>
  );
}

export function SettingsSection({ children }: { children: React.ReactNode }) {
  return (
    <div className="overflow-hidden rounded-[var(--radius-lg)] border border-[var(--stroke-control)] bg-[var(--card-fill)]">
      {children}
    </div>
  );
}

export function ProgressBar({ value, max }: { value: number; max: number }) {
  const pct = max > 0 ? Math.min(100, (value / max) * 100) : 0;
  return (
    <div
      role="progressbar"
      aria-valuenow={value}
      aria-valuemin={0}
      aria-valuemax={max}
      className="h-1 w-full overflow-hidden rounded-full bg-[var(--control-fill-tertiary)]"
    >
      <div
        className="h-full rounded-full bg-[var(--accent-usable)] transition-[width] duration-150"
        style={{ width: `${pct}%` }}
      />
    </div>
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
