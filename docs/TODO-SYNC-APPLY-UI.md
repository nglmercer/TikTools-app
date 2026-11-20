# TODO: apply synced plugin selections and notify the UI

Status: open. Found while testing two instances signed in to the same account.
This file is local and uncommitted; commit it only when we agree on the plan.

## What was verified

- Two instances (desktop and headless) are signed in to the same account
  (accountId `a79e4ae3-…`), and sync is enabled on both.
- A plugin disabled on the headless instance is pushed (`pushed: 1`) and pulled
  by the desktop (`pulled: 1`). The transport works.
- The desktop reports `applicationDeferred: true` and keeps the plugin enabled.
  The change is not applied, so the UI has nothing to show.

## Why it is deferred (code)

`crates/tiktools-core/src/control/cloud_sync/legacy.rs`, `sync_now`:
remote values stay in the durable account snapshot. Applying them "needs a
session-, consent- and source-fenced review and a recoverable local-runtime
outcome". `application_deferred` is `skipped != 0`. Nothing applies the remote
selections yet, so this is a missing feature, not a flaky test.

## What to build

1. **Apply pipeline** (core): turn each pulled plugin selection into a local
   change behind the review the comment describes (session, consent and source
   fences), with a recoverable outcome if a plugin cannot be changed right now.
   Keep the local-only mode unchanged.
2. **Notification to the UI** (desktop): after an apply, publish a domain event
   (for example `plugin-selections-changed`) on the existing domain-events path
   (`crates/tiktools-desktop/src/window/domain_events.rs`) so the webview refreshes
   the plugin list and settings.
3. **Headless instances**: an apply path or an explicit command, so a headless
   workspace can also converge. Decide whether the change must go through a
   WebSocket or stays on IPC; IPC already reaches the desktop, and a WebSocket is
   only needed if an external page must receive events.
4. **Component refresh** (web): the plugin list and the sync status subscribe to
   that event and reload; no full reload.
5. **Tests**: integration test that pulls a selection and asserts the plugin
   state changes on the receiving host, and a UI test that the event refreshes
   the list.

## Manual check (until tests exist)

1. Sign in to the desktop and to a headless workspace with the same account
   (`scripts/e2e/login-instance.ts`), and enable sync on both.
2. On the headless instance: `plugin disable audio.play.process`, then `api call sync.now`.
3. On the desktop: `api call sync.now`, then check the plugin list in the UI.
   Expected after the fix: the plugin shows as disabled and `applicationDeferred` is false.

## Phase 4 result (2026-11-20): blocked by a data-loss risk. Do not run library sync.

Observed on a clean account (two headless workspaces, A and B):
- Convergence works once: A pushes its library, B applies it, and a second A sync returns `nothing`.
- A created `p4-from-a` and synced: the push was accepted, but B did not show it.
- B then synced and reported `push-local`: B's content digest changed after it applied the
  remote snapshot, even with no user edit. B pushed its own library, which lacks `p4-from-a`.
  The remote no longer has the automation; A still has it locally.
- Running a sync on A now would apply the remote version and drop `p4-from-a` locally.

Root cause: the change detector uses a digest of `owner_library_snapshot`, and that digest is not
stable across an apply on another host (the content written by `owner_library_submit` does not
round-trip byte for byte). Fix options:
1. Compare canonical content (sorted keys, fields that the submit writes only), then test round-trip.
2. Track changes explicitly: a dirty counter in the automation storage, bumped on every write,
   so change detection does not depend on re-reading the snapshot.
Option 2 is the robust one and needs approval because it touches the automation storage writes.

Also fixed in this pass: a rejected remote snapshot no longer advances the sync base, and the
sync result carries `rejection` with the reason.

## Phase 4 status (verified on the final build)

Verified with two headless workspaces on one account (A and B):
- Steady state: both sides return `nothing`; no push loops.
- A pushes a new automation, B applies it (`apply-remote`, no conflicts), and both lists match.
- Agent conflict: B keeps its unsynced local automation (not discarded), A pushes its own,
  B pushes the merged library, A receives it, and both settle at `nothing`.

Root causes fixed in this pass:
- Change detection uses a stored write counter (triggers on the library tables), not a digest.
- Apply expectations (catalog revision and definition digest) come from storage, not the
  runtime-merged view. The importer creates only, so records already present locally are skipped.
- An owner-library submit without acknowledgement is a rejection, with the owner's reason.
- A rejected snapshot does not advance the base; pending local edits keep their pre-apply counter.

Resolved in the last pass (verified live, A and B, both directions):
- Events with no actions now sync. The importer accepts them and the template validator allows an
  entry with no actions only when its event references no actions. Events that reference actions
  carry those actions in their own entry; each action is created once.
- Limitation kept: an event whose referenced actions already exist locally, or that is shared
  between two events, is not imported (it is skipped, the rest of the snapshot still applies).

Not verified: the desktop `ui` origin (needs the app restart, not yet confirmed) and the UI-wins rule
with a real desktop. The UI-wins decision branch is covered by `decision_covers_every_branch`.

## Desktop UI-wins test (run after the desktop restart with origin `ui`)

Verified on the running desktop (origin `ui`, sign-in to the phase 4 account):
- The desktop decides `push-local-wins-conflict`, which only the `ui` origin does (an agent would
  get `apply-remote-wins-conflict`). Origin `ui` is active.
- Import capacity fixed: the importer no longer stops at 32 entries (it marked a partial import as
  applied and never retried). The profile import validator allowed 32 entries, which blocked any
  library with more events; it now uses the record budget, `OWNER_LIBRARY_MAX_RECORDS` (512).

