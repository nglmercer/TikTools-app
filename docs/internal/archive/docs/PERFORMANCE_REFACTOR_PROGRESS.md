# LIVE reliability refactor progress

Target: `remake`. Baseline and fetched target: `79dd1ab193cf7e8e251575d44e0a6270f7814010`.
Feature branch: `fix/live-reliability-refactor`. Work in progress; no acceptance criterion is assumed complete.

## Current implementation audit

The transport, core lifecycle/ingress/handlers, points writer/SQLite,
analytics writer, plugin manager/workers/process/VM, fan-out/supervision,
poll/push runtime, frontend connection/feed/control/controller, desktop
outbox/lifecycle, existing tests and performance documentation were inspected.
Existing bounded queues, replay guards and compatibility interfaces must remain.

| Phase | Confirmed findings / remaining work | Status |
| --- | --- | --- |
| 1 | Transport task ownership, original stamping and stale fencing implemented. Upstream reconnect visibility and signer OS-thread teardown remain. | Partial |
| 2 | Queue accounting and bounded owned points execution implemented. Payload-byte limits, expensive logging and complete pipeline stress validation remain. | Partial |
| 3 | Durable FIFO ordering implemented: single ordered queue, no synchronous fallback, separate durable/resolved watermarks, atomic per-op recovery, full-reset reload gating. Real-stress throughput numbers still pending. | Complete |
| 4 | Failed workers lose join ownership. Stop can join indefinitely. Startup handshake has no deadline. Process per-call I/O threads are detached. Replacement can race old stop's running flag. | Complete (see Phase 3 below) |
| 5 | Subscriber uniqueness, separate poll/push owners and gauge cleanup implemented. Poll generation/metrics and pushed processing isolation remain. | Partial |
| 6 | Main RPC bounds, feed ownership, scoped/aged dedup, debounce cancellation and stale response fences implemented. Full composable/WebView/performance audit remains. | Partial |
| 7 | Profiler thread/CPU/state labels and measured primitive comparisons added. Full host/child/scenario matrix remains unavailable. | Partial |
| 8 | Rust/frontend checks and native smoke passed; Playwright 3 passed/36 failed. Real-LIVE testing and remaining implementation verification are not complete. | Partial |

## Evidence and validation

Findings above are static code findings. No new benchmark results or real-LIVE
memory measurements have been collected. No real memory leak is claimed fixed.
Git working tree was clean before branch creation. No repository AGENTS.md was found.

## Completed batches

### Batch 1: transport ownership and ingress generation/accounting

- Added an owned connection task, cancellation token, monotonically increasing
  transport generations and an additive stamped stream; legacy `subscribe`
  and connect/disconnect interfaces remain available.
- Connect and pick supersede initialization; replacement joins the old task.
  Five-second teardown failure retains ownership and refuses replacements.
  Network initialization has a 60-second deadline; socket close has three seconds.
  Embedded signer construction is joined rather than abandoned on cancellation.
- Core consumes original generations, fences stale lifecycle and data items,
  and publishes lifecycle topics from the ordered worker rather than duplicating
  them in RPC handling. Connection status reads transport state.
- Admission reserves capacity and accounts before publication. Shutdown counts
  cancelled queued payloads explicitly. Removed the unsupported byte-bound claim.
- Validation: transport unit suite passed (4 tests); core feature-enabled library
  suite passed (288 passed, one opt-in RSS test ignored). Includes 100,000 concurrent
  admissions, full-queue cancellation and stale disconnect regression.
- Remaining phase-1 work: upstream reconnect-in-progress state is opaque to this
  adapter; `Reconnecting` is represented but cannot yet reflect each upstream
  retry. Additional real transport concurrency/termination validation is required.
- This is synthetic/static validation; no real-LIVE memory evidence was collected.

Later batches are in progress and not covered by this commit's completion record.


### Batch 2: fan-out and polling isolation

- Fixed exact/repeated/wildcard/match-all overlap by returning a subscriber set.
- Polling and pushed events have independently owned tasks, with bounded existing
  broadcast admission and explicit lag-gap reporting. Shutdown joins both tasks.
- Regression: a real fixture worker holds a poll open while a pushed event must
  reach the domain bus within 500 ms, then both owners are joined.
