# Rustatio frontend redesign plan

## Architecture

1. Establish shared tokens and primitives: compact buttons and fields, visible focus, section headers, disclosures, page headers, status chips, empty states, action bars, modal structure, and inline feedback.
2. Replace the current double header with a single application shell. Keep `standard`, `grid`, and `watch` store values unchanged and keep Settings as an overlay destination.
3. Extract a Standard workbench from `App.svelte` while leaving API calls, polling, persistence, and instance orchestration unchanged.
4. Align Grid and Watch around the same page-header, filtering, selection, empty-state, and action hierarchy.
5. Restructure Settings and Auth, then complete responsive and theme passes.

## Surface behavior

### Standard

- No torrent: show a focused welcome/selection state and short three-step explanation; hide configuration and stop conditions.
- Ready: show torrent identity, essential client/rate settings, stop policy, and a labelled Start session action. Place timing, initial state, randomization, progressive rates, and torrent internals in disclosures.
- Running/paused/idling: keep state and Pause/Resume/Stop visible; prioritize current rates, ratio, progress, and elapsed time. Collapse setup while retaining access.
- Error/retry: show the tracker issue beside the instance identity with a clear retry status; do not let empty metrics compete with recovery information.
- Logs: retain the existing preference and log stream in a compact collapsible diagnostics section.

### Navigation and instances

- Keep a 240 px expanded / 64 px collapsed desktop rail and the current mobile overlay behavior.
- Present view navigation first, a dedicated Instances section second, and network/version utilities last.
- Each instance row shows name, status, meaningful progress/ratio, and one incident indicator. Bulk controls appear only when useful and use labels.

### Grid

- Use a page title, live summary strip, primary Import action, global search, filter toggle, and instance count.
- Keep the table dense and central. Desktop filters may remain beside it only when explicitly expanded; compact windows use an overlay/panel.
- When rows are selected, show a labelled action bar for Start, Pause/Resume, Stop, Edit, Tag, and Delete.
- Preserve sorting, virtualization, row context actions, import modes, folder browsing, presets, and bulk-edit payloads.

### Watch

- Use the same header/action/filter grammar as Grid.
- Put watch-folder configuration in a disclosure with an explicit configured/unconfigured summary.
- Make the file explorer the main surface, with concise counts, visible selection scope, and labelled bulk actions.
- Preserve all runtime restrictions, path depth, auto-start, reload, delete, tree expansion, and confirmation behavior.

### Settings, Auth, and dialogs

- Settings navigation becomes General, Appearance, Presets, and Help. Web proxy settings live in General/Network and only render in WASM mode.
- Preserve all theme IDs and preset operations. Built-in presets use a compact comparison list rather than repeated large cards.
- Auth remains a single-task page with clearer hierarchy, inline error feedback, and the shared theme selector.
- Standardize modal header, content, footer, Escape handling, initial focus, focus return, and destructive emphasis.

## Visual system

- System UI font for prose and controls; monospace only for operational values.
- Spacing scale: 4, 8, 12, 16, and 24 px. Default control height: 36 px. Radius: 6-8 px.
- Use flat grouped sections and separators before elevated cards; never nest decorative cards.
- Preserve Light, Dark, Latte, Frappé, Macchiato, and Mocha through semantic tokens for canvas, surface, border, text, muted text, primary, success, warning, danger, upload, download, and ratio.
- Meet WCAG AA contrast, keep a 2 px focus ring, explicit labels, `aria-live` status updates, keyboard-operable disclosures, and 32-36 px minimum targets.

## Implementation order

1. Tokens, shared primitives, app shell, and header/status consolidation.
2. Standard empty/ready/running/error workbench and simplified instance rail.
3. Grid header, filters, table container, selection actions, and dialogs.
4. Watch configuration/explorer hierarchy.
5. Settings, Auth, proxy, logs, confirmations, and update feedback.
6. All-theme and responsive passes, screenshot comparison, accessibility review, and final cleanup.

## Validation

- Automated: frontend JavaScript tests, component tests, Svelte check, ESLint, Prettier check, and Vite build.
- Manual: select/configure/start/pause/resume/stop, tracker retry, multi-instance selection, Grid import/edit/tag/delete, Watch configuration/reload/delete, presets, Auth, theme switching, and modal keyboard behavior.
- Visual: reproduce the meaningful baseline set under `screenshots/after`, including 1920 x 1080, 1440 x 900, 1024 x 768, and mobile web checks.
- Boundary: final diff may contain only `ui/**` and `docs/ui-audit/**`; backend contracts and files remain untouched.
