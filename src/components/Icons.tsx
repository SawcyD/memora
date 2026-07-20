/**
 * Fluent System Icons, drawn as 16px regular-weight strokes to match the
 * Segoe Fluent Icons metrics. One consistent set — do not mix in other styles.
 */
type IconProps = { className?: string };

function Glyph({ children, className }: IconProps & { children: React.ReactNode }) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.1"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      className={className}
    >
      {children}
    </svg>
  );
}

export const HomeIcon = (p: IconProps) => (
  <Glyph {...p}>
    <path d="M2.5 7.2 8 2.8l5.5 4.4V13a.5.5 0 0 1-.5.5H3a.5.5 0 0 1-.5-.5V7.2Z" />
    <path d="M6.5 13.5v-4h3v4" />
  </Glyph>
);

export const MemoryIcon = (p: IconProps) => (
  <Glyph {...p}>
    <rect x="3.5" y="3.5" width="9" height="9" rx="1" />
    <rect x="6" y="6" width="4" height="4" rx="0.5" />
    <path d="M6 3.5v-2M10 3.5v-2M6 14.5v-2M10 14.5v-2M3.5 6h-2M3.5 10h-2M14.5 6h-2M14.5 10h-2" />
  </Glyph>
);

export const ProcessesIcon = (p: IconProps) => (
  <Glyph {...p}>
    <rect x="1.5" y="2.5" width="13" height="11" rx="1" />
    <path d="M1.5 5.5h13M5.5 5.5v8" />
  </Glyph>
);

export const CleanerIcon = (p: IconProps) => (
  <Glyph {...p}>
    <path d="M9.5 1.8 13 5.3 6.4 11.9a1 1 0 0 1-.7.3H3.2l-.3-2.5a1 1 0 0 1 .3-.8L9.5 1.8Z" />
    <path d="M8 3.3 11.5 6.8M2.5 14.5h11" />
  </Glyph>
);

export const AutomationIcon = (p: IconProps) => (
  <Glyph {...p}>
    <circle cx="8" cy="8" r="2.2" />
    <path d="M8 1.5v1.8M8 12.7v1.8M14.5 8h-1.8M3.3 8H1.5M12.6 3.4l-1.3 1.3M4.7 11.3l-1.3 1.3M12.6 12.6l-1.3-1.3M4.7 4.7 3.4 3.4" />
  </Glyph>
);

export const HistoryIcon = (p: IconProps) => (
  <Glyph {...p}>
    <path d="M2.6 8a5.4 5.4 0 1 0 1.6-3.8" />
    <path d="M2 3v2.6h2.6" />
    <path d="M8 5.2V8l2 1.4" />
  </Glyph>
);

export const SettingsIcon = (p: IconProps) => (
  <Glyph {...p}>
    <circle cx="8" cy="8" r="1.9" />
    <path d="M13.2 9.4a5.6 5.6 0 0 0 0-2.8l1.2-.9-1.4-2.4-1.4.6a5.6 5.6 0 0 0-2.4-1.4L9 1.2H6.2l-.2 1.5a5.6 5.6 0 0 0-2.4 1.4l-1.4-.6L.8 5.9l1.2.9a5.6 5.6 0 0 0 0 2.4l-1.2.9 1.4 2.4 1.4-.6a5.6 5.6 0 0 0 2.4 1.4l.2 1.5H9l.2-1.5a5.6 5.6 0 0 0 2.4-1.4l1.4.6 1.4-2.4-1.2-.9Z" />
  </Glyph>
);

export const AboutIcon = (p: IconProps) => (
  <Glyph {...p}>
    <circle cx="8" cy="8" r="6" />
    <path d="M8 7.2v4M8 4.9v.1" />
  </Glyph>
);

export const MenuIcon = (p: IconProps) => (
  <Glyph {...p}>
    <path d="M2 4h12M2 8h12M2 12h12" />
  </Glyph>
);
