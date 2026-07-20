# Memora — Native Windows 11 Memory Manager

Build a Windows desktop application named Memora using Tauri v2, Rust, React, TypeScript, and Tailwind CSS.

Memora must look and behave like a real Windows 11 system utility. The interface must not resemble a generic AI-generated SaaS dashboard. Avoid oversized cards, excessive gradients, random glowing elements, floating widgets, excessive rounded rectangles, and decorative charts that do not provide useful information.

The application should appear as though Microsoft designed it alongside Task Manager, Windows Security, Settings, and PowerToys.

## Native Windows 11 Design Requirements

Follow Microsoft's Windows 11 Fluent design language closely. Use:

- Segoe UI Variable as the primary font.
- Windows 11 spacing, sizing, and visual hierarchy.
- Compact information density similar to Task Manager.
- Native-looking toggles, buttons, dropdowns, tooltips, context menus, dialogs, and navigation.
- Windows system accent color.
- Light and dark themes that follow the Windows appearance setting.
- Mica for the main application background where supported.
- Acrylic only for temporary surfaces such as menus and dialogs.
- Subtle borders separating surfaces.
- Small corner radiuses consistent with Windows 11.
- Native Windows shadows and elevation.
- Smooth but restrained animations.
- Windows-style focus indicators and keyboard navigation.
- Proper scaling from 100% through 250% DPI.

Do not use glassmorphism as the primary design style. Do not blur every panel or place every statistic inside a large card.

## Visual References

The interface should draw inspiration from:

- Windows 11 Task Manager
- Windows 11 Settings
- Windows Security
- Microsoft PowerToys
- Windows 11 File Explorer properties dialogs

It should not copy these applications exactly, but it must use similar spacing, typography, control sizing, navigation behavior, and information density.

## Application Window

Use a native-looking Windows 11 title bar containing:

- Memora application icon
- "Memora" title
- Minimize button
- Maximize or restore button
- Close button

Support Windows Snap Layouts when hovering over the maximize button.

The application should remember: window size, window position, maximized state, selected page, sidebar state.

Closing the window should optionally minimize Memora to the system tray instead of terminating it.

## Navigation

Use a Windows 11 Settings-style navigation sidebar.

Navigation items:

```text
Home
Memory
Processes
Cleaner
Automation
History
Settings
```

The bottom of the sidebar should contain:

```text
About Memora
```

The sidebar should:

- Collapse at smaller window sizes.
- Show icons and text while expanded.
- Show icons only while collapsed.
- Use Windows-style selection indicators.
- Support keyboard navigation.
- Avoid oversized icons and excessive spacing.

## Home Page

The Home page should present the most important information without looking like a web analytics dashboard.

At the top, show:

```text
Memory

63% in use
15.1 GB of 24.0 GB
8.9 GB available
```

Include a clean memory usage graph similar to Task Manager.

Below the graph, show a compact two-column information layout:

```text
In use                 15.1 GB
Available               8.9 GB
Committed        18.4 / 31.8 GB
Cached                   5.2 GB
Paged pool               642 MB
Non-paged pool           811 MB
Memory speed          3200 MT/s
Slots used                2 of 2
```

Include one primary action:

```text
Optimize memory
```

The action should use the Windows system accent color. Do not place multiple large call-to-action buttons on the page.

## Memory Page

Create a detailed Task Manager-style memory view. Show:

- Total physical memory
- In-use memory
- Available memory
- Cached memory
- Committed memory
- Compressed memory
- Paged pool
- Non-paged pool
- Standby memory
- Modified memory
- Free memory
- Hardware-reserved memory

Include a real-time graph with selectable ranges:

```text
60 seconds
5 minutes
30 minutes
1 hour
```

Use simple lines and labels. Do not add unnecessary gradients, shadows, or animations.

## Processes Page

Use a compact native-style data grid.

Columns:

```text
Name
PID
Status
Memory
Private memory
Commit
CPU
Threads
Handles
```

Support: sorting, searching, filtering, column resizing, column visibility, multi-selection, right-click context menus, keyboard navigation, process icons, expandable grouped processes.

The right-click menu should contain:

```text
Trim memory
Exclude from cleaning
Open file location
View properties
Copy details
End task
```

Dangerous actions such as ending a process must require confirmation.

Protected or inaccessible processes should be marked clearly instead of causing errors or freezing the application.

## Cleaner Page

Present cleaning options as a native Windows settings list rather than a collection of oversized cards.

Each cleaning method should have: name, brief explanation, estimated risk, toggle, learn-more link.

Options may include:

```text
Trim inactive process working sets
Trim selected process working sets
Clear standby memory
Clear modified page list
Combine memory lists
Clear system file cache
```

Experimental or potentially disruptive methods must be disabled by default and clearly labeled.

The primary button should read:

```text
Optimize now
```

During cleaning, show: current process, progress bar, processes completed, processes skipped, memory recovered, cancel button.

The window must remain responsive throughout the operation.

## Cleaning Results

After optimization, show a Windows-style results dialog or page:

```text
Memory optimization completed

Available before        6.7 GB
Available after         8.4 GB
Immediately recovered   1.7 GB
Still available after
30 seconds               1.3 GB

Processes trimmed          18
Processes skipped           7
Errors                      0
Duration                 0.8 s
```

