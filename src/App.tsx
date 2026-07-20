import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { NavigationView, FOOTER_ITEMS, NAV_ITEMS } from "@/components/NavigationView";
import { TitleBar } from "@/components/TitleBar";
import { CleanerPage } from "@/pages/Cleaner";
import { AboutPage } from "@/pages/About";
import { AutomationPage } from "@/pages/Automation";
import { HistoryPage } from "@/pages/History";
import { HomePage } from "@/pages/Home";
import { MemoryPage } from "@/pages/Memory";
import { PlaceholderPage } from "@/pages/Placeholder";
import { ProcessesPage } from "@/pages/Processes";
import { SettingsPage } from "@/pages/Settings";
import type { PageId } from "@/system/types";
import { useClean } from "@/system/useClean";
import { useMemory } from "@/system/useMemory";
import { useSettings } from "@/system/useSettings";
import { useSystemTheme } from "@/system/useTheme";

/** Width below which the navigation pane auto-collapses to an icon rail. */
const COMPACT_BREAKPOINT = 860;

const SUMMARIES: Partial<Record<PageId, string>> = {};

export default function App() {
  useSystemTheme();
  const memory = useMemory();
  // Lives at the app level so a run keeps going while the user browses pages.
  const clean = useClean();
  const settings = useSettings();

  const [page, setPage] = useState<PageId>("home");
  const [userCollapsed, setUserCollapsed] = useState(false);
  const [compact, setCompact] = useState(() => window.innerWidth < COMPACT_BREAKPOINT);
  // Exclusions live in settings as process names, so they survive restarts and
  // keep protecting the same program when its pid changes.
  const excludedNames = settings.settings?.excludedProcesses ?? [];

  useEffect(() => {
    const onResize = () => setCompact(window.innerWidth < COMPACT_BREAKPOINT);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  // Tray click actions: navigate, or start an optimization from the tray.
  useEffect(() => {
    const subs = [
      listen<PageId>("tray://navigate", (e) => setPage(e.payload)),
      listen<string>("automation://run", () => setPage("cleaner")),
      listen("tray://optimize", () => {
        // Show what is happening rather than running invisibly.
        setPage("cleaner");
        clean.start(["trimWorkingSets"], { kind: "tray" });
      }),
    ].map((p) => p.catch(() => () => {}));

    return () => {
      subs.forEach((p) => p.then((f) => f()));
    };
  }, [clean]);

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
          ) : page === "memory" ? (
            <MemoryPage memory={memory} />
          ) : page === "cleaner" ? (
            <CleanerPage clean={clean} excludedNames={excludedNames} />
          ) : page === "processes" ? (
            <ProcessesPage
              excludedNames={excludedNames}
              onToggleExcluded={(name) => {
                const key = name.toLowerCase();
                settings.update({
                  excludedProcesses: excludedNames.includes(key)
                    ? excludedNames.filter((n) => n !== key)
                    : [...excludedNames, key],
                });
              }}
            />
          ) : page === "automation" ? (
            <AutomationPage state={settings} />
          ) : page === "history" ? (
            <HistoryPage />
          ) : page === "about" ? (
            <AboutPage elevated={clean.elevated} />
          ) : page === "settings" ? (
            <SettingsPage state={settings} />
          ) : (
            <PlaceholderPage title={title} summary={SUMMARIES[page] ?? ""} />
          )}
        </main>
      </div>
    </div>
  );
}
