# @memora/ui

The reusable React UI system extracted from Memora. It packages the compact Windows 11 Fluent look, theme tokens, keyboard behavior, accessibility states, and component CSS without requiring Tailwind in the consuming project.

## Install privately

Build and create a portable tarball from the Memora repository:

```bash
npm run ui:build
npm run ui:pack
```

Install the generated `memora-ui-0.1.0.tgz` in another project:

```bash
npm install /path/to/memora/memora-ui-0.1.0.tgz
```

For active local development, install the package folder directly:

```bash
npm install /path/to/memora/packages/memora-ui
```

The package is marked `UNLICENSED` and configured for restricted publishing. To use GitHub Packages or another private registry, rename the npm scope to your account or organization and authenticate that registry before publishing.

## Use

```tsx
import {
  Button,
  FluentProvider,
  SettingsRow,
  SettingsSection,
  ToggleSwitch,
} from "@memora/ui";
import "@memora/ui/styles.css";

export function Settings() {
  const [enabled, setEnabled] = useState(true);

  return (
    <FluentProvider theme="system" accentColor="#0078d4">
      <SettingsSection>
        <SettingsRow
          title="Automatic cleaning"
          description="Run after sustained memory pressure."
          control={
            <ToggleSwitch
              label="Automatic cleaning"
              checked={enabled}
              onChange={setEnabled}
            />
          }
        />
      </SettingsSection>
      <Button variant="primary">Save changes</Button>
    </FluentProvider>
  );
}
```

`FluentProvider` supports `system`, `light`, and `dark` themes; `compact` and `comfortable` densities; and any CSS accent color. All component classes are prefixed with `memora-`, so the stylesheet can coexist with an app's own CSS or utility framework.

## Components

- Foundations: `FluentProvider`, theme and density types
- Controls: `Button`, `IconButton`, `ToggleSwitch`, `ComboBox`, `NumberBox`, `SearchBox`
- Layout: `SettingsSection`, `SettingsRow`, `SectionHeader`, `InfoRow`, `CommandBar`, `CommandGroup`
- Feedback: `ProgressBar`, `InfoBar`, `Tooltip`, `TeachingTip`, `ContentDialog`
- Navigation and data: `NavigationView`, `DataGrid`
- Menus: `ContextMenu`

Every icon slot accepts a React node, so the package does not force an icon dependency on consumer applications.

## Visual development

```bash
npm run ui:demo
```

This starts the included component showcase at `http://localhost:4178`. The demo is intentionally excluded from the published tarball.