- Validation: 11 plugin-event tests passed; the slow-poll isolation regression passed.
- Not complete: startup can still block a Tokio caller; pushed-event processing
  itself can wait on automation; generation/queue metrics need further work.

Batch 1 commit: `4c3cb3e`. Later lifecycle crash handling is separately tested
(5 transport tests passed); no real-LIVE measurements have been collected.


### Batch 3: frontend admission and disposal

- Main control client admits at most 128 pending RPCs, rejects excess with
  `overloaded`, exposes count diagnostics, and rejects calls after detach.
- Feed replay maps belong to the composable instance. Message IDs expire after
  ten minutes; creator/room switching clears them; same-room reconnect keeps them.
- Feed disposal cancels owned frame callbacks, watchers, scroll listeners and
  debounces; debounces now expose a compatible callable `cancel` method.
- Connection RPC completions are fenced by operation/refresh revisions and
  disposal. Topic subscriptions are explicitly released.
- Controller uses the feed's coalesced scroll frame and coalesces gap refreshes;
  gaps during a refresh schedule one further pass rather than unbounded refreshes.
- Validation: 40 focused tests passed, full scoped suite 682 passed; frontend
  type-check, lint and production build passed. Full application mount/WebView
  integration, age-expiry and other composable teardown audits remain outstanding.

Batch 2 commit: `46ac837`.


### Batch 4: connection task failure guard

- A task exit guard marks unexpected completion/panic failed; a closed startup
  response channel also marks failure before returning, avoiding a status race.
- Disconnect domain notification rechecks authority after asynchronous cleanup.
- Validation: all five transport lifecycle tests passed; workspace Clippy passed
  with warnings denied after removing unit-valued send bindings.

Batch 3 commit: `f24e1ea` (updated in the next documentation batch).


### Batch 5: owned bounded LIVE points execution

- One `live-points` OS thread and a 128-job queue replace per-event
  `spawn_blocking` calls and the timeout that abandoned running awards.
- Ingress awaits the award outcome in order. Queue admission is cancellable;
  queued callers already dropped are skipped. Executing synchronous work is not
  forcibly cancelled and is not reported as cancelled by this service.
- Worker retains PointsService only, not AppCore. Explicit shutdown closes
  admission, joins the execution worker and joins the durable writer off Tokio.
- Validation: 128 worker creation/shutdown cycles apply 1,024 exact awards,
  return the PointsService owner count to one after each join, and reject later
  admission. Core feature-enabled library suite: 290 passed, one RSS test ignored.
- This fixes execution ownership, not the unresolved durable writer's sequence,
  synchronous fallback, failure/recovery or dirty-cache correctness defects.
- Extra lifecycle regression covers 32 simultaneous connection requests.
- Upstream EmbeddedSigner owns an internal OS thread without an exposed join
  handle; complete signer teardown verification remains a phase-1 limitation.

Batch 4 commit: `9f1102e`.


### Batch 6: plugin queue accounting

- Per-plugin admission now reserves capacity and records acceptance before
  publishing; consumers cannot take items ahead of their gauge increment.
- Cancelled workers close admission and account for queued payloads they release.
  Saturated-cancellation regression now verifies the queue gauge returns to zero.
- Validation: all 11 plugin queue/fan-out tests passed, including saturated cancellation accounting.

Batch 5 commit: `0653ffe`.


### Batch 7: measurement and verification record

- Profiler accepts build/workload labels and idle/load/cleanup phase labels;
  samples host CPU/threads and connection generation/state, treats missing RSS
  as unknown, and rejects mismatched health PID. Three Python regressions passed.
- Analytics-only Linux RSS test: 24 cycles, 98,304 records, 32 batches per cycle,
  zero pending after each join; RSS 19,784 KiB from cycle 1 through cycle 23.
- Frontend: 682 scoped unit tests passed; type-check, lint and build passed.
- Rust: format and workspace Clippy with warnings denied passed; workspace tests,
  core persistence/native tests, throughput tests, loader all-feature tests and
  analytics RSS opt-in test passed. Final rerun results are recorded below.
- Native WebView smoke test was enabled and passed on the configured display.
- Playwright: 3 passed, 36 failed. `navigation.spec.ts` boot failure from an
  unhandled `globals.list` was reproduced on exact `79dd1ab` in an isolated
  checkout. Other failures remain untriaged, not classified as pre-existing.
