# Sync and the Changes panel (plan)

Status: **planned**. Nothing below is implemented unless a line says so. It builds on
[Desktop Cloud v2](CLOUD-V2-DESKTOP.md), which already provides the sync collections, the stored
base version, conflict review (base, local, Cloud) and the background worker.

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
  behaviour per type. Until then the Variables tab shows the button only if the adapter exists
  (nothing is shown as synced that is not).
- [ ] Three-way conflict view in the panel built on two `diffJson` calls (base to local, base to
  Cloud), with per-field choice and "all local / all Cloud".
- [ ] Per-record history and restore (phase 2).
- [ ] Branch provenance and a public-template fork action (phase 2).
- [ ] Overlay revisions adapter (phase 3).
- [ ] Decide whether auto-sync is stored locally or in the `preferences` collection.
