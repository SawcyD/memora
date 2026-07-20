import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { NavigationView, FOOTER_ITEMS, NAV_ITEMS } from "@/components/NavigationView";
import { TitleBar } from "@/components/TitleBar";
import { CleanerPage } from "@/pages/Cleaner";
import { HomePage } from "@/pages/Home";
import { PlaceholderPage } from "@/pages/Placeholder";
import { ProcessesPage } from "@/pages/Processes";
import type { PageId } from "@/system/types";
import { useClean } from "@/system/useClean";
import { useMemory } from "@/system/useMemory";
import { useSystemTheme } from "@/system/useTheme";

/** Width below which the navigation pane auto-collapses to an icon rail. */
const COMPACT_BREAKPOINT = 860;

const SUMMARIES: Partial<Record<PageId, string>> = {
  memory: "The detailed memory breakdown and multi-range graph are not built yet.",
  automation: "Profiles and automatic cleaning rules are not built yet.",
  history: "Optimization history is not recorded yet.",
  settings: "Settings, including tray behavior, are not built yet.",
  about: "Memora 0.1.0",
};

export default function App() {
  useSystemTheme();
  const memory = useMemory();
  // Lives at the app level so a run keeps going while the user browses pages.
  const clean = useClean();

  const [page, setPage] = useState<PageId>("home");
  const [userCollapsed, setUserCollapsed] = useState(false);
  const [compact, setCompact] = useState(() => window.innerWidth < COMPACT_BREAKPOINT);
  // Owned here so the Processes page and the Cleaner agree on what is excluded.
  const [excluded, setExcluded] = useState<number[]>([]);

  useEffect(() => {
    const onResize = () => setCompact(window.innerWidth < COMPACT_BREAKPOINT);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  // Double-clicking the tray icon opens Memora straight to a given page.
  useEffect(() => {
    const unlisten = listen<PageId>("tray://navigate", (e) => setPage(e.payload)).catch(
      () => () => {},
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // A narrow window forces the rail; above the breakpoint the user's choice wins.
  const collapsed = compact || userCollapsed;

  const title = [...NAV_ITEMS, ...FOOTER_ITEMS].find((n) => n.id === page)?.label ?? "";

  return (
    <div className="flex h-full flex-col">
      <TitleBar />

      <div className="flex min-h-0 flex-1">
        <NavigationView
          selected={page}
          onSelect={setPage}
          collapsed={collapsed}
          onToggleCollapse={() => setUserCollapsed((v) => !v)}
        />

        {/* Content is its own layer: rounded top-left corner and a hairline
            border, the way Settings separates the pane from the content. */}
        <main
          id="memora-content"
          role="tabpanel"
          aria-label={title}
          tabIndex={-1}
          className={[
            "min-w-0 flex-1 overflow-y-auto",
            "rounded-tl-[var(--radius-lg)] border-l border-t border-[var(--stroke-divider)]",
            "bg-[var(--layer-fill)] px-9 py-6",
          ].join(" ")}
        >
          {page === "home" ? (
            <HomePage memory={memory} onOptimize={() => setPage("cleaner")} />
          ) : page === "cleaner" ? (
            <CleanerPage clean={clean} excluded={excluded} />
          ) : page === "processes" ? (
            <ProcessesPage
              excluded={excluded}
              onToggleExcluded={(pid) =>
                setExcluded((s) => (s.includes(pid) ? s.filter((p) => p !== pid) : [...s, pid]))
              }
            />
          ) : (
            <PlaceholderPage title={title} summary={SUMMARIES[page] ?? ""} />
          )}
        </main>
      </div>
    </div>
  );
}
