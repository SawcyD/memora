import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/**
 * Resolved lazily: `getCurrentWindow()` reads Tauri's injected internals, so
 * calling it at module scope throws when the UI is loaded in a plain browser
 * (which is how the frontend is iterated on without a Rust rebuild).
 */
const appWindow = () => getCurrentWindow();

/** Fires a caption action, no-op when Tauri is not present. */
function act(method: "minimize" | "toggleMaximize" | "close") {
  try {
    void appWindow()[method]();
  } catch {
    /* browser preview */
  }
}

/**
 * Caption glyphs from Segoe Fluent Icons — the same font the Windows shell
 * draws its own caption buttons with, so the metrics match exactly.
 */
const GLYPH = {
  minimize: "",
  maximize: "",
  restore: "",
  close: "",
} as const;

function CaptionButton({
  glyph,
  label,
  onClick,
  danger = false,
}: {
  glyph: string;
  label: string;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={[
        "grid h-8 w-[46px] place-items-center text-[10px] leading-none",
        "transition-colors duration-75",
        danger
          ? "hover:bg-[#c42b1c] hover:text-white active:bg-[#c42b1c]/90"
          : "hover:bg-[var(--subtle-hover)] active:bg-[var(--subtle-pressed)]",
      ].join(" ")}
      style={{ fontFamily: '"Segoe Fluent Icons", "Segoe MDL2 Assets"' }}
    >
      {glyph}
    </button>
  );
}

export function TitleBar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let disposed = false;
    let win: ReturnType<typeof getCurrentWindow>;
    try {
      win = appWindow();
    } catch {
      return; // Browser preview: caption buttons stay inert.
    }

    const sync = () =>
      win.isMaximized().then((m) => !disposed && setMaximized(m)).catch(() => {});

    sync();
    const unlisten = win.onResized(sync);
    return () => {
      disposed = true;
      unlisten.then((f) => f()).catch(() => {});
    };
  }, []);

  return (
    <header
      data-tauri-drag-region
      className="flex h-8 shrink-0 items-center justify-between"
    >
      <div data-tauri-drag-region className="flex items-center gap-2 pl-3">
        <AppIcon />
        <span data-tauri-drag-region className="text-xs text-[var(--text-primary)]">
          Memora
        </span>
      </div>

      <div className="flex items-center">
        <CaptionButton glyph={GLYPH.minimize} label="Minimize" onClick={() => act("minimize")} />
        <CaptionButton
          glyph={maximized ? GLYPH.restore : GLYPH.maximize}
          label={maximized ? "Restore Down" : "Maximize"}
          onClick={() => act("toggleMaximize")}
        />
        <CaptionButton glyph={GLYPH.close} label="Close" danger onClick={() => act("close")} />
      </div>
    </header>
  );
}

/** The same Memora mark Windows uses for the executable and taskbar. */
function AppIcon() {
  return (
    <img
      src="/memora-icon.png"
      alt=""
      width="16"
      height="16"
      aria-hidden="true"
      className="size-4"
      draggable={false}
    />
  );
}
