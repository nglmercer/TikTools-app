# TikTools LIVE Performance

The reliability refactor against `79dd1ab` is **incomplete**. See
[progress and unresolved P0 defects](PERFORMANCE_REFACTOR_PROGRESS.md).
Historical measurements below describe earlier work; the dated section records
this branch's observations without claiming full LIVE performance or memory stability.

Related docs:

- `bun scripts/profile-live-memory.ts` — cross-platform RSS profiler
  (extended with the refactor's health columns; no Python required).

## Test environment

All numbers below were measured on:

- CPU: AMD BC-250 (2 threads visible in this container)
- RAM: 14 GiB (7 GiB available at measure time)
- OS: Linux (Ubuntu-based container)
- Build: `cargo test --profile test` (unoptimized + debuginfo), single
  container, no other load reported. Release builds will be faster;
  treat these as lower bounds, not promises.

Reproduce:

```sh
# Throughput harness (numbers print with --nocapture):
cargo test -p tiktools-core --features persistence,native-tiktok --locked \
  --test live_throughput -- --nocapture

# In-crate stress tests (admission accounting, saturated shutdown,
# unavailable storage, lifecycle churn):
cargo test -p tiktools-core --features persistence,native-tiktok --locked --lib

# Frontend batching tests:
bun test src/web/features/live-batch.test.ts

# Memory profiler (needs a running host PID; Linux and Windows):
bun scripts/profile-live-memory.ts --pid <PID> --samples 30 --interval 2
```

## Before / after: award throughput

Same machine, same sizes, same debug profile. Baseline is the `remake`
tip before the refactor (`85ea2c9`); the probe awarded N distinct-viewer
chats through `PointsService::default()` on both trees.

| Awards (distinct viewers) | Baseline | Refactored | Speedup |
| --- | --- | --- | --- |
| 500 chats | 2,300/s | 119,852/s | ~52× |
| 2,000 chats | 569/s | 174,544/s | ~307× |

Why the gap widens: the baseline sorted the whole viewer vector on
**every** award (O(N log N) per event, O(N² log N) total — the rate
falls 4× for a 4× population). The refactored service does one hash
lookup and no sort per award, so the rate is flat with population.

During the work an intermediate revision still scanned the cache once
per insert past capacity (20k awards in 6.0 s); batching eviction scans
(64 rows per scan) brought it to 0.15 s. The harness pins this with
latency percentiles, not just averages.

## Sustained workload rates (refactored)

From `cargo test ... --test live_throughput -- --nocapture`:

| Scenario | Events | Rate |
| --- | --- | --- |
| Chat-only (20k distinct) | 20,000 | ~137,000/s, p50 0.005 ms, p99 0.017 ms |
| Gift-heavy (500 viewers × 10) | 5,000 | ~205,000/s |
| 10k burst, one viewer | 10,000 | ~219,000/s |
| Mixed chat/gift/like/share | 10,000 | ~200,000/s |
| 10k distinct viewers | 10,000 | ~125,000/s |
| 100k distinct viewers | 100,000 | ~159,000/s |
| 8 concurrent streams | 40,000 | ~95,000/s |
| Persisted awards (SQLite writer) | 5,000 | ~1,550/s |

Correctness asserted alongside every rate: exact coin/chat/like/share
totals, exact per-viewer balances, top-K rank order with deterministic
tie-breaks, bounded hot cache, and full restart recovery (5,000-viewer
round-trip through SQLite, balances exact to the cent).

The persisted rate is SQLite-bound (one batched writer thread, ≤128
ops per transaction) and intentionally so: awarding never waits for it
in memory, and shutdown drains it.

## Runtime limits and overload policies

| Stage | Capacity | Overload behavior |
| --- | --- | --- |
| Hot viewer cache | 10,000 + 64 slack for clean rows; dirty rows can exceed this (open P0) | Dirty-cache admission and reliable committed progress still need correction |
| Points writer queue | 4,096 ops | Existing synchronous fallback can overtake queued writes; ordering/failure handling remains an open P0 |
| LIVE ingress queue | 2,048 ordered items | Lifecycle: backpressure. Gift/follow/share: 1 s backpressure then gap-reported drop. Chat/likes/stats: shed with counters |
| Plugin queues | 64 per plugin, 128-call admission | Shed with per-plugin counters; slow plugins isolated; gap protocol for reliable loss |
| Automation slots | 32 tasks | Rejected work counted; domain event already delivered |
| Processor slots | 64 total / 8 per plugin | Overload counted per processor (`processorStats.overloaded`) |
| UI feed | 300 visible, 900 staged | Oldest staged shed with visible counter; one reactive update per frame |
| WebView outbox | 512 lossy, 4,096 msgs / 16 MiB reliable | Coalesce snapshots; reliable overflow fails loudly, never silently |

## What was verified

- **Bounded memory under sustained overload**: 10k-event shed flood —
  accepted + dropped == sent, queue never exceeds capacity
  (`sustained_overload_sheds_with_exact_accounting`).
- **Saturated shutdown**: 64-deep queue drains and the worker joins with
  no retained payloads (`saturated_queue_drains_and_joins_on_shutdown`).
- **Reconnect replay**: identical `(type, msg_id)` replays award points
  exactly once end-to-end; gift streaks keep every increment while final
  replays are suppressed; missing IDs never suppress.
- **Unavailable SQLite**: poisoned database file — awards and reads keep
  serving from memory, errors are counted and logged, shutdown joins.
- **Rapid lifecycle churn**: 100 enable/disable cycles keep fan-out
  revisions consistent; repeated worker pruning releases queues.
- **Forced WebView teardown under load**: saturated outbox clears on
  teardown and recovers functional and bounded.
- **Cooldown cardinality**: 9,000 distinct cooldown keys stay under the
  8,192 bound with identical allow/deny decisions.

## Known limitations (not fixed, documented)

- 500–1,000 events/s end-to-end are stress targets, not production
  guarantees: upstream TikTok disconnects, broadcast replay limits, and
  disk failures remain explicitly reported, never absorbed.
- Under *sustained* overload past every bound, sheddable events (chat,
  likes, room stats) are dropped with counters — their loyalty points
  included. Critical events (gifts, follows, shares) get backpressure
  first and gap reports if ultimately shed. This is the documented
  trade: explicit local shedding beats unpredictable upstream loss.
- `system.health` reports host RSS on Linux only; plugin and WebView
  RSS are `null` there by design — use `bun scripts/profile-live-memory.ts`,
  which attributes them by OS process name. No per-process value is ever
  fabricated.
- Cooldown tracking fails open past 8,192 keys (clears rather than
  growing): one window of over-execution beats unbounded memory.
- UI numbers are starting policies (300 visible, 30–60 fps flush,
  500 ms leaderboard window), not measured optima.
- No real-LIVE (production TikTok connection) memory claim is made:
  real-LIVE stability requires real-LIVE evidence (§Definition of Done
  in the refactor brief). The harness proves bounded behavior under
  synthetic load on real application paths.


## 2026-10-13 reliability branch observations

The same microbenchmark command was run sequentially on the exact baseline and
candidate checkout with `--test-threads=1`, in the unoptimized test profile.
Environment: AMD BC-250, 12 logical CPUs, Linux 7.2.9-2-cachyos, Rust 1.99.0.
Other host load was not controlled, and each build has only one observation.

| PointsService workload | Baseline events/s | Candidate events/s |
| --- | ---: | ---: |
| 20,000 chats | 181,439 | 181,091 |
| 10,000-event burst | 323,724 | 312,258 |
| 100,000 distinct viewers | 191,550 | 189,864 |
| 5,000 persisted awards | 1,695 | 1,674 |

The chat harness RSS delta was 9,564,160 bytes on each checkout. These calls
exercise PointsService directly, whose implementation remains unchanged. They
provide no evidence of an optimized full LIVE pipeline or a leak fix. Tail
latency for chat awards was p99 0.0074 ms baseline and 0.0075 ms candidate.

The owned LIVE points execution service now has 128 queued jobs plus one executing
job. Executing synchronous awards are awaited; there is no two-second timeout
that abandons a running award. This is distinct from durable writing, whose
ordering, saturation fallback and failure/recovery defects remain unresolved.
The policies in the historical table do not establish those defects are safe.
End-to-end transport, slow SQLite, plugin load and real-LIVE scenarios remain required.
