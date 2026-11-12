# Sync and the Changes panel (plan)

Status: **phase 1 frontend implemented in part** (see "Implemented" below). Everything else is planned. It builds on
[Desktop Cloud v2](CLOUD-V2-DESKTOP.md), which already provides the sync collections, the stored
base version, conflict review (base, local, Cloud) and the background worker.

## Implemented

- `SyncPanel` (modal, Changes and History views), `SyncButton` (icon, count, disabled reason) and the
  `SyncAdapter` contract in `src/web/entities/cloud-sync/api/sync-adapter.ts`. The panel renders only the
  controls an adapter declares in `capabilities`.
- Read-only RPC `sync.v2.changes.list`: a local read of the v2 workspace (cached Cloud value, pending edit
  with its frozen base). No network, no fences, no writes.
  **Plugin settings values never leave the host.** Settings are arbitrary plugin data and may be sensitive, so
  the reply values are projected only for the credential-free `plugin-selections` and `preferences`
  collections. Every other collection, and always `plugin-settings`, is reduced to metadata: plugin id,
  collection, state and revision of the cached and base records, the pending status, `hasNewer`, and for
  settings a sorted `changedKeys` list of top-level setting NAMES that differ from the base. The cached Cloud
  value and the pending edit value of a settings record are `null` in the reply (asserted on the serialized
  JSON in the `tiktools-core` `change_entry_tests`, the typed-client parity test and the adapter tests).
- Plugins adapter (selections and staged settings): whole-pass sync (`sync.v2.now`) and keep-local or
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

- [ ] **Variables sync.** Planned, deliberately not built. Runtime globals are local to the host and
  no Cloud collection carries them (`preferences` only holds the `desktop` record). Needs a backend
  decision first: a new `variables` collection in `cloud-contracts` and RemakeBackend, or a section
  of the preferences record. Open points: typed values (text, number, boolean, list, secret);
  **secrets must never sync in clear text** (exclude them or store only a reference); conflict
  behaviour per type. Until then the Variables tab shows no sync button (nothing is shown as synced that is not); the
  adapter would be registered next to the others in `AutomationsPage.vue`.
- [x] Three-way conflict view in the panel (per-field Base/Local/Cloud choice, all local / all Cloud, apply
  merge). Only adapters that declare `merge` show it; no current adapter can stage a merged value.
- [ ] Per-record history and restore (phase 2).
- [ ] Branch provenance and a public-template fork action (phase 2).
- [ ] Overlay revisions adapter (phase 3).
- [ ] Decide whether auto-sync is stored locally or in the `preferences` collection.
