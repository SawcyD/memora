/** WinUI SearchBox: 32px tall, leading glyph, clear button when non-empty. */
export function SearchBox({
  value,
  onChange,
  placeholder = "Search",
  label,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  label: string;
}) {
  return (
    <div className="relative w-[260px]">
      <svg
        viewBox="0 0 16 16"
        width="14"
        height="14"
        aria-hidden="true"
        className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--text-tertiary)]"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.2"
      >
        <circle cx="7" cy="7" r="4.5" />
        <path d="m10.5 10.5 3 3" strokeLinecap="round" />
      </svg>

      <input
        type="search"
        aria-label={label}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className={[
          "h-8 w-full rounded-[var(--radius-md)] border pl-8 pr-2 text-[13px]",
          "border-[var(--stroke-control)] bg-[var(--control-fill)]",
          "text-[var(--text-primary)] placeholder:text-[var(--text-tertiary)]",
          // Windows underlines the focused field in the accent colour.
          "outline-none focus:border-b-2 focus:border-b-[var(--accent-usable)]",
        ].join(" ")}
      />
    </div>
  );
}
