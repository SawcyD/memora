import { InfoBar, SectionHeader } from "@/components/primitives";

/**
 * Pages the shell routes to but that have no implementation yet. They state
 * that plainly rather than showing sample data.
 */
export function PlaceholderPage({ title, summary }: { title: string; summary: string }) {
  return (
    <div className="max-w-[720px]">
      <SectionHeader>{title}</SectionHeader>
      <InfoBar title="Not implemented yet" message={summary} />
    </div>
  );
}