- No suitable authenticated real-LIVE session was used. Real-LIVE, full-host
  idle/peak/cleanup and child/thread comparisons have not been measured.
- Full refactor is not done: phase 3 durable points and phase 4 plugin lifecycle
  are still unimplemented, plus the partial phase limitations in the table.

Batch 6 commit: `9a656f0`.
Single draft PR: https://github.com/nglmercer/TikTools-app/pull/19


### Additional frontend regressions

Replay-age expiry and out-of-order concurrent status refreshes now have explicit
regressions. Final scoped frontend suite: **684 passed**, zero failed;
frontend type-check and lint passed. Earlier 682/40 counts are historical runs.

### Final Rust rerun on the implemented batches

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo test --workspace --locked` | Passed; opt-in analytics RSS test ignored here |
| `cargo test -p tiktools-core --features persistence,native-tiktok --locked` | 290 library tests + 8 throughput tests passed; one RSS test ignored here |
| `cargo test -p tiktools-tiktok --locked` | 6 lifecycle tests passed |
| Explicit opt-in analytics RSS test | Passed separately, 24 cycles |
| Plugin-loader `--all-features --locked` tests | Passed |
| Native WebView smoke with gate enabled | Passed on configured display |
| Main Playwright suite | **Failed: 36 failed, 3 passed** |
| Python profiler regressions | 3 passed |

The outstanding phase-3/phase-4 work is an implementation gap, not an environment
blocker. This draft must not be described as meeting the full definition of done.


Frontend regression commit: `77b6967`.
Native WebView smoke was rerun after the core execution-worker changes and passed.


## Phase 1 — loss-aware graceful ingress shutdown (prompt phase 1)

Root cause fixed: the pump and the ordered ingress worker shared
`plugin_state.observer_shutdown`, so `AppCore::shutdown` cancelled both at
once. The worker's biased select therefore released every already-admitted
event as `cancelled` instead of draining it, contradicting the documented
graceful-drain contract.

- Two separate scopes now exist: `AppCore::live_admission` stops *new*
  admission (the pump observes it and drops the queue sender), and
  `AppCore::live_ingress_stop` is an emergency stop used only after the
  drain deadline expires. The worker keeps draining until the sender closes.
- Explicit monotonic lifecycle: `running -> stoppingAdmission -> draining ->
  stopped`, published in ingress diagnostics. A stale writer can never move
  it backwards.
- `AppCore::stop_live_ingress` owns the bounded ladder: pump join
  (`TIKTOOLS_LIVE_PUMP_STOP_DEADLINE_MS`, 5 s) → ingress drain
  (`TIKTOOLS_LIVE_DRAIN_DEADLINE_MS`, 10 s) → emergency stop
  (`TIKTOOLS_LIVE_INGRESS_STOP_DEADLINE_MS`, 5 s) → task abort with explicit
  accounting. Every stage is bounded, so a wedged synchronous points award
  cannot deadlock shutdown.
- Admission accounts every event it receives: an event released because
  admission stopped is counted in `admission_cancelled`, never dropped
  silently, including cancellation while a lifecycle event waits on a queue
  reservation.
- Accounting: `stale` is now exclusive of `processed`; a new `in_flight`
  gauge carries items between take and completion; `abandoned` reports
  in-flight items cut off by an emergency abort (their outcome is unknown,
  never reported as a clean cancel). Settled invariant:
  `accepted = processed + stale + cancelled + abandoned`, and
  `outstanding()` is exposed in diagnostics.
- Shutdown now cancels the observer token *after* the LIVE drain, so drained
  events keep their automation side effects instead of being dropped by the
  cancellation they were admitted for.
- `start_live_event_pump` refuses to start after shutdown began or admission
  stopped, so no unjoinable pump can be registered.

Tests (all new/deterministic): graceful drain of 2,048 accepted events;
emergency stop accounting; empty-queue shutdown; cancellation during queue
reservation; admission stop racing an active producer; `stop_live_ingress`
drain + owner release; shutdown racing an active producer; 8 repeated
start/stop cycles with post-shutdown start refusal; and a wedged synchronous
points award (test-only stall gate in `PointsWorkService`) proving the ladder
aborts, accounts 3 queued + 1 in-flight events, and returns ownership to the
baseline.

Validation: `cargo fmt --all -- --check`, workspace Clippy with `-D warnings`,
workspace tests (647 passed, 0 failed), and core feature-enabled suite
(303 library + 8 throughput, 1 ignored RSS test) all pass. No real-LIVE
measurement was collected.

Phase 1 commit: `ed3e042`.

## Phase 2 — durable points FIFO ordering and SQLite correctness

Root cause fixed: sequence allocation and the queue push happened in
different critical sections, and a saturated queue fell back to a
*sequential inline write* on the awarding thread. That inline write could
land after ops with higher sequences and overwrite them with a stale row
snapshot (each op upserts a whole row), silently dropping queued
increments. Failed batches also advanced the committed sequence, and
shutdown stored `u64::MAX` regardless of outcome, hiding durable gaps.

- One ordered queue (`VecDeque` + `Condvar`) replaces the `mpsc` channel.
  Sequence allocation, the memory mutation and the push all happen inside
  the viewer lock, so queue order *is* sequence order by construction.
- No synchronous fallback. Admission waits (bounded, 5 s, per-service) for
  a free slot, then rejects **without mutating memory** — the caller, the
  hot cache and SQLite agree the op did not happen. Saturation is reported
  as `writerOverload`/`writeRejected`, never papered over with an
  out-of-order write.
- Two watermarks: `committedSeq` is contiguous and durable and stops at the
  first permanently failed op; `resolvedSeq` covers applied *or* explicitly
  failed ops and drives eviction and cold-reload waits. A failed write
  stays visible as a watermark gap instead of being erased by a sentinel.
- Writer: bounded whole-transaction retries with exponential backoff, then
  per-op **atomic** transactions (each op is its own transaction, so a
  retry can never half-apply an op and re-insert its award row). One bad
  row cannot discard its neighbours. A writer with no SQLite connection
  drains and reports every op as a permanent failure rather than
  accumulating unbounded work.
- Full resets no longer clear dirty markers early. A `reset_pending`
  watermark gates every cold reload until the reset op lands, so a reload
  cannot resurrect pre-reset rows and win the FIFO race. With no database
  configured, no markers are tracked at all (they would pin the cache).
- `PointsService::shutdown` closes admission, joins the writer and never
  claims durability it does not have. `AppCore::shutdown` bounds the join
  in a 30 s timeout with an explicit error policy.
- Diagnostics: queue depth, executing gauge, rejected/failed/applied counts,
  both watermarks, and commit latency (mean/max). `system.health` reports
  degraded when writes are rejected or permanently failed.

Tests (new): saturated-writer rejection with a durable order check after
recovery; full-reset reload gating; 100,000 ordered awards durable in
order; concurrent awards durable and exact across 8 threads; watermark-gap
assertions on a poisoned database. A test-only writer stall gate mirrors
the Phase 1 ingress gate.

Validation: `cargo fmt --all -- --check`, workspace Clippy with
`-D warnings`, workspace tests (all pass; 307 core library tests), and
`bun scripts/task.ts frontend-lint && bun scripts/task.ts frontend-test && bun scripts/task.ts frontend-build` all pass. No real-LIVE measurement was collected.

Phase 2 commit: `961eb26`.

## Phase 3 — plugin worker lifecycle and isolation

Root causes fixed:

- `remove_failed_worker` dropped the worker's `JoinHandle` on the hot path:
  the thread's outcome was never observed (join ownership lost).
- `finish_stop`, `stop_all` and `terminate_child` joined without deadlines,
  so a guest stuck in `onUnload` or native teardown blocked stop/shutdown
  forever.
- The napi-vm startup handshake (`ready_rx.recv()`) had no deadline: a guest
  wedged in `onLoad` froze every other plugin's start/stop behind the
  lifecycle lock.
- `set_running(id, false)` ran *after* the lifecycle lock was released, so a
  concurrent start of the same id could be mislabelled stopped, and two
  generations could coexist between the old instance's removal and its
  `Shutdown` send.
- `start()` only checked `contains_key(id)`: a worker that died without being
  observed was reported running forever and could never be restarted.
- The process runtime spawned a detached I/O thread per call and joined its
  stderr drainer without a bound; the points-work worker join in
  `AppCore::shutdown` was also unbounded.

What changed (`crates/tiktools-plugin-loader`, `tiktools-core`):

- Bounded joins everywhere: `join_worker` polls `is_finished` against a
  per-manager stop deadline (default 5 s), then abandons the thread with an
  explicit error and an `abandoned_workers` counter — the same "outcome
  unknown" ladder as the LIVE ingress. `PluginManager::Drop` and `stop_all`
  reap retained handles as a last chance.
- Join ownership is retained: a hot-path failure retires the generation into
  a bounded `retired` list (cap 32, oldest abandoned loudly on overflow);
  `reap_retired` joins only already-finished threads, so it never blocks.
- `RunningInstance::worker_finished()` plus a `start()` check let a
  dead-but-mapped generation be retired (ownership kept) and replaced by a
  fresh one instead of being reported running forever.
- Stop and shutdown now close admission, send `Shutdown` and flip the
  running flag *inside* the lifecycle lock hold (`retire_generation_locked`
  shared by `start`, `stop`, `stop_for_shutdown`, `remove_failed_worker`),
  so two generations can never coexist for one id and an old stop cannot
  clear a newer start's flag.
- Per-manager `PluginDeadlines { stop, start }` (not process-wide env/OnceLock
  globals, so tests stay isolated). The napi-vm startup handshake uses
  `ctx.start_deadline()`; a timeout sends `Shutdown`, detaches the thread
  with an explicit report, and fails the start typed.
- Process runtime: the per-call I/O thread is stored (`io_thread`), reaped at
  the start of the next call and after success/timeout/disconnect with a
  bounded join (3 s, explicit abandonment); the stderr drainer join is
  bounded the same way; a `Builder::spawn` failure kills the child and
  surfaces a typed error.
- Points-work worker join bounded (30 s, matching the durable writer's
  bound); an abandoned join is counted, reported, and degrades
  `system.health`.

Tests (new):

- `stop_abandons_a_wedged_worker_after_the_deadline_instead_of_hanging`
  (wedged-shutdown runtime, injected 100 ms stop deadline).
- `start_retires_a_dead_but_mapped_worker_and_spawns_a_fresh_generation`
  (owned worker that exits immediately; start must replace it).
- `panic_retirement_keeps_join_ownership_until_the_next_reap`.
- `napi_vm_startup_handshake_is_bounded_by_the_manager_deadline` (new
  `napi-vm-hang` fixture whose `onLoad` blocks; injected 200 ms start
  deadline).
- Process: bounded helper-join success/abandonment, and a timed-out call
  that kills the child and reaps its I/O thread.
- Points work: a wedged worker is abandoned at the join deadline and
  counted.

Validation: `cargo fmt --all -- --check`, workspace Clippy with
`-D warnings`, `cargo test --workspace --locked` (all pass; 308 core
library tests, 62 plugin-loader library tests), `cargo test -p
tiktools-core --features persistence,native-tiktok --locked`,
`cargo test -p tiktools-tiktok --locked`, `bun scripts/task.ts frontend-lint && bun scripts/task.ts frontend-test && bun scripts/task.ts frontend-build`,
`bun scripts/task.ts contracts --check`, `bun scripts/check-tiktok-signer-revision.ts` all pass. No real-LIVE
measurement was collected.

Phase 3 commit: `pending` (this batch).

### Reviewable commit sequence

- `4c3cb3e`: connection attempt ownership, transport generation and ingress accounting.
- `46ac837`: unique subscriber matching and independent poll/push owners.
- `f24e1ea`: RPC admission and frontend ownership/disposal.
- `9f1102e`: unexpected task termination status guard.
- `0653ffe`: owned bounded LIVE points execution and off-Tokio writer joining.
- `9a656f0`: per-plugin reserve-before-publication and cancellation accounting.
- `77b6967`: replay age and concurrent refresh regressions plus verification tracking.
- `da366ba`: sanitized benchmark record and Python profiler changes.
- `ed3e042`: loss-aware graceful LIVE ingress shutdown (phase 1 of this refactor).
- `961eb26`: durable points FIFO ordering and SQLite correctness (phase 2).
- This batch: plugin worker lifecycle and isolation (phase 3).


### October 2026 desktop transport follow-up

Shared pre-Winit admission now enforces count/byte budgets, main inbound RPC has
128 permits, and prepared outbound batches carry generation/sequence acknowledgments.
Native plugin output is bounded and batched; Linux terminal teardown disconnects
the private IPC signal without changing native destroy ordering or the external
`mem::forget` workaround. Frontend feature and iframe owners dispose owned work.
