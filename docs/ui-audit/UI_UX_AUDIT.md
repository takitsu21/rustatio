# Rustatio UI/UX audit

Audit performed against Rustatio 2.8.0 in server mode at 1920 x 1080, with additional checks at 1440 x 900, 1024 x 768, and 390 x 844. The test torrent announces only to `127.0.0.1`, so error states were reproduced without contacting a public tracker.

## Surface inventory

- Application shell: sidebar, view switcher, instance list, product header, theme/download utilities, status strip, update prompt.
- Standard: empty torrent selector, torrent summary/details, client and transfer configuration, randomization, progressive rates, stop conditions, progress, session/total statistics, chart, logs, tracker retry states, and watch-managed instance affordance.
- Grid: summary metrics, facet filters, search, sortable/virtualized table, row context menu, selection actions, tags, import dialog, folder browser, bulk-edit dialog, and delete confirmation.
- Watch: runtime availability, folder configuration, preset summary, status tabs, search, tree/table selection, folder/file reload and deletion, and confirmation dialogs.
- Settings: general/logging, themes, built-in and custom presets, import/export/save dialogs, default preset, and detection guidance.
- Authentication: token entry, remembered-token state, verification progress, and invalid-token feedback.
- Shared states: loading, empty, ready, running, paused, idling, stopped, tracker retry/error, disabled, destructive confirmation, and responsive sidebar overlay.

## Priority findings

| Severity  | Surface / component                     | Observed problem                                                                                                                                     | UX consequence                                                                                                      | Recommendation and rationale                                                                                                                                           |
| --------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Critical  | Standard / `App.svelte`                 | The full configuration is visible before a torrent is selected. The empty state and next action occupy only half of a dense workbench.               | A first-time user must scan irrelevant controls before understanding the required first step.                       | Use a staged workbench: torrent selection first, essentials after load, advanced settings on demand. This mirrors the actual workflow and removes premature decisions. |
| Critical  | Standard / Header + StatusBar           | Status, product branding, download/theme utilities, and the state-dependent primary action compete across two horizontal layers.                     | The next action is visually detached from the instance it affects and becomes an icon at compact widths.            | Use one compact page header containing current instance identity, status, and explicitly labelled primary controls.                                                    |
| Important | Standard / configuration and statistics | Most groups use equal-weight bordered cards and nested sub-surfaces. Session, total, performance, sidebar, and progress repeat related values.       | The page is long, every region asks for attention, and monitoring requires comparison across several cards.         | Replace card stacking with sections, keep four primary metrics, and place details/chart under progressive disclosure.                                                  |
| Important | Sidebar                                 | Navigation, bulk actions, aggregate metrics, instance management, network status, and version metadata share a narrow fixed column.                  | Navigation and content selection are conflated; multi-instance use becomes cramped and visually noisy.              | Separate stable view navigation from a simplified instance list; show only identity, state, useful progress, and incidents.                                            |
| Important | Grid / toolbar and filters              | A permanent 280 px filter card remains visible even with no data, while selected actions rely heavily on icon-only buttons.                          | The table loses space and important bulk actions require icon recognition.                                          | Make Import the primary action, keep search in the page header, allow filter collapse, and use a labelled selection action bar.                                        |
| Important | Watch / `WatchView.svelte`              | Configuration, counters, global actions, selection actions, filters, overview metrics, and the explorer are presented at the same level.             | The primary task of inspecting files is pushed down and users must parse multiple control clusters.                 | Split configuration from the explorer, collapse setup once valid, and share Grid's search, status filters, and selection-action pattern.                               |
| Important | Typography / global styles              | A monospaced display font is applied to every label and paragraph.                                                                                   | Longer labels and guidance are harder to read, while operational data does not stand out from prose.                | Use the system UI stack for interface copy and reserve monospace for hashes, rates, sizes, times, and logs.                                                            |
| Important | Settings                                | General, theme, presets, and safety guidance are split into three broad tabs; theme is duplicated in a global dropdown.                              | Related preferences are harder to predict and the modal grows vertically without a stable information architecture. | Use General, Appearance, Presets, and Help navigation; retain the quick theme control while making Appearance authoritative.                                           |
| Medium    | Empty and error states                  | Grid and Watch describe absence but do not always make the recovery action primary; tracker retry expands a full metrics dashboard with zero values. | Users receive status without a decisive next step and see noisy data that has no current value.                     | Pair every empty/error state with one recovery action and suppress zero-value monitoring until meaningful.                                                             |
| Medium    | Dialogs and feedback                    | Several dialogs implement their own headers, footers, spacing, and messages; proxy settings use blocking browser alerts.                             | Interaction and keyboard behavior are inconsistent, and feedback interrupts the workflow.                           | Standardize modal composition and use inline/toast feedback with consistent focus handling.                                                                            |
| Medium    | Responsive behavior                     | At 1024 px the fixed filter/sidebar allocations constrain the table; mobile inherits desktop information order.                                      | Content becomes compressed before controls reorganize around the task.                                              | Collapse secondary rails and disclosures first, preserve the primary action and status, then stack essential fields.                                                   |
| Minor     | Theme system                            | Theme values are comprehensive but duplicated across base and Catppuccin tokens.                                                                     | Visual fixes are harder to apply consistently to all seven selectable modes including System.                       | Normalize semantic tokens while preserving every existing theme identifier.                                                                                            |

## What already works well

- Status colors are used consistently for upload, download, ratio, warning, and destructive actions.
- Grid already has filtering, virtualization, sorting, bulk editing, tagging, and destructive confirmations worth preserving.
- Watch supports keyboard row selection, shift selection, hierarchical grouping, and confirmation of destructive actions.
- Configuration persistence, presets, view-mode persistence, theme selection, and runtime capability detection are already separated from backend implementation details.
- The application exposes useful status messages and tracker retry information rather than hiding failures.
- Existing UI helpers and pure JavaScript tests provide a stable foundation for presentation refactoring.

## Screenshot index

The baseline set is stored in [`screenshots/before`](screenshots/before):

- `auth-empty-1920x1080.png`, `auth-error-1920x1080.png`
- `standard-empty-1920x1080.png`, `standard-ready-1920x1080.png`, `standard-tracker-error-1920x1080.png`, `standard-multiple-instances-1920x1080.png`
- `grid-empty-1920x1080.png`, `grid-populated-1920x1080.png`, `grid-selection-1920x1080.png`, `grid-import-dialog-1920x1080.png`
- `watch-unconfigured-1920x1080.png`
- `settings-general-1920x1080.png`, `settings-presets-1920x1080.png`, `settings-help-1920x1080.png`
- `standard-compact-1440x900.png`, `grid-compact-1024x768.png`, `standard-mobile-390x844.png`

## Audit conclusion

Rustatio already contains the required functionality, but its presentation treats configuration, operation, monitoring, and diagnostics as equally important. The redesign must change the information sequence rather than merely restyle cards: select first, configure essentials, start, monitor, then inspect details only when needed.