Include a collapsible details section listing each affected process.

Do not report cached or standby memory as permanently "freed." Clearly distinguish temporary working-set reduction from genuinely increased available memory.

## Dynamic System Tray Icon

Memora must include a Windows system tray icon that displays current physical memory usage in real time.

### Tray Icon Appearance

The icon should behave like a small memory meter. Use a simple circular or vertical meter that fills based on current RAM usage:

- Empty or low fill for low usage
- Half-filled around 50%
- Mostly filled around 75%
- Fully filled near 100%

Use the Windows system accent color where practical.

The icon must remain readable at common tray sizes:

```text
16 × 16
20 × 20
24 × 24
32 × 32
```

Generate the icon dynamically instead of using several unrelated static icons.

Where readable, show the current percentage inside the icon:

```text
42
67
91
```

Because tray icons are small, the meter shape is the primary indicator. The number is secondary and may be hidden at resolutions where it becomes unreadable.

Handle 100% usage using either `99+` or a completely filled critical icon.

### Tray Update Behavior

Update the tray icon every two seconds by default. Allow the user to choose:

```text
1 second
2 seconds
5 seconds
10 seconds
30 seconds
```

Do not recreate the entire tray object on every update. Update only the icon and tooltip to avoid flickering, unnecessary allocations, or Explorer instability.

Generate tray icon images on a background worker and cache recently used percentage states. For example, icons may be cached for percentages from 0 through 100.

Round visual updates to whole percentages to prevent needless redraws.

### Tray Tooltip

Hovering over the icon should display:

```text
Memora
Memory: 63%
Used: 15.1 GB
Available: 8.9 GB
```

If Windows restricts multiline tooltips, use:

```text
Memora — Memory 63% — 15.1 of 24.0 GB used
```

### Tray Usage States

Use restrained state changes:

```text
0–69%       Normal
70–84%      Elevated
85–94%      High
95–100%     Critical
```

Do not constantly flash or animate the tray icon.

A critical state may use a small warning indicator, but it must still match the Windows 11 visual style.

Allow users to customize thresholds.

### Tray Context Menu

Right-clicking the tray icon should open a native Windows-style context menu:

```text
Open Memora
Optimize memory
────────────────
Memory usage: 63%
Available: 8.9 GB
────────────────
Active profile
  Balanced
  Gaming
  Development
────────────────
Pause automatic cleaning
Start Memora with Windows
Settings
Exit Memora
```

The memory information entries should be disabled informational menu items.

### Tray Interactions

- Single-click: Open the Memora window.
- Double-click: Open Memora directly to the Memory page.
- Middle-click: Run memory optimization, if enabled in settings.
- Right-click: Open the tray context menu.

All click behaviors should be configurable.

When optimization runs from the tray:

1. Change the tray tooltip to indicate that cleaning is in progress.
2. Show subtle progress through the icon when possible.
3. Restore the live memory meter when completed.
4. Display a native Windows notification with measured results.

Example:

```text
Memora finished optimizing memory

Available memory increased by 1.2 GB.
18 processes were trimmed in 0.7 seconds.
```

### Tray Settings

Include settings for:

```text
Show memory percentage in tray icon
Show graphical memory meter
Tray icon update interval
Memory warning threshold
Critical memory threshold
Single-click action
Double-click action
Middle-click action
Minimize to tray
Close to tray
Show optimization notifications
Start with Windows
```

## Native Notifications

Use Windows toast notifications rather than custom notification popups.

Notifications should be used for: high memory usage, completed optimizations, failed optimizations, automatic profile changes, repeated memory growth that may indicate a leak.

Avoid sending notifications for routine status updates.

## Accessibility

Memora must support: keyboard-only navigation, Windows screen readers, high-contrast mode, reduced-motion settings, text scaling, DPI scaling, clear focus indicators, accessible names for icon-only controls, sufficient color contrast.

The interface must remain understandable without relying only on color.

## UI Restrictions

Do not create:

- A generic SaaS dashboard
- Oversized statistic cards
- Neon gradients
- Glowing borders
- Excessive glass panels
- Floating pill-shaped navigation
- Random decorative charts
- Huge headings
- Excessive whitespace
- Emoji as interface icons
- Mobile-style bottom navigation
- Fake performance statistics
- "Your PC is unhealthy" scare messaging
- Large animated speedometers
- Unnecessary AI chat interfaces

Use Fluent-compatible icons or another consistent Windows-style icon set. Do not mix unrelated icon styles.

## Technical UI Standard

Create reusable components for:

```text
NavigationView
SettingsRow
SettingsSection
CommandBar
InfoBar
ProgressBar
TeachingTip
ContentDialog
ContextMenu
DataGrid
SearchBox
ToggleSwitch
NumberBox
ComboBox
Tooltip
```

Component naming and behavior should follow Windows terminology where reasonable.

Keep application state and system monitoring separate from presentation components.

## Final Design Goal

Memora should look like a legitimate Windows 11 utility installed alongside Microsoft PowerToys. A user should not immediately assume it was produced from a generic AI-generated frontend template.

The tray icon should provide useful memory information even when the main window is closed, allowing users to monitor current RAM usage and run an optimization without opening the full application.
