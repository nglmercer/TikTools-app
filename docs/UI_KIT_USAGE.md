# UI kit and layout patterns

The current Vue kit lives in `src/web/shared/ui/`. Import individual components
through `@/shared/ui/...`. `app/styles/tokens.css` owns visual values. `/ui-kit`
is a development-only interactive catalogue; its sample data is demonstration
content, not a connection to the user's account or plugins.

## Visual language and density

Use flat `--color-bg-app`, `--color-bg-panel`, `--color-bg-panel-header` and
`--color-bg-field` surfaces, 1px semantic borders and 2–3px token radii. Use
`--color-accent` for selection/primary actions and semantic status colors for
meaning. Do not add gradients, per-feature hex colors, pill cards or large
shadows. Floating surfaces use `--shadow-popup`; ordinary panels have no shadow.
Use `--color-border-focus` for visible keyboard focus. Color never replaces a
status label or action name.

Desktop controls are compact. On coarse-pointer devices (including hybrid
laptops), or below 700px, control height tokens become 44px; the application
chrome grows with them. Layout and input density are independent:

- Below 600px, forms stack labels above controls.
- Below 900px, workspaces use list/detail navigation and the sidebar drawer.
- Diagnostics retain their dedicated page below 768px.
- Features may use another breakpoint when their content requires it; explain
  that reason instead of overriding a shared component's internal layout.

Support reduced motion. Inputs on phones retain 16px text to avoid browser
zoom. Never hide a required action behind hover on touch devices.

## Existing primitives: extend rather than duplicate

| Need | Component |
| --- | --- |
| Action / icon action | `button/BaseButton.vue`, `button/BaseIconButton.vue` |
| Text / secret / numeric input | `text-field/BaseTextField.vue`, `spin-box/BaseSpinBox.vue` |
| Selection | `combo-box/BaseComboBox.vue`, `search-select/`, `segmented/`, `tabs/` |
| Form layout | `form/BaseFormGrid.vue`, `form/BaseFormRow.vue` |
| Semantically related settings | `group-box/BaseGroupBox.vue` (fieldset/legend) |
| Workspace region | `panel/BasePanel.vue` |
| Collection | `list/BaseList.vue`, `table/BaseTable.vue` |
| Modal confirmation | `dialog/BaseDialog.vue`, `dialog/BaseConfirmDialog.vue` |
| Empty result / inline notice | `empty-state/BaseEmptyState.vue`, `alert/BaseAlert.vue` |
| Command toolbar | `toolbar/BaseToolbar.vue` (arrow-key button navigation) |
| Resizable panes | `splitter/BaseSplitter.vue` (pointer and keyboard) |

Controlled fields use Vue `v-model`; keep domain validation in the feature.
`BaseFormRow` owns the visible label and field help/error. Associate help and
error with the actual input, not its decorative wrapper. Do not duplicate
labels in both a wrapper and a field. Composite controls need a group label.
The row’s `required` adds its marker and ARIA semantics; native HTML constraint
validation remains owned by the actual input.

`BasePanel` accepts `title`, `subtitle`, `flush` and `headerLayout` (`auto`,
`inline`, `stacked`). Slots are default body, `actions` and `footer`. `auto`
wraps actions and stacks them on narrow screens; `inline` is an explicit
opt-in for headers whose contents fit. Titles may wrap without losing text.

## Shared compositions

- `status/BaseStatus.vue`: labelled semantic state. Keep domain state-to-tone
  mapping in the feature. `BaseStatusAction` adds an accessible action;
  diagnostics navigation remains in `DiagnosticStatusAction`.
- `floating-panel/BaseFloatingPanel.vue`: nonmodal movable surface with keyboard
  movement, viewport limits and focus restoration. Do not replace consent
  dialogs with it. Feed it translated labels; never trap focus as a modal.
- `layout/MasterDetailLayout.vue`: desktop splitter and narrow master/detail
  presentation. Keep both subtrees alive across layout changes so drafts are
  retained. `back` is a request; the feature owns dirty-draft confirmation.
- `collection-toolbar/CollectionToolbar.vue`: labelled group with `search`,
  default/filter and `actions` slots and optional count. Unlike a command
  toolbar, native inputs keep ordinary Tab navigation.
- `settings-row/SettingsRow.vue`: consistent setting label, description,
  control and optional supplementary content. Use it inside a group; keep
  settings writes in the feature's handler/store.

## Ownership and migration

`shared/ui` has no account, plugin, router or store knowledge. Features own
operations/permissions, entities own contracts/state, widgets compose domain
features, and pages choose layout. Plugin-specific UI remains isolated behind
the existing declarative renderer or sandbox/broker boundary; shared visual
patterns never add privileges.

