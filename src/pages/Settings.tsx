import {
  ComboBox,
  InfoBar,
  NumberBox,
  SectionHeader,
  SettingsRow,
  SettingsSection,
  ToggleSwitch,
} from "@/components/primitives";
import type { ClickAction, Settings } from "@/system/types";
import type { SettingsState } from "@/system/useSettings";

const INTERVALS = [1, 2, 5, 10, 30].map((s) => ({
  value: s,
  label: s === 1 ? "1 second" : `${s} seconds`,
}));

const CLICK_ACTIONS: { value: ClickAction; label: string }[] = [
  { value: "none", label: "Do nothing" },
  { value: "openMemora", label: "Open Memora" },
  { value: "openMemoryPage", label: "Open the Memory page" },
  { value: "optimize", label: "Optimize memory" },
];

export function SettingsPage({ state }: { state: SettingsState }) {
  const { settings, error, update } = state;

  if (!settings) {
    return (
      <div className="max-w-[720px]">
        <SectionHeader>Settings</SectionHeader>
        {error ? (
          <InfoBar title="Settings unavailable" message={error} />
        ) : (
          <p className="text-[13px] text-[var(--text-secondary)]">Loading…</p>
        )}
      </div>
    );
  }

  const toggle = (key: keyof Settings, title: string, description: string) => (
    <SettingsRow
      title={title}
      description={description}
      control={
        <ToggleSwitch
          label={title}
          checked={Boolean(settings[key])}
          onChange={(v) => update({ [key]: v } as Partial<Settings>)}
        />
      }
    />
  );

  return (
    <div className="max-w-[720px] pb-6">
      <SectionHeader>Settings</SectionHeader>

      {error && (
        <div className="mb-3">
          <InfoBar title="That change was not saved" message={error} />
        </div>
      )}

      <h3 className="mb-2 mt-4 text-[13px] font-semibold">Tray icon</h3>
      <SettingsSection>
        {toggle(
          "showTrayPercentage",
          "Show memory percentage in tray icon",
          "The meter ring always shows usage. The number is secondary and can be hard to read at small tray sizes.",
        )}
        <SettingsRow
          title="Tray icon update interval"
          description="How often the icon and tooltip refresh. The graph always samples every second."
          control={
            <ComboBox
              label="Tray icon update interval"
              value={settings.trayIntervalSecs}
              options={INTERVALS}
              onChange={(v) => update({ trayIntervalSecs: v })}
            />
          }
        />
      </SettingsSection>

      <h3 className="mb-2 mt-5 text-[13px] font-semibold">Usage thresholds</h3>
      <SettingsSection>
        <SettingsRow
          title="Elevated"
          description="Above this, usage is considered elevated."
          control={
            <NumberBox
              label="Elevated threshold"
              value={settings.warningThreshold}
              min={1}
              max={98}
              suffix="%"
              onChange={(v) => update({ warningThreshold: v })}
            />
          }
        />
        <SettingsRow
          title="High"
          description="The icon leaves the accent colour at this point."
          control={
            <NumberBox
              label="High threshold"
              value={settings.highThreshold}
              min={2}
              max={99}
              suffix="%"
              onChange={(v) => update({ highThreshold: v })}
            />
          }
        />
        <SettingsRow
          title="Critical"
          description="Thresholds are kept in ascending order; values that would cross are adjusted."
          control={
            <NumberBox
              label="Critical threshold"
              value={settings.criticalThreshold}
              min={3}
              max={100}
              suffix="%"
              onChange={(v) => update({ criticalThreshold: v })}
            />
          }
        />
      </SettingsSection>

      <h3 className="mb-2 mt-5 text-[13px] font-semibold">Tray actions</h3>
      <SettingsSection>
        <SettingsRow
          title="Single-click"
          control={
            <ComboBox
              label="Single-click action"
              value={settings.singleClick}
              options={CLICK_ACTIONS}
              onChange={(v) => update({ singleClick: v })}
            />
          }
        />
        <SettingsRow
          title="Double-click"
          control={
            <ComboBox
              label="Double-click action"
              value={settings.doubleClick}
              options={CLICK_ACTIONS}
              onChange={(v) => update({ doubleClick: v })}
            />
          }
        />
        <SettingsRow
          title="Middle-click"
          description="Optimizing affects the whole system, so this does nothing by default."
          control={
            <ComboBox
              label="Middle-click action"
              value={settings.middleClick}
              options={CLICK_ACTIONS}
              onChange={(v) => update({ middleClick: v })}
            />
          }
        />
      </SettingsSection>

      <h3 className="mb-2 mt-5 text-[13px] font-semibold">Window</h3>
      <SettingsSection>
        {toggle(
          "closeToTray",
          "Close to tray",
          "Closing the window keeps Memora running so the tray meter stays live. Exit from the tray menu.",
        )}
        {toggle(
          "minimizeToTray",
          "Minimize to tray",
          "Minimizing also removes Memora from the taskbar.",
        )}
        {toggle(
          "startWithWindows",
          "Start Memora with Windows",
          "Adds Memora to your account's startup list. This does not affect other users of this PC.",
        )}
        {toggle(
          "showOptimizationNotifications",
          "Show optimization notifications",
          "Shows a Windows notification with the measured result after an optimization. Failures are always reported, whatever this is set to.",
        )}
      </SettingsSection>
    </div>
  );
}
