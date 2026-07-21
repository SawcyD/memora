import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Button,
  ComboBox,
  InfoBar,
  NumberBox,
  SectionHeader,
  SettingsRow,
  SettingsSection,
  ToggleSwitch,
} from "@/components/primitives";
import type { AutomationRule, Settings, Trigger } from "@/system/types";
import type { SettingsState } from "@/system/useSettings";

function describeTrigger(t: Trigger): string {
  switch (t.kind) {
    case "usageAbove":
      return `When memory stays above ${t.percent}% for ${t.sustainedSecs} seconds`;
    case "scheduled":
      return `Every ${t.everyMins} minutes`;
    case "systemIdle":
      return `After ${t.idleMins} minutes with no keyboard or mouse input`;
  }
}

export function AutomationPage({ state }: { state: SettingsState }) {
  const { settings, error, update } = state;
  const [suspended, setSuspended] = useState<string[]>([]);
  const [minimizeMonitorAvailable, setMinimizeMonitorAvailable] = useState<boolean | null>(null);

  const refreshSuspended = useCallback(() => {
    invoke<string[]>("suspended_rules")
      .then(setSuspended)
      .catch(() => setSuspended([]));
  }, []);

  useEffect(() => {
    refreshSuspended();
    const id = setInterval(refreshSuspended, 5000);
    return () => clearInterval(id);
  }, [refreshSuspended]);

  useEffect(() => {
    invoke<boolean>("minimize_monitor_available")
      .then(setMinimizeMonitorAvailable)
      .catch(() => setMinimizeMonitorAvailable(false));
  }, []);

  if (!settings) {
    return (
      <div className="max-w-[720px]">
        <SectionHeader>Automation</SectionHeader>
        {error ? <InfoBar title="Unavailable" message={error} /> : <p>Loading…</p>}
      </div>
    );
  }

  const a = settings.automation;
  const m = settings.minimizeTrim;
  const profile = a.profiles.find((p) => p.name === a.activeProfile) ?? a.profiles[0];

  const patchAutomation = (patch: Partial<Settings["automation"]>) =>
    update({ automation: { ...a, ...patch } });

  const patchMinimizeTrim = (patch: Partial<Settings["minimizeTrim"]>) =>
    update({ minimizeTrim: { ...m, ...patch } });

  const patchRule = (id: string, patch: Partial<AutomationRule>) =>
    patchAutomation({
      profiles: a.profiles.map((p) =>
        p.name !== profile.name
          ? p
          : { ...p, rules: p.rules.map((r) => (r.id === id ? { ...r, ...patch } : r)) },
      ),
    });

  const patchTrigger = (id: string, patch: Partial<Trigger>) =>
    patchAutomation({
      profiles: a.profiles.map((p) =>
        p.name !== profile.name
          ? p
          : {
              ...p,
              rules: p.rules.map((r) =>
                r.id === id ? { ...r, trigger: { ...r.trigger, ...patch } as Trigger } : r,
              ),
            },
      ),
    });

  return (
    <div className="max-w-[720px] pb-6">
      <SectionHeader>Automation</SectionHeader>

      {error && (
        <div className="mb-3">
          <InfoBar title="That change was not saved" message={error} />
        </div>
      )}

      {/* The honest framing, stated before any switch is offered. */}
      <div className="mb-4">
        <InfoBar
          title="Automatic cleaning rarely helps, and can hurt"
          message="Trimming moves pages to the standby list rather than freeing them, so the gain fades as programs resume. A rule that fires whenever memory is high can loop: trim, fade, trim again. Memora enforces a cooldown and switches off any rule that keeps recovering little, but the safest setting is still off."
        />
      </div>

      {!a.enabled && (
        <div className="mb-4">
          <InfoBar
            title="Automatic cleaning is currently off"
            message="Turn on Run optimizations automatically below. In the Balanced profile, sustained usage above 85% will then run the safe working-set trim after 60 seconds."
          />
        </div>
      )}

      <SettingsSection>
        <SettingsRow
          title="Run optimizations automatically"
          description="Off by default. Manual optimization is unaffected by this."
          control={
            <ToggleSwitch
              label="Run optimizations automatically"
              checked={a.enabled}
              onChange={(v) => patchAutomation({ enabled: v })}
            />
          }
        />
        <SettingsRow
          title="Active profile"
          description="Gaming has no rules on purpose: trimming during a game causes the stutter the profile is meant to avoid."
          control={
            <ComboBox
              label="Active profile"
              value={a.activeProfile}
              options={a.profiles.map((p) => ({ value: p.name, label: p.name }))}
              onChange={(v) => patchAutomation({ activeProfile: v })}
            />
          }
        />
        <SettingsRow
          title="Minimum time between runs"
          description="The main protection against repeated trimming. Cannot be set below 5 minutes."
          control={
            <NumberBox
              label="Minimum minutes between runs"
              value={Math.round(profile.minIntervalSecs / 60)}
              min={5}
              max={720}
              suffix="min"
              onChange={(v) =>
                patchAutomation({
                  profiles: a.profiles.map((p) =>
                    p.name === profile.name ? { ...p, minIntervalSecs: v * 60 } : p,
                  ),
                })
              }
            />
          }
        />
      </SettingsSection>

      <h3 className="mb-2 mt-5 text-[13px] font-semibold">
        Rules in {profile.name}
      </h3>

      {profile.rules.length === 0 ? (
        <InfoBar
          title="This profile has no rules"
          message="Nothing will run automatically while it is active."
        />
      ) : (
        <SettingsSection>
          {profile.rules.map((r) => (
            <SettingsRow
              key={r.id}
              title={describeTrigger(r.trigger)}
              description={
                suspended.includes(r.id)
                  ? "Paused automatically: recent runs recovered very little memory. Your system may simply be using the memory it has."
                  : undefined
              }
              note={
                <div className="flex items-center gap-3">
                  {r.trigger.kind === "usageAbove" && (
                    <>
                      <NumberBox
                        label="Usage threshold"
                        value={r.trigger.percent}
                        min={50}
                        max={99}
                        suffix="%"
                        onChange={(v) => patchTrigger(r.id, { percent: v } as Partial<Trigger>)}
                      />
                      <NumberBox
                        label="Sustained for"
                        value={r.trigger.sustainedSecs}
                        min={30}
                        max={3600}
                        suffix="s"
                        onChange={(v) =>
                          patchTrigger(r.id, { sustainedSecs: v } as Partial<Trigger>)
                        }
                      />
                    </>
                  )}
                  {r.trigger.kind === "systemIdle" && (
                    <NumberBox
                      label="Idle minutes"
                      value={r.trigger.idleMins}
                      min={1}
                      max={720}
                      suffix="min"
                      onChange={(v) => patchTrigger(r.id, { idleMins: v } as Partial<Trigger>)}
                    />
                  )}
                  {r.trigger.kind === "scheduled" && (
                    <NumberBox
                      label="Interval minutes"
                      value={r.trigger.everyMins}
                      min={5}
                      max={1440}
                      suffix="min"
                      onChange={(v) => patchTrigger(r.id, { everyMins: v } as Partial<Trigger>)}
                    />
                  )}
                  {suspended.includes(r.id) && (
                    <Button
                      onClick={async () => {
                        await invoke("resume_rule", { rule: r.id });
                        refreshSuspended();
                      }}
                    >
                      Resume
                    </Button>
                  )}
                </div>
              }
              control={
                <ToggleSwitch
                  label={describeTrigger(r.trigger)}
                  checked={r.enabled}
                  onChange={(v) => patchRule(r.id, { enabled: v })}
                />
              }
            />
          ))}
        </SettingsSection>
      )}

      <p className="mt-4 text-[12px] leading-4 text-[var(--text-secondary)]">
        Automatic runs appear on the Cleaner page while they happen and can be cancelled like any
        other. Every run, and every rule that matched but was held back, is recorded in History.
      </p>

      <h3 className="mb-2 mt-6 text-[13px] font-semibold">Experimental: trim when minimized</h3>

      <div className="mb-3">
        <InfoBar
          title="Use only for apps you understand"
          message="Memora can reduce a selected app's working set after it stays minimized. This may lower its visible memory temporarily, but returning to the app can cause disk reads or a brief pause while Windows reloads pages. Restoring before the delay cancels the action."
        />
      </div>

      {minimizeMonitorAvailable === false && (
        <div className="mb-3">
          <InfoBar
            title="Minimize monitoring is unavailable in this session"
            message="Memora could not register the Windows event listener. Other monitoring and cleaning features still work; restart Memora before enabling this experiment."
          />
        </div>
      )}

      <SettingsSection>
        <SettingsRow
          title="Trim selected applications when minimized"
          description="Off by default. Add applications from the Processes page context menu."
          control={
            <ToggleSwitch
              label="Trim selected applications when minimized"
              checked={m.enabled}
              disabled={minimizeMonitorAvailable === false}
              onChange={(enabled) => patchMinimizeTrim({ enabled })}
            />
          }
        />
        <SettingsRow
          title="Wait before trimming"
          description="Restoring the application during this delay cancels the trim."
          control={
            <NumberBox
              label="Wait before trimming"
              value={m.delaySecs}
              min={3}
              max={60}
              suffix="s"
              onChange={(delaySecs) => patchMinimizeTrim({ delaySecs })}
            />
          }
        />
        <SettingsRow
          title="Minimum working set"
          description="Small applications are skipped because trimming them provides little benefit."
          control={
            <NumberBox
              label="Minimum working set"
              value={m.minimumWorkingSetMb}
              min={50}
              max={16384}
              suffix="MB"
              onChange={(minimumWorkingSetMb) => patchMinimizeTrim({ minimumWorkingSetMb })}
            />
          }
        />
        <SettingsRow
          title="Per-application cooldown"
          description="Prevents repeated minimize and restore cycles from constantly trimming the same app."
          control={
            <NumberBox
              label="Per-application cooldown"
              value={Math.round(m.cooldownSecs / 60)}
              min={1}
              max={60}
              suffix="min"
              onChange={(minutes) => patchMinimizeTrim({ cooldownSecs: minutes * 60 })}
            />
          }
        />
      </SettingsSection>

      <h3 className="mb-2 mt-5 text-[13px] font-semibold">Selected applications</h3>
      {m.applications.length === 0 ? (
        <InfoBar
          title="No applications selected"
          message="Open Processes, right-click an application, and choose Trim when minimized."
        />
      ) : (
        <SettingsSection>
          {m.applications.map((name) => (
            <SettingsRow
              key={name}
              title={name}
              description={
                settings.excludedProcesses.includes(name)
                  ? "Skipped because this application is excluded from cleaning."
                  : "Eligible after it remains minimized and passes the working-set threshold."
              }
              control={
                <Button
                  onClick={() =>
                    patchMinimizeTrim({
                      applications: m.applications.filter((item) => item !== name),
                    })
                  }
                >
                  Remove
                </Button>
              }
            />
          ))}
        </SettingsSection>
      )}
    </div>
  );
}
