# Cloud Sync (Desktop multi-device plugin selections)

This feature syncs plugin selections, not collaborative LIVE state. For the
two-PC moderator/agent implementation plan, see
[Collaborative LIVE management](LIVE-COLLABORATION.md) and its
[implementation specification](LIVE-COLLABORATION-SPEC.md).

PR7 (`feat/desktop-cloud-sync`, stacked on PR6) adds opt-in sync of the
`plugin-selections` collection through RemakeBackend. It reuses the pinned
`cloud-client` sync engine with no new protocol.

## Backend v2 handoff reviewed on 2026-11-08

The complete Desktop implementation and acceptance requirements are in
[Desktop Cloud v2](CLOUD-V2-DESKTOP.md). Its contract/coordinator foundation is
implemented and locally tested. Native transport, durable state/store and
WorkspaceCoordinator support all five collections, but the application still
uses v1. Explicit cutover, product RPC/UI, reviewed local application and actual
two-Desktop v2 acceptance are pending. The components, flow and historical
validation sections below describe v1.

The reviewed sibling RemakeBackend commit is `cb37900`, including canonical
registry shards and bounded v2 capability discovery in the SDK, following bounded
catalog pagination in `9f7e642`, catalog compatibility in `995d496`, integrity
and recovery changes in `c0fb951`, and the v2 implementation in `0c552b3`.
Its current contract is `docs/CLOUD-API-V2.md`; `docs/CLOUD-REDESIGN.md` and the
latest section of `docs/IMPLEMENTATION-STATUS.md` supersede older roadmap gaps.
That commit includes the digest-repair, recovery, byte-ordered pagination and
browser/CI changes covered by this checkpoint.

The backend now implements a generation-scoped plugin catalog, a revisioned
private/public template/profile library with independent moderation, and HTTP
workspace sync for `plugin-selections`, `rule-templates`, `profiles`,
`plugin-settings` and `preferences`. V2 has separate tables/cursors, conditional
ETags, tombstones, account quotas and a bounded change feed with snapshot rebuild.
It serves account configuration; LIVE management continues through the host's
separate scoped collaboration transport.

The implemented foundation preserves the following boundaries:

- Backend-owned portable contracts and v2 transport were added without replacing
  Desktop authentication or v1 state. `WorkspaceState`, `WorkspaceStore` and
  `WorkspaceCoordinator` are separate APIs; `SyncState`, `StateStore` and the
  existing product `sync.*` integration retain their v1 meaning.
- V2 files are private, generation-checked and partitioned by origin/account.
  Frozen base bodies, unconfirmed writes, conflicts and newer local generations
  survive restart. Writes become durably unconfirmed before HTTP; a matching
  current value is not treated as proof of an unknown original result.
- Read-only conflict review is bound to its coordinator, identity and local
  generation. Resolution requires another fresh read. A 410 persists an explicit
  rebuild requirement; validated cache rebuilds retain pending edits and reject
  rewinds without applying remote source locally.
- Release builds ignore runtime origin overrides; operators choose the origin
  at build time and debug builds retain the development override. Independent
  package trust roots and the remaining product selector restrictions still
  apply.

The next product integration must:

- Expose opt-in and explicit reviewed cutover, stage existing selections once,
  preserve v1 files, and run exactly one protocol for each identity. Never copy
  v1 revisions/cursors into v2 or silently activate both coordinators.