The initial migrations cover diagnostic/status surfaces, Plugins and
Automations workspaces, settings/updater/Cloud, and Rankings form wrappers.
Cloud views display only values exposed by the existing contracts: no invented
base/local/cloud comparison, device roster or sync capability. Conflict choices,
permission consent, overwrite confirmation and validation remain explicit.

## Verification for each change

1. Add the relevant interactive states to `/ui-kit` (EN/ES): populated, empty,
   loading, invalid and disabled as applicable.
2. Test behavior that can regress: keyboard/focus, field associations, retained
   drafts/selection, return navigation, disabled operations and consent.
3. Run `bun x --no-install vitest run`, `bun x --no-install vue-tsc --noEmit`,
   `bun x --no-install eslint src/web --max-warnings=0`, and
   `bun scripts/task.ts frontend-build`.
4. Inspect actual desktop and mobile UI in EN/ES, long labels, touch targets,
   reduced motion, zoom/reflow and overflow. Label demonstration fixtures as
   such; do not present them as a successful live integration.

## Overlay library

The Overlays page opens `OverlayCloudPanel` in a `BaseDialog`; the design
workspace stays mounted underneath. The modal uses a source sidebar, a
search/type/sort row, preview cards and a selection summary with an explicit
open action. Keep the footer outside the scrollable dialog body.

Library previews use `OverlayLibraryPreview` and the SDK's existing preview
mode with synthetic sample events. They show the included widgets and open
the existing local editor; they are not public Cloud catalogue entries.
“My overlays” uses the capability-gated account list. Package operations retain
their existing feature implementation. Portable package tools are no longer
exposed in the overlay workspace. Do not add style filters or publication badges without supporting metadata.

Use the existing app/panel surfaces, selection border, accent, focus and radius
tokens rather than copying the mockup's brighter blue or rounded corners.
Below 900px the source navigation wraps above the collection. Below 700px
the modal fills the viewport, navigation becomes a labelled selector and cards
stack; only its content scrolls. The main workspace uses a widget selector below
800px, keeping the preview and properties accessible without the desktop list. The UI kit includes overlay preview, empty, loading, failure and
disabled-action examples in the active locale.

### File selection and image uploads

Use `file-upload/BaseFileUpload.vue` for a single binary file, with explicit
`accept` and `maxBytes` limits. Picker and drop share validation; a selected file
shows its name, size and image thumbnail. Selection never uploads automatically.
`CloudImageUpload` owns the upload action and busy state. The backend still
validates file contents. Use `BaseFileDrop` for existing JSON/text imports and
clipboard operations. The UI kit includes a binary image upload example.

Combo-box popups opened inside a modal must be teleported into the open dialog,
so native top-layer and inert behavior keep them visible and interactive.
Keep decorative introductions and repeated instructions out of the overlay
workspace; preserve validation, errors and accessible field labels.

## Collaboration settings and grants

Keep Teams in its capability-gated settings tab rather than appending it below
remote-host configuration. Collaboration uses tabs for status, device records,
invitation creation and direct transport. Navigation retains unsaved drafts and
makes no configuration writes. Permission categories use tabs; exact resource
access uses labelled All / Selected IDs / None segmented controls.

Bulk permission actions edit the local grant draft only. Enable-all selects
known capabilities without widening resource, action, settings, points or
external-effect restrictions. Disable-all also removes reusable automation
authorization so the draft remains valid. Owner approval and grant-application
confirmation remain explicit. The UI kit contains an isolated editable grant
example with a disabled state. Mobile tab rows scroll horizontally; the primary
settings tabs use a compact two-column layout on narrow phones.

## Composition boundaries for complex views

Split views by user task and state ownership rather than moving an entire
component into one large composable. Collaboration's root owns lifecycle,
approval/revocation confirmations and writes; overview, direct connection,
records and invitation components receive data and emit actions. Grant panels
share immutable capability/delegation edits while keeping resource-selection
and raw-JSON drafts local. Explicit reset signals preserve preset and JSON
apply behavior across the panels.

SyncPanel owns its adapter and dialog-lifetime state. The sidebar and diff pane
render narrow state interfaces; diff and merge choices are initialized in the
root's lifecycle so closing a dialog cannot silently discard them. Cloud account
sync separately owns one account/session invalidation generation shared by
legacy recovery, v2 protocol/migration and v2 conflict hooks. Those hooks never
subscribe to account changes independently.

Team access, devices, invitations and membership use the existing Teams store;
the root alone starts refresh and clears secret tokens on unmount. Automation
record editor components emit saves/deletes to the page's existing guarded
workflow. Read-only library projections and the history-menu registry have
separate composables.

Feature styles shared by extracted components live in namespaced feature CSS
files, loaded by the root. Restrict native-control rules to explicit classes;
do not broaden formerly scoped rules to every descendant input or label.