Fixed in the per-record merge (this pass):
- Records are merged one by one. A change on one side never erases an unrelated change on the other.
  The base stores a digest per record, so each side is compared with what both held last time.
- Same-record conflicts: the UI version wins on the UI; the Cloud version wins on agents, and the
  discarded local edit is counted in `conflicts`.
- Removals propagate both ways. A malformed Cloud snapshot is refused, never read as empty.
- Import capacity: the old 32-entry cap is gone from the sync path. Sync no longer uses the
  owner-library profile import, so `owner_library.rs` is back at HEAD.

Verified live (headless agents C and D, the desktop not yet rebuilt):
- Disjoint changes: D renames a shared record, C creates a new one, both syncs in order. Both
  changes survive on both sides, and both sides reach `nothing` with identical lists.
- Same-record conflict between agents: C edits and pushes, D edits from a stale base. D takes the
  Cloud version (`apply-remote-wins-conflict`, 1 conflict), and both sides agree.

UI-wins verified live (desktop rebuilt and restarted, origin `ui`, agents C and D):
- C pushes a new record; D edits a shared record and pushes; the desktop edits the same record and
  syncs. Result: `push-local-wins-conflict` with 1 conflict, the UI version wins, C's record survives,
  D and C take the UI version on their next sync, and all three sides end `nothing` with identical lists.
- First sync of the desktop after the upgrade: its base came from the old code (no record digests), so
  every difference counted as a conflict and the UI version won. This is expected once per upgrade.

Known limitations:
- The catalog (profiles and templates) is not merged. A pass publishes the local catalog, so a remote
  catalog change can be overwritten. This needs its own merge.
- Local writes use the versioned record operations, not the owner-library journal. The sync no longer
  produces an owner-library receipt for each applied change.

## Catalog merge (profiles, templates, active profile)

Implemented with the same per-record scheme: `profile:<id>`, `template:<id>` and `active` are merge
keys with their own digests. The merged catalog is validated before any write, records are written,
then the catalog is replaced with a revision fence (`replace_automation_catalog`), so a concurrent
local catalog change is refused rather than overwritten.

Verified live on a fresh account, two agents: each imports a different template; both templates
reach both sides, the template lists match and both sides end `nothing`.

Bugs found and fixed while testing:
- The published snapshot put catalog keys into `definitions.events`. Any other device then refused
  the whole snapshot (`library record has no id`), and this device could not read its own record
  either. Only `action:` and `event:` keys go into `definitions` now.
- A missing Cloud record was refused forever. It now publishes the local state as a union, with the
  base treated as empty, so nothing local is deleted.

Not verified live: profile merge (covered by unit tests). Profile import/export from the CLI needs a
confirmation path that was not worked out in this pass.

Account note: the earlier test account `e2e-1795181771104-...` holds a malformed `automation-profiles/
library` record from the bug above. Its agents refuse it until that record is deleted with a native
session. The desktop is signed out of that account path and has sync disabled.

## Profile membership (three-way merge)

A profile changed on both sides used to be one whole-key conflict, so the UI won and the other side's
added members were lost while the template itself survived. Now:
- `name` and `description` are scalars. A side that did not change one takes the other's value; two
  different changes are a conflict (UI wins on the UI, the Cloud wins on agents).
- `eventIds` and `actionIds` are merged as sets against the base content, which the sync base now
  stores (`profiles`): each side's additions survive, and each side's removals apply.

Verified live (two agents, new account): C and D each import a template while the active profile is
`123`, D pushes, C merges. C's result is `apply-remote` with 0 conflicts. Both template events appear in
the snapshot on both sides, and both sides reach `nothing`.

Not yet run with the desktop: the desktop binary still has the pre-membership code. Its sync is turned
off on the test account until the desktop is rebuilt and restarted.

## Open items after this pass (superseded by the final verification below)

- Desktop with the membership merge: the rebuild and restart were denied (killing the running desktop
  was refused by the permission classifier). The desktop still runs the pre-membership binary with sync
  disabled. To run: close the desktop, then `TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:3000 bun scripts/task.ts start`.
- UI-wins on profile membership and on records with the new binary: not run (depends on the above).
- Profile scalar conflict live: not verified. `template profile-import` created templates and events, not a
  profile whose name could be checked from the CLI. Unit tests cover the name/description rule.
- Malformed record on the old test account `e2e-1795181771104-…`: not repaired. Deleting it needs a native
  session (token in the OS keychain). The new account `e2e-1795190268234-038729@example.test` is clean.

## Final verification (desktop restarted with the membership merge)

- Desktop restarted with the current binary, signed into the new test account, sync enabled.
- Profile membership, UI branch: the desktop and agent D each import a template into the active profile.
  The desktop merges both with 0 conflicts; both events are present on UI, C and D; all three settle.
- Records, UI wins: the desktop edits a record that an agent edited; `push-local-wins-conflict`, 1 conflict,
  the UI name wins on all three sides, and an agent-created record survives.
- Profile name, live: C renames profile `exported-profile` to "Shows C2" and pushes; D renames it to "Shows D"
  and syncs: `apply-remote-wins-conflict`, 1 conflict, and both sides end on "Shows C2".
- Malformed record on the old test account: repaired with `automation.library.sync.replace_unreadable --confirm`
  (guarded: refused while the record is readable). Afterwards the normal sync settles and a second agent syncs.

Note: the replace operation publishes this device's library over an unreadable Cloud record. Use it only when
the normal sync refuses with a malformed-record rejection.