- Wire catalog/library views and apply reviewed imports through the native atomic
  [Owner library transition](LIVE-COLLABORATION-AUTOMATION.md#atomic-owner-library-transitions).
  Resolve local `secretRef` values through a defined local policy; preserve
  unsupported source without silently renaming, dropping or activating it.
- Check cloud portability explicitly: backend documents have a 256 KiB ceiling,
  depth 24, 8,192 nodes, bounded strings and ASCII document IDs. The broader
  local library limits do not imply cloud portability.
- Complete the thirteen acceptance scenarios in [Desktop Cloud v2](CLOUD-V2-DESKTOP.md#required-acceptance-evidence),
  including actual Desktop offline/lost-response/account-switch/conflict/rebuild
  behavior and the supported local application paths for all five collections.

### Current local evidence

- Foundation package suites: **86 passed** (48 client, 5 contracts, 14 auth,
  19 coordinator), covering existing regressions plus state/storage and loopback
  HTTP cases. Four release-origin policy tests are included in auth.
- Core regression: **755 unit + 8 integration tests passed**; control API,
  collaboration and client suites: **302 passed**. Clippy passed across the
  selected 11 Rust packages. The feature-disabled core check passed, with
  existing unused-code warnings. Rust 1.95 checks passed for the four foundation
  packages with all targets/features; the SDK Rust 1.89 check also passed.
  The normal Desktop/CLI build passed with the build-time origin override
  unset and CLI remote-control/local-web features enabled.
- The current frontend production bundle built successfully. The packaged Owner
  library browser regression passed against rebuilt current assets (**95.87 s**),
  covering all eight operations in EN/ES, sixteen native receipts and lost-reply
  recovery. This remains local Owner behavior, separate from v2 product acceptance.
- The maintained backend pre-v2 upgrade/digest-repair/actual SQL dump-and-restore
  drill passed on native PostgreSQL (**17.31 s**) and pgrust (**18.61 s**), with
  two pure URL-guard tests. Six v2 database cases, including ASCII punctuation
  pagination, passed on both SQL runtimes; the earlier 67-case database suite
  passed on native PostgreSQL. Backend Rust 1.95 workspace and Rust 1.89 SDK
  compiler checks passed.
- Backend authenticated rejected-Origin browser coverage passed locally on both
  SQL runtimes (two browser cases). Admin build/Playwright coverage is now in CI
  configuration; **GitHub Actions itself was not run**.

These checks used disposable local fixtures, not the configured application or
production database. The restore drill covers SQL data only, excluding object
volumes, credentials and live cutover. No deployed migration, production trust
or completed two-Desktop v2 product workflow is claimed.

## Existing v1 components

| Area | Location |
| --- | --- |
| Sync engine (vendored, reused) | `crates/cloud-client` (`SyncState`, `Session::{pull_page,push,rebuild}`, `StateStore`) |
| Coordinator | `crates/tiktools-cloud-sync` (`SyncCoordinator`, `SyncStatus`, `SyncOutcome`) |
| Auth session sharing | `tiktools-cloud-auth` `session()` / `try_session()` / `last_identity()` |
| Core wrapper + hooks | `crates/tiktools-core/src/cloud_sync.rs`, `control/cloud_sync.rs`, hooks in `control/plugins.rs` |
| Control API | `crates/tiktools-control-api/src/modules/sync.rs` (`sync.*`, feature `cloud-auth`) |
| Frontend | `src/web/entities/cloud-sync/`, `src/web/features/cloud-sync/ui/CloudSyncEditor.vue`, Settings wiring |

## Existing v1 flow

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
   Ordinary feeds longer than the 200-page run limit checkpoint the pulled
   pages and require a retry before any pending upload. A cursor-expiry rebuild
   is currently all-or-nothing: more than 200 snapshot pages or 200 catch-up
   pages returns `TooManyPages` without updating the old state. Repeating it
   cannot advance an oversized rebuild; restartable snapshot recovery remains
   required before claiming support for that volume.
4. Downloaded remote records remain in the durable account snapshot. The
   product does not automatically apply their enabled state or tombstones to
   local plugins. `sync.now` reports `applied: 0`, a `skipped` record count and
   `applicationDeferred`; these count retained remote records, not differences.
   Settings explains that reviewed local application is not yet available.
5. `SyncStatus.conflicts` lists conflicting keys. Until the durable reviewed
   recovery path is wired, `sync.resolve` rejects either decision without
   discarding or rebasing either side; the corresponding Settings actions are
   disabled. Local plugin operations remain available.

## Existing v1 state isolation

- `StateStore` files live per device under `<data>/cloud-sync/` and are
  bound to `(origin, account_id)`. The in-progress fence preserves separate
  partitions instead of replacing one account's state when another signs in.
- Logout keeps the same account's local state. Switching accounts never merges
  their pending work; the product integration and cross-process guarantees of
  the new partitioning still require validation before release.

## Historical v1 validation (2026-11-04, local)

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
  ALL-E2E-PASS. This historical result predates the suspension of automatic
  remote application described above and does not establish current acceptance.
- Not validated: real RemakeBackend sync endpoints, Windows/macOS
  keychains, concurrent multi-process state access.