The controller proposal builder owns the preview capture and the only call that
applies generated JSON to the proposal model. Its draft composable owns field
values, source loading and identity resets for the builder lifetime, so changing
conditional field panels cannot discard input. Action fields, event fields and
action selection receive narrow state interfaces. Pure field validators preserve
size, URL, identity and numeric bounds independently of rendering. Shared proposal
styles remain scoped in each component rather than becoming global selectors.

Plugin connection layouts share one editor-owned autosave, model-refresh timer
and provisioning dialog. Stacked and sectioned panels render the same draft and
emit edits/test/connection intents; switching layouts does not restart the save
debounce or duplicate connection requests. The presentation hook only projects
schemas, icons and localized status text, and service notes are shared by both
layouts. Credentials continue to report intent to the editor through its slot.

Automation record sessions own the page-lifetime editor, guarded selection and
profile navigation, version comparisons and recovery invalidation. The page
keeps template/profile dialogs, route handling and persisted view preferences.
The mobile sheet toolbar receives the same history-panel refs as the desktop
menu; it never mounts another set of recovery panels. Toggle confirmation is a
controlled view of the existing review model, including its stale-read gate.

The live feed keeps delivery identity, cursor acceptance, bounded staging and
transport subscriptions in its state coordinator. Backfill recovery owns one
serialized, generation-fenced paging loop; gift/like coalescing only folds visual
rows after acceptance. The viewport owns saved preferences, scroll restoration
and its frames/listeners. Pausing commits accepted staging before disabling the
presentation, and disposal cancels each module's own resources.

Plugin details keep tab availability, one-shot catalog requests, pending test
state and uninstall confirmation in their root. The general pane projects
metadata and capabilities. The connection pane owns its editor ref and flushes
pending settings before reporting a probe request to the root; it does not
create a second plugin store or reset pending checks when changing tabs.

EventEditor owns its single cloned draft, baseline/dirty comparison, save/test/fire
commands and deletion confirmation. Conditions and their template picker, action
selection/order and cooldown sections emit field changes back to that draft.
They never save or replace the whole event independently, so edits in one section
preserve changes from the others and Discard restores the shared baseline.

Automation state composes a coherent snapshot/version/host projection, a bounded
recent-run buffer, a checked record gateway and explicit test/fire execution.
All transport listeners still use one root-owned subscription client. The root
alone invalidates refresh generations and installs library snapshots; it owns
the shared error state. Gateway helpers receive its refresh callbacks and the original shared native
journals instead of opening independent mutation paths.

AutomationLibrary owns area visits, query/sort models and the committed-state
checkbox reset while a toggle waits for review. Its read-only row projection
combines filters, localized labels and recent-run metadata. Event/action tables
emit selection and toggle intent without mutating records. Keep extra areas
mounted after their first visit so changing tables cannot discard their drafts.

Approval review keeps one model and all journal/selection/version observers in
the root. Saved-source, manifest/subject, decision and history panels render
captured data and report user intent. Consent refs and activate/revoke/recovery
calls remain root-owned, so remounting a panel cannot create another authority
path or retain independent consent after the host, revisions or dialog change.

Rule-template dialogs own navigation and one dialog-lifetime draft/import pair.
The catalog only projects availability and reports a selection; the configuration
view edits the existing draft fields. Applying retains the generated candidate
IDs after a failed attempt and invalidates them only when the draft changes under
the existing working-state rules. Manual import stages parsed templates separately
and reports completion through the caller's callback.

Profile document parsing, parameter layering/bounded expansion and default-pack
resolution live in the pure rule-profile library. Export conversion is separate
from the local sidecar decoder and stateful profile operations. Document readers
and preview code import the pure helpers directly; the original state module
re-exports its public contract for existing callers. Native atomic changes stay
in the profile store and its shared library journal.

AppShell keeps runtime startup, global diagnostics and the single Owner library
review mounted at the application level. Navigation projects built-in/plugin
pages and synchronizes the route with the existing navigation store. One shell
interaction hook owns the drawer, media/key listeners and touch disposers.
Notifications keep their dismissal state in an always-mounted shell child.

The control client coordinates the transport, presentation revision/generation
and subscriber delivery. Its request queue owns bounded admission, byte limits,
serialization, deadlines, cancellation and response settlement. Contracts/errors
and metadata validation are independent modules; the original client entry point
re-exports the public API so transports and callers retain one error identity.
Detach and transport failures drain that same queue through one cleanup path.

ExploreList owns catalog refresh, pagination, hidden/installed filtering and
loading/error/import notices. Its row component renders the kind, availability
and action controls, then reports open/install/import intent to the list. It
never initializes another catalog store or starts a library mutation itself.

SchemaFieldControl dispatches simple control kinds and delegates option, media
and text/template controls to focused renderers. Picker errors and variable-menu
state remain in one field-lifetime helper. Renderers report typed value changes
without creating their own field draft; media references retain host metadata,
and variable popups use the live anchor from the rendered text control.
