# Sync and the Changes panel (plan)

Status: **phase 1 frontend implemented in part** (see "Implemented" below). Everything else is planned. It builds on
[Desktop Cloud v2](CLOUD-V2-DESKTOP.md), which already provides the sync collections, the stored
base version, conflict review (base, local, Cloud) and the background worker.

## Implemented

- `SyncPanel` (modal, Changes and History views), `SyncButton` (Changes label, count, disabled reason) and the
  `SyncAdapter` contract in `src/web/entities/cloud-sync/api/sync-adapter.ts`. The panel renders only the
  controls an adapter declares in `capabilities`.
- Read-only RPC `sync.v2.changes.list`: a local read of the v2 workspace (cached Cloud value, pending edit
  with its frozen base). No network, no fences, no writes.
  **Plugin settings values never leave the host.** Settings are arbitrary plugin data and may be sensitive, so
  the reply values are projected only for the credential-free `plugin-selections`, `preferences` and `variables`
  collections. Every other collection, and always `plugin-settings`, is reduced to metadata: plugin id,
  collection, state and revision of the cached and base records, the pending status, `hasNewer`, and for
  settings a sorted `changedKeys` list of top-level setting NAMES that differ from the base. The cached Cloud
  value and the pending edit value of a settings record are `null` in the reply (asserted on the serialized
  JSON in the `tiktools-core` `change_entry_tests`, the typed-client parity test and the adapter tests).
- Plugins adapter (selections and staged settings): section sync (`sync.v2.section`, Plugins only) and keep-local or
  take-Cloud for conflicts. No selective send, fetch-only, field merge, force, reset or history. A settings
  change is listed as "Changed settings: a, b" with no JSON diff and no field-level merge; resolving it keeps
  the local version or takes Cloud's through the existing review and resolve RPCs, which run on the host.
- Automations adapter (Events and Actions tabs): **compares with the last-known snapshot kept in this
  browser, not with Cloud**, because rule profiles and templates are not staged into the v2 workspace
  (only plugin selections, plugin settings and preferences are). The panel's status strip says so plainly
  ("compared with this device's reference copy, not with Cloud; nothing is sent"), and the toolbar button
  stays enabled while signed out because the diff is local. It offers reset and **mark as reviewed**: an
  explicit `markReviewed` capability (declared in the `SyncAdapter` contract, false by default and hidden by
  the panel when absent) with a confirmed header action that advances the reference copy to the current local
  records and clears the count. It discards the ability to reset to the old reference. Without it the
  reference is taken the first time a section is seen and only moves when a transport exists.

What the plan's table claimed for automations (push, fetch, merge, force, history) needs backend work first:
native adapters for the `profiles` and `rule-templates` records, staging from the Owner library operations,
reviewed application of fetched profiles, per-record push and fetch, and the snapshots table of phase 2.

### Desktop simplification and diagnostics (2026-11-13)

Settings contains account/session, diagnostics and collaboration, with synchronization
inside the shared Changes panel. The retired `CloudSyncEditor` was removed. Plugins
requests plugin catalog records only; templates/profiles remain in Automations.
Optional Driver.js tours are lazy loaded from `?` and follow real Teams state.

`sync.v2.section { section: "plugins" }` publishes only plugin selections/settings.
Downloads cache all collections before advancing the shared account cursor; other
queues remain durable and are excluded from application candidates. Older hosts
cannot silently fall back to whole-workspace publication.

Migration diagnostics distinguish an incomplete inventory from missing packages.
Only an absent automatically derived bundled-plugin root may be omitted. Explicit
roots remain strict. Inventory recovery previews exact missing package IDs and
archives their install flags in SQLite before a reviewed local repair. It preserves
Cloud queues, plugin data and conflicts. Tested on a copy of the affected profile;
no migration was run against the production account.

The AppImage bundles the GStreamer plugins and scanner used by its media libraries;
packaging verification checks every required plugin. Host-specific helper paths
are configured before GTK starts.

Teams matches this PC against its persisted host/device binding. Approved-team
lookup uses owner nicknames. Invitation recipient search returns only nicknames,
blocks ambiguous aliases, and binds redemption to the selected account; it still
requires host approval. Backend migration: RemakeBackend
`20261113000001_remote_invitation_recipients.sql` (additive).

### Verification and remaining work

- Optimized Linux desktop built; AppImage and DEB packaging verified. Final AppImage
  SHA256: `556c2f750b1972aefcd33ac7a8467b3aff1b21fb6e4ffbc5ca084efa17316898`.
- 167 sync-engine tests and 146 targeted frontend tests passed; TypeScript/ESLint
  passed. Latest Teams layout/recipient tests passed after the final layout change.
- Real PostgreSQL test verifies nickname-only results, duplicate detection and
  recipient-bound redemption. Two-PC native onboarding and native relay fallback
  tests passed against an isolated backend, including approval and revocation.
- Real AppImage opened with a copy of the affected profile. Plugins, compact
  preparation and Teams layout were inspected. The original profile was untouched.
- The original log also records SQLite BUSY errors while connecting the LIVE,
  followed by collaboration-retention failures. The original lock holder is still
  unknown. Extended SQLite reason codes are now logged without SQL/data values;
  a competing-writer test proves a failed batch is preserved and retried once the
  write lock is released. This does not establish the historical lock's cause.
- Automations still uses a local comparison snapshot. Reviewed application of
  downloaded settings/other section records and Variables/Overlays adapters remain
  separate implementation work; the section-scoped publication change does not
  claim those transports exist.
