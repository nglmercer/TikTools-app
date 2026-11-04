# Cloud Sync (Desktop multi-device plugin selections)

PR7 (`feat/desktop-cloud-sync`, stacked on PR6) adds opt-in sync of the
`plugin-selections` collection through RemakeBackend. It reuses the pinned
`cloud-client` sync engine with no new protocol.

## Components

| Area | Location |
| --- | --- |
| Sync engine (vendored, reused) | `crates/cloud-client` (`SyncState`, `Session::{pull_page,push,rebuild}`, `StateStore`) |
| Coordinator | `crates/tiktools-cloud-sync` (`SyncCoordinator`, `SyncStatus`, `SyncOutcome`) |
| Auth session sharing | `tiktools-cloud-auth` `session()` / `try_session()` / `last_identity()` |
| Core wrapper + hooks | `crates/tiktools-core/src/cloud_sync.rs`, `control/cloud_sync.rs`, hooks in `control/plugins.rs` |
| Control API | `crates/tiktools-control-api/src/modules/sync.rs` (`sync.*`, feature `cloud-auth`) |
| Frontend | `src/web/entities/cloud-sync/`, `src/web/features/cloud-sync/ui/CloudSyncEditor.vue`, Settings wiring |

## Flow

1. The user opts in (`sync.enable` / Settings toggle); the flag lives in
   `<data>/cloud-sync/cloud-sync.json`. Disabled means inert: staging is a
   no-op and `sync.now` is rejected.
2. Local install/enable/disable/uninstall calls stage a `plugin-selections`
   record (`{pluginId, version, enabled}`) or tombstone. Staging is local
   only and offline-safe; a missed lock or a missing identity is a no-op,
   never a failed plugin operation.
3. `sync.now` pulls cursor pages (auto-rebuild from a snapshot on 410),
   pushes staged edits (409 marks a conflict, never overwrites), persists
   the generation-checked state, and returns remote records.
4. The core applies each remote record **only to already-installed plugins**
   (enabled flag; tombstone disables). Unknown plugins are skipped, versions
   never trigger installs, and applying never re-stages — that is what
   prevents sync loops.
5. Conflicts resolve explicitly: `sync.resolve {key, keepLocal:true}`
   rebases for the next upload; `keepLocal:false` drops the edit and applies
   the remote value immediately. `SyncStatus.conflicts` lists them.

## State isolation

- `StateStore` files live per device under `<data>/cloud-sync/` and are
  bound to `(origin, account_id)`; a mismatch is replaced, never merged.
- Logout keeps the file (same-account re-login resumes); switching accounts
  resets to a fresh state without touching the old account's server records.

## Validation (2026-11-04, local)

- `cargo test -p tiktools-cloud-sync`: **11 passed** (two-device, offline
  staging + resume, both conflict directions, cursor expiry, tombstones,
  idempotent pulls, server failure + retry, account switch, no-secret
  surface, opt-out) against a mock speaking the real wire contract.
- `cargo test -p tiktools-core --features cloud-auth`: **3 sync tests**
  (opt-in lifecycle, fail-closed without sign-in, remote never installs or
  stages); default-feature suite unaffected.
- `cargo fmt`, workspace clippy `--all-targets --all-features -D warnings`
  clean; frontend `vitest` 102 files/555 tests, `eslint`, `vue-tsc` green.
- Full two-device E2E with real binaries against a loopback mock Cloud
  plus the real OS keychain (device dirs isolated via `TIKTOOLS_HOME`,
  shared `repo.game` declarative fixture): A login → enable → stage →
  push → B login → pull → remote apply toggles B's installed plugin →
  B disable → push → A pull applies → both directions settle with
  `pushed:0` on re-sync (no loops) → logout wipes the keychain.
  ALL-E2E-PASS.
- Not validated: real RemakeBackend sync endpoints, Windows/macOS
  keychains, concurrent multi-process state access.
