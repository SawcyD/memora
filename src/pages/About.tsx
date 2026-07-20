import { InfoRow, SectionHeader, SettingsSection } from "@/components/primitives";

/**
 * Deliberately plain. The About page states what Memora does and, just as
 * importantly, what it does not — a memory tool is the kind of software users
 * are right to be sceptical of.
 */
export function AboutPage({ elevated }: { elevated: boolean }) {
  return (
    <div className="max-w-[640px]">
      <SectionHeader>About Memora</SectionHeader>

      <SettingsSection>
        <div className="px-4 py-3">
          <InfoRow label="Version" value="0.1.0" />
          <InfoRow label="Running as administrator" value={elevated ? "Yes" : "No"} />
        </div>
      </SettingsSection>

      <h3 className="mb-2 mt-5 text-[13px] font-semibold">What Memora does</h3>
      <p className="text-[13px] leading-5 text-[var(--text-secondary)]">
        Memora reports physical memory using the same Windows counters Task Manager reads, and can
        ask Windows to trim process working sets.
      </p>

      <h3 className="mb-2 mt-4 text-[13px] font-semibold">What trimming actually does</h3>
      <p className="text-[13px] leading-5 text-[var(--text-secondary)]">
        Trimming a working set does not free memory. It moves pages to the standby list, where they
        stay in RAM and are loaded back as processes resume. Available memory rises immediately and
        then decays, which is why every result is measured again after 30 seconds and recorded in
        History.
      </p>
      <p className="mt-2 text-[13px] leading-5 text-[var(--text-secondary)]">
        High memory usage is usually Windows working correctly: unused RAM is wasted RAM, and the
        cache exists to make things faster. Memora will not tell you your PC is unhealthy, and it
        does not report cached or standby memory as freed.
      </p>

      <h3 className="mb-2 mt-4 text-[13px] font-semibold">Where your data goes</h3>
      <p className="text-[13px] leading-5 text-[var(--text-secondary)]">
        Nowhere. Settings and history are plain files in your local app data folder. Memora makes no
        network connections and collects no telemetry.
      </p>
    </div>
  );
}
