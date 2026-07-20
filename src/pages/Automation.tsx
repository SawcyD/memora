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

  if (!settings) {
    return (
      <div className="max-w-[720px]">
        <SectionHeader>Automation</SectionHeader>
        {error ? <InfoBar title="Unavailable" message={error} /> : <p>Loading…</p>}
      </div>
    );
  }

  const a = settings.automation;
  const profile = a.profiles.find((p) => p.name === a.activeProfile) ?? a.profiles[0];

  const patchAutomation = (patch: Partial<Settings["automation"]>) =>
    update({ automation: { ...a, ...patch } });

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
    </div>
  );
}