- Backend code/migration has not been deployed. The initial 4.2.0 validation did
  not activate Cloud v2. During the later RC1 GUI test, v2 activation executed on
  the copied profile. This operation has account-wide scope: the server policy
  was confirmed active afterwards, but no baseline was captured to determine
  whether it was already required. The six workspace conflicts remain intact;
  original profile files were untouched.

### 4.2.1-rc.1 reproduction

The Linux prerelease is published at
<https://github.com/nglmercer/TikTools-app/releases/tag/v4.2.1-rc.1>.
The downloaded AppImage, DEB and portable archive match the uploaded checksums.
The actual AppImage reports `4.2.1-rc.1`; bundled `appsink`, WAV and H.264 tests
passed. GitHub Actions is disabled, so these Linux artifacts were built locally.

The GUI reproduced "The changes could not be loaded" after v2 activation.
A diagnostic linked against the compiled core, without starting the background
sync worker, reads all six changes and conflicts sequentially. With 32 concurrent
read-only `sync_v2_changes_list` calls, 3 succeed and 29 fail with
`conflict: another sync or cutover operation is active`. These reads currently
capture their identity through `with_product_local_edit`, which reserves the
exclusive mutation admission. The toolbar counter and panel also issue reads
independently. Fix the read admission and retain exact session/consent revalidation;
do not remove write fences or hide the failure behind an empty successful list.
The panel should also expose the native error and offer a local-read retry.

The production backend responds `200` at `/api/v2/capabilities` and announces v2,
while the new `/api/v3/remote/people` route responds `404`. The existing remote
hosts route responds `401` without credentials. Nickname search therefore still
requires deployment of backend commit `6a69c99` and its additive migration.

## Goal

A git-like but simplified way to see what differs from Cloud and to push, fetch, reset and merge it,
reused by every section that syncs. It is **not** one global list: each section shows only its own
elements.

## Shape

- One reusable modal, `SyncPanel`, opened from a button (icon plus a count) in the toolbar of each
  tab that syncs. Automations can also embed it. It takes a `scope` and a `SyncAdapter`.
- A `SyncAdapter` is how a section describes itself: list changes, read both sides of one element
  (rendered with `BaseJsonDiff`), push, fetch, reset an element to the synced version, and history.
  It declares what it supports (`branches`, `fork`, `merge`, `force`, `history`) and the panel hides
  everything else.
- Panel: Changes and History as two views of the same panel (like GitHub Desktop), a checklist of
  elements to send, a diff on the right, a sync button, an auto-sync switch, and a menu with force
  push, force pull and reset all (each asks first).

## Per section

| Section | Elements | Backend today | Supported |
| --- | --- | --- | --- |
| Automations: events, actions | records inside rule profiles | `profiles` collection (events and actions travel inside a profile) | push, fetch, reset, merge, force, history |
| Automations: templates | rule templates | `rule-templates` collection and the Cloud library | same, plus fork |
| Automations: branches | rule profiles act as branches | `profiles` | create from current, compare, merge |
| Plugins | selections (enabled, version), plugin settings | `plugin-selections`; `plugin-settings` staging is still pending in the v2 doc | push, fetch, reset, force (no merge, no fork) |
| Overlays | Cloud overlay documents | not a sync collection; documents with revisions | local changes versus last revision, save new revision, fetch, reset (no merge, no force) |

Fork applies to templates only: copy a public template to "My documents", then publish it through the
existing review flow (draft, pending, published). Plugins are signed packages, so their equivalents
are the stable and beta channel and pinning or restoring a version. Overlays get no fork until
Cloud has a public overlay gallery.

## Phases

1. Frontend only, with the RPC that exists (`sync.*`, `sync.v2.*`): panel, adapters for automations
   and plugins, diff with the JSON view, reset to the base version, auto-sync preference. Needs one
   read of the stored base version of a record, which today is only readable inside a conflict.
2. Backend: per-record history (a snapshots table written on save and on sync) for "restore a
   previous version" and "revert an operation"; branch provenance (`parentId`) on profiles, which
   changes the portable contract here and in RemakeBackend.
3. Overlays adapter on top of the Cloud overlay revisions.

## TODO (not implemented)

- [ ] **Variables sync.** Host done, UI pending. The `variables` collection exists in `cloud-contracts` and
  RemakeBackend (types text, number, boolean, list; **secret never syncs**), is announced in
  `sync.optionalCollections`, and the host stages it with `sync.v2.stageVariables` (secrets are excluded in the
  host, contract violations are reported by name, an instance without the capability gets
  `capability_unavailable`). `sync.v2.changes.list` projects variable values (credential-free by contract).
  Details: [Desktop Cloud v2](CLOUD-V2-DESKTOP.md#optional-collection-variables-host-done-ui-pending).
  Still to do: the Variables tab adapter in `AutomationsPage.vue` (typed local variables, a secret type that
  is never offered to the host as a value), applying pulled variables to the runtime globals (today globals
  are text-only and the host does not apply them), conflict behaviour per type in the UI, and a test of the
  generic conflict review/resolve on a `variables` key. Until the adapter exists the Variables tab shows no
  sync button.
- [x] Three-way conflict view in the panel (per-field Base/Local/Cloud choice, all local / all Cloud, apply
  merge). Only adapters that declare `merge` show it; no current adapter can stage a merged value.
- [ ] Per-record history and restore (phase 2).
- [ ] Branch provenance and a public-template fork action (phase 2).
- [ ] Overlay revisions adapter (phase 3).
- [ ] Decide whether auto-sync is stored locally or in the `preferences` collection.
