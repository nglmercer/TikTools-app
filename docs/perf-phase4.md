# Phase 4 profiling notes

Profile-guided performance pass. Method: measure first with the repo's
existing harnesses plus throwaway `/tmp` probes (never committed); change
code only where profiling proves pressure. Conclusion: **no code change
justified** — every candidate is far from pressure at realistic load.

## Environment

- Linux x86_64, 12 CPUs; bun 1.4.0; cargo 1.97.1 (via
  `node scripts/cargo-with-linker.mjs`); branch `remake`.
- Toolchain note: measurements were captured with bun 1.4.0 while the
  repo pins bun 1.4.1 (`packageManager`). Numbers were NOT re-captured
  and are left unchanged; a patch-version bump does not affect the
  no-pressure conclusions below.
- No `crates/*/benches` harness exists, so Rust numbers come from the
  existing unit-test suites (real code paths) plus engine-level proxies
  (`bun:sqlite` for SQLite statements, `cat` over pipes for frame RTT);
  proxies are labeled as such below.

## Baselines and decisions

| # | Candidate | Measurement | Decision |
|---|-----------|-------------|----------|
| 1 | Automation throughput + semaphore drops | 21 `plugin_processors` tests in 0.31 s; global slots = 32, max 4 concurrent processors per event; overload fails open with `Overloaded` (no unbounded queue). Per-event host overhead is ≤4 `event.clone()` + one JSON size check (µs) vs ms-scale plugin calls. | No change: bounded and fail-open; throughput is set by plugin latency, not host overhead. |
| 2 | Plugin process cold start | Host-side spawn floor 0.54 ms (`/bin/true` + piped stdio, n=50). `load()` performs no handshake — cold start beyond spawn is the plugin's own init, outside host control. | No change: nothing host-side to remove. |
| 3 | Steady-state plugin call latency | Pipe frame RTT floor 0.025 ms for a 260 B frame (n=200); JSON serde ~1 µs (see 4). One `request.clone()` per call is required to move into `spawn_blocking`. Process-plugin I/O workers are explicitly out of scope. | No change: transport is ~0.1% of any real call. |
| 4 | JSON volume across plugin/control boundaries | Typical event 213 B; caps 256 KiB/event, 16 MiB/frame. 20k stringify+parse round-trips in 16 ms (0.8 µs/op). Control API does one `to_vec` per local-IPC request. | No change: no clone/serde pressure. |
| 5 | SQLite analytics write/query latency | `record_analytics_event` equivalent (fresh conn + txn + 3 upserts + commit): 0.265 ms/op, ~3.7k events/s sustained (n=200, `bun:sqlite` proxy, same schema/statements). Batched variant 0.008 ms/event; daily-summary SELECT 0.011 ms. Real `rusqlite` path: 9 analytics tests in 0.03 s. | No change: realistic live rates (tens/s peak) are 100x below the write ceiling; batching would add latency/complexity for no need. |
| 6 | Autocomplete filtering on large option lists | `filterSuggestions`: n=120 → 0.066 ms; n=500 → 0.49 ms; n=2000 → 1.8 ms; synthetic n=20000 → 16.4 ms. Real lists are capped (object sources 60–120 items; selects hold dozens). A behavior-identical `lastIndexOf` variant (output verified MATCH on 4 queries) reaches 8.9 ms at n=20000 but saves only 0.013 ms at realistic n=120. | No change: <0.1 ms at every realistic size; the relative win at synthetic scale is not pressure. |
| 7 | TTS event throughput | 50k deduper `claim` calls in 334 ms (6.7 µs/claim, worst case with a full 500-entry eviction scan); eligibility is O(1) predicates per event. | No change: realistic rate is a few events/s. |
| 8 | Frontend bundle chunks | `build:web`: one 533.95 KiB JS chunk (140.91 KiB gzip) + 131.78 KiB CSS (22.29 KiB gzip); 285 modules, built in 1.92 s. The desktop WebView loads the bundle locally. | No change: code-splitting a locally-loaded 141 KiB-gzip bundle buys no measured user-visible gain. |

## Reproduce

- Frontend probes: `bun /tmp/perf4-front.mjs`, `bun /tmp/perf4-ac2.mjs`
  (throwaway; import repo `src/` directly).
- SQLite probe: `bun /tmp/perf4-sqlite.mjs` (`bun:sqlite`, same DDL/DML).
- Rust suites: `node scripts/cargo-with-linker.mjs test -p tiktools-core
  --locked plugin_processors` and `... --features persistence db::`.
- Bundle: `bun run build:web`.
