import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Button,
  ComboBox,
  DataGrid,
  FluentProvider,
  InfoBar,
  InfoRow,
  NumberBox,
  ProgressBar,
  SearchBox,
  SectionHeader,
  SettingsRow,
  SettingsSection,
  ToggleSwitch,
  type DataGridSort,
  type MemoraTheme,
} from "../src";
import "./showcase.css";

const processes = [
  { id: 1, name: "File Explorer", memory: 284, status: "Running" },
  { id: 2, name: "Code", memory: 1248, status: "Running" },
  { id: 3, name: "Widgets", memory: 96, status: "Suspended" },
];

function Showcase() {
  const [theme, setTheme] = useState<MemoraTheme>("system");
  const [automatic, setAutomatic] = useState(true);
  const [threshold, setThreshold] = useState(85);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<DataGridSort>({ columnId: "memory", direction: "descending" });
  const visible = processes.filter((process) => process.name.toLowerCase().includes(query.toLowerCase()));

  return (
    <FluentProvider theme={theme} className="showcase-shell">
      <header className="showcase-header">
        <div><h1>Memora UI</h1><p>Windows 11 controls for desktop React applications.</p></div>
        <ComboBox label="Theme" value={theme} onChange={setTheme} options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]} />
      </header>
      <main className="showcase-content">
        <section>
          <SectionHeader>Memory</SectionHeader>
          <div className="showcase-memory"><div><strong>63%</strong><span>in use</span></div><ProgressBar value={63} /></div>
          <div className="showcase-info"><InfoRow label="In use" value="15.1 GB" /><InfoRow label="Available" value="8.9 GB" /><InfoRow label="Committed" value="18.4 / 31.8 GB" /></div>
          <Button accent>Optimize memory</Button>
        </section>
        <section>
          <SectionHeader>Automation</SectionHeader>
          <SettingsSection>
            <SettingsRow title="Automatic cleaning" description="Optimize when memory stays above the threshold." control={<ToggleSwitch label="Automatic cleaning" checked={automatic} onChange={setAutomatic} />} />
            <SettingsRow title="Memory threshold" description="Cleaning begins after sustained pressure." control={<NumberBox label="Memory threshold" value={threshold} min={50} max={99} suffix="%" onChange={setThreshold} />} />
          </SettingsSection>
        </section>
        <section className="showcase-wide">
          <div className="showcase-command"><SectionHeader>Processes</SectionHeader><SearchBox label="Search processes" value={query} onChange={setQuery} /></div>
          <DataGrid ariaLabel="Processes" rows={visible} rowKey={(row) => row.id} sort={sort} onSortChange={setSort} columns={[{ id: "name", header: "Name", render: (row) => row.name, sortable: true }, { id: "status", header: "Status", render: (row) => row.status }, { id: "memory", header: "Memory", render: (row) => `${row.memory} MB`, sortable: true, align: "end" }]} />
        </section>
        <section className="showcase-wide"><InfoBar title="Portable by default" message="No Tailwind setup is required. Import the package and its stylesheet, then use the provider." /></section>
      </main>
    </FluentProvider>
  );
}

createRoot(document.getElementById("root")!).render(<StrictMode><Showcase /></StrictMode>);
