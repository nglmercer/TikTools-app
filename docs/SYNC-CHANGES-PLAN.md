# Sync and the Changes panel (plan)

Status: **shared panel, Plugins and reviewed Variables implemented; Automations Cloud,
history and Overlay revisions still pending** (see the completion checklist). It builds on
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
Cloud queues, plugin data and conflicts. The initial 4.2.0 validation used a copy
without migration; the later RC1 account-policy event is documented below.

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

### Read contention correction (RC2)

Local Changes/conflict lists and protocol selection now hold a shared protocol
read lease instead of exclusive product mutation admission. Strict consent,
account scope and both v2 markers are checked on blocking threads. The final
exact-session check awaits the short account mutex; it rejects logout, pending
transitions and a new login to the same account. Reading files does not hold the
account gate, so logout is not delayed. Write admission is unchanged.

The deterministic reproduction failed before this correction and now passes all
32 concurrent readers while package mutation admission is occupied. Additional
tests reject damaged/disabled consent and another origin, and discard a snapshot
when the same account logs in again while its disk read is blocked. No network
requests are issued by the list tests. The panel shows the native read reason
and a local-read retry; its recovery test verifies that it calls no sync,
upload, fetch or conflict operation. Lists also reject a signed-out host with
saved credentials without initiating an implicit session restore.

Validation: 37 CloudAuth tests, 169 sync-engine tests, 21 native v2 tests
(including eight variables/read tests) and 64 frontend sync tests pass.
Frontend TypeScript/ESLint passes.
RC1 assets remain unchanged and still contain the read-contention bug.

The production backend responds `200` at `/api/v2/capabilities` and announces v2,
while the new `/api/v3/remote/people` route responds `404`. The existing remote
hosts route responds `401` without credentials. Nickname search therefore still
requires deployment of backend commit `6a69c99` and its additive migration.

### Typed local Variables foundation

The Variables editor now uses native type metadata and an atomic reviewed
replacement (`globals.typed.list/replace`). Text, number, boolean, list and
local-secret types persist while legacy textual rendering stays compatible.
No SQLite schema migration is required: the existing `app_state` rows retain
the text values, separate metadata stores only type tags and a revision nonce,
and one transaction commits the complete replacement. Public revisions are
bound to the exact native host lifetime as well as the stored state.

A dirty draft retains its original revision after a conflicting save. Losing
typed-method support does not convert a secret draft to legacy writes or
unmask its old value. The operator must discard/reload before changing hosts.
Legacy writes preserve an existing secret classification on the native host.

Native staging overrides a client's claimed nonsecret type when local metadata
says secret. Upload admission rechecks that policy, including previously queued
puts, and native edits cannot overtake an admitted emission. Changes/conflict
projections redact values for those names. Policy checks load only type tags;
SQLite is released before workspace preparation. A host without owned-edit
source integration fails closed. Previously published Cloud values are not
automatically erased by this local classification change.

Verification: 22 native Variables/read tests, three memory-only variable tests,
170 sync-engine tests, one owner/remote RPC test, 17 Cloud-enabled client parity
tests, one CLI typed-secret redaction test, 52 frontend tests and two browser
tests pass. The default CLI sandbox verifies 286 registered/typed methods and
23 read-only smoke calls. TypeScript/ESLint and Rust formatting pass. Five
previously missing Teams/sync typed wrappers and validators were added while
closing registry parity. The browser rendering was inspected with test data.

This is source work after RC2; published RC2 assets remain unchanged. Saving
variables remains local. The native review bridge described below is the next
increment; the Variables Changes adapter and explicit send/withdraw remain pending.

### Native Variables comparison and explicit cached application

The local-only `sync.v2.variables.review` returns account-bound comparisons between
typed native values and the cached workspace. It omits equal values, masks both
sides of local-secret/credential-like entries, and issues stable, bounded, expiring
references per comparison. It requires the same session/consent checks as Changes.

`sync.v2.variables.apply {review}` applies only the reviewed target, including a
cached deletion, through the atomic local Variables replacement. The reference is
one-time and invalidated by a new session (even the same account), changed consent,
local edit, host restart or a changed target cache entry. Pending records cannot
be adopted through this bridge. Saving still never uploads.

`sync.v2.variables.fetch` caches the account-wide feed with no publication or local
application. Variables-scoped `sync.v2.section` publishes already-staged Variables
only; it does not drain plugin intents or apply downloaded plugin state. Both check
the optional collection before dispatch. Other section queues remain intact.

Validation: 28 native Variables/read tests and 17 Cloud-enabled client parity tests
pass. Owner-only RPC checks cover review/fetch/apply. These are source changes after
RC2; a Variables section adapter and reviewed send/withdraw/conflict handling are
still required before exposing a sync control in that tab.

### Reviewed Variables actions and Changes panel (4.2.2)

The Variables tab now uses the shared Changes panel. Native comparison references
bind send/adopt/withdraw to the displayed local revision, cache entry, session and
consent. Send publishes that one saved native variable; withdrawal removes only
its Cloud copy. A second edit after acknowledgement advances the durable receipt
counter. Other pending Variables and section queues are not published by that action.

Queued changes may be cancelled before adoption or replaced by a reviewed send.
Uncertain/conflicted originals use the existing epoch barrier and fresh comparison;
a dedicated discard admission allows cancelling a now-secret put without permitting
its value to be emitted. A changed Cloud comparison requires review again. Local
adoption remains a separate atomic save and preserves unrelated local variables.

The panel now supports per-record action eligibility and confirmed adoption/
withdrawal. Secret/blocked values have metadata only and no send/adopt buttons.
A dirty editor blocks the Changes button. Old hosts without the full native
bridge keep the local editor and do not fall back to broad synchronization.

Validation so far: 32 native Variables/read tests and 170 sync-engine tests pass,
as do the plugin runtime suite and focused frontend/browser tests. Workspace
Clippy passes with Rust 1.99.1, all targets/features and `-D warnings`. Full CI and
release results are recorded with the 4.2.2 release evidence after completion.

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

## Completion checklist

- [x] **Variables sync.** Typed local editor, native secret policy, exact reviewed
  send/adopt/withdraw, queued/uncertain recovery and the shared Changes adapter.
  Per-record publication preserves unchecked Variables and other section queues.
  A dirty local draft blocks Changes until saved/discarded. Missing host methods
  hide the control; unsupported backend capability rejects network actions before
  staging. Text, number, boolean and list retain their types; secrets expose only
  metadata and explicit Cloud withdrawal. No variable uploads through local Save.
  Details: [Desktop Cloud v2](CLOUD-V2-DESKTOP.md#reviewed-variables-actions-422).
- [x] Three-way conflict view in the panel (per-field Base/Local/Cloud choice, all local / all Cloud, apply
  merge). Only adapters that declare `merge` show it; no current adapter can stage a merged value.
- [ ] Per-record history and restore (phase 2).
- [ ] Branch provenance and a public-template fork action (phase 2).
- [ ] Overlay revisions adapter (phase 3).
- [ ] Decide whether auto-sync is stored locally or in the `preferences` collection.
