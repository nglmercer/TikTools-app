# Main WebView presentation contract and validation

The main WebView has a version-1 `ui.projection` capability handled by the desktop
IPC router. It is deliberately absent from the shared CLI/plugin ControlApi.
The request declares `feed`, `points`, `behavior`, `visible` and a strictly
increasing page revision. Native page generations fence retired-page calls and
outbound batches; frontend revisions reject delayed presentation messages.
Revisions increase across full Vue remounts in the same document. Window
visibility is delivered explicitly because native hiding need not update
`document.visibilityState` in WebKitGTK.

Main-page presentation:

- Normalized `live.event` is filtered before JSON construction. `live.ui-event`
  is forwarded only while Feed is visible. Every observed feed event is also
  retained in a bounded native journal (512 render events, evict-oldest)
  keyed by its monotonic bus cursor, before the visibility filter. On return
  from a hidden period the page requests `ui.feed.backfill` (version 1,
  generation-fenced, no revision advance) with its last presented cursor and
  replays retained events read-only; it inserts a discontinuity marker only
  when the replay reports an unrecoverable gap or is truncated, or when no
  host history is available. Backfill performs no TTS/points/automation side
  effects. The forwarded `live.ui-event` carries its `cursor` for bookkeeping.
- Individual `points.changed` stays reliable on the domain/control bus. The
  main page receives immutable authoritative top-100 `ui.points.snapshot`
  projections at most every 250 ms while Feed or Points is visible. A dirty
  bit bounds projection memory independently of distinct viewer count. It
  includes viewers entering the top K; absolute totals are never summed.
- `automation.run.completed` always remains on the shared bus. The main page
  receives coalescable `ui.automation.runs` snapshots (at most 60 runs, 250 ms)
  only while Behavior is visible. Open/return and reliable gaps request full
  authoritative state. No action execution depends on presentation delivery.
- Compatibility histories are constructed only for an interested legacy
  emitter, default raw domain subscribers or eligible plugin subscriptions.
  Existing `subscribe_domain()` requests the full old history contract;
  `subscribe_projection()` opts out. All existing emitters retain their default
  behavior unless explicitly opting out. Snapshot RPCs remain available.
- `ui.projection.stats` separates suspended feed events, lossy domain skips,
  exact feed-source skips, attempted feed deliveries and projection counts.
  It also reports the retained journal cursors (`feedOldestCursor`,
  `feedLatestCursor`, `feedJournalDepth`). A main-page delivery sequence counts
  transport shedding separately from source-bus loss. Coalesced snapshots are
  not reliable award notifications.

The shared bus stores immutable Arc payloads. Its old owned receive API retains
its wire/ownership behavior; `recv_shared()` avoids deep cloning events that a
projection or plugin observer will discard. Plugin eligibility is checked with
its existing subscription index before constructing an unused JSON envelope.

## Renderer filter A/B

On the same Linux native release test executable and identical compiled Vue
assets, disabling only `.tt-badge-level` backdrop blur changed renderer RSS near
14 seconds from **3,982,036 KiB to 488,488 KiB**. Disabling avatars/gift images
alone did not remove the growth.
The per-row backdrop blur has been removed; badge background, border, text and
all 300 buffered rows remain. These short synthetic measurements identify an
expensive renderer path; they do not prove a retained native-memory leak or
long-duration real-LIVE stability. They did not justify decoded-image caching. A separate DOM-count A/B justified limiting pinned rendering.

## Pinned rendering and paused scrolling

A separate same-artifact 30-second native DOM-count comparison reduced rendered
nodes from approximately 5,027 to 1,807 and the median sampled rAF interval p95
from 155 ms to 102 ms when painting 100 rather than 300 rows. The pinned feed
therefore renders the newest 100 cards while retaining its existing 300-event
buffer. Pausing, searching or selecting a filter exposes the full buffer, with
an explicit footer hint. No virtualization dependency is introduced.
Programmatic autoscroll is immediate: smooth scrolling repeatedly chased an
advancing bottom and continued after pause in the actual browser regression.
Paused scroll offsets survive Feed navigation; restoration also has a bounded
fallback for stopped native frame clocks.

## Frame scheduling

An actual Wry/Vue synthetic probe exposed stopped `requestAnimationFrame`
callbacks while native/page visibility still indicated visible. The feed's
existing rAF batching now has one owned 100 ms trailing timeout; the first
scheduler to run cancels the other. Scroll scheduling uses the same fallback.
Hiding Feed and disposing the root cancel both schedulers and discard/count
staging. Visible/staged limits remain 300/900. This addresses the observed
presentation stall; it is not evidence of a retained-memory leak.

## Reproducible native workload

Build a diagnostic Vue bundle and an opt-in native test executable:

```bash
VITE_TIKTOOLS_DIAGNOSTICS=1 bun scripts/task.ts frontend-build
node scripts/cargo-with-linker.mjs test -p tiktools-desktop --release \
  --features live-benchmark --locked native_live_matrix --no-run
bun scripts/run-live-webview-benchmark.ts \
  --binary target/release/deps/tiktools_desktop-EXACT_TEST_BINARY \
  --case C --output /tmp/tiktools-case-C \
  --warmup 30 --load 600 --cleanup 120 --rate 1000
```

Use the executable path emitted by Cargo (with `.exe` on Windows). The runner
creates isolated temporary databases, records executable/asset SHA-256 hashes,
the exact HEAD and dirty patch/source additions, host/UI NDJSON and process-tree
RSS/PSS/USS where supported. PSS/USS can be unavailable; RSS includes shared
pages. Windows process sampling uses the built-in CIM cmdlets. Run cases sequentially on an
otherwise idle machine. Diagnostic assets must remain unchanged during a run.
Normal release assets omit diagnostic/root-remount hooks; normal native builds
exclude the synthetic producer and test-only telemetry path.

Cases A–H and J are implemented: headless, native hidden, Feed, Settings,
Feed images disabled, points projection disabled, automation disabled, 10k burst,
and full-root remounts / main-page reloads (`--lifecycle reload`). The producer enters the real bounded ingress worker,
points/persistence/analytics and automation paths with deterministic mixed
chat/gift/like events. It never opens a TikTok network connection. Cases except
G configure an actual `core.log` chat workflow. URLs use a fixed local PNG with
100 stable viewer URLs, avoiding CDN variance. This cannot prove behavior with
real changing CDN images or slow external TTS/plugins. Case I requires separate
plugin/TTS fixtures and is not implemented by this harness yet.

`tests/e2e/live-lifecycle.spec.ts` exercises 100 actual Vue root remounts and
captures three Chromium heap snapshots. This covers real Vue disposal through a
fake IPC host, not native WebKit/WebView2 heaps. Inspect strong retaining paths
separately from WeakMap/ephemeron snapshot entries and fixture history. Native
plugin-window IPC teardown and the external-destruction workaround are retained.

## Latest native lifecycle evidence

The isolated Linux native test completed 100 actual main-page reloads while
processing 110,000 synthetic events. Page generation ended at 101 and script
evaluation errors stayed zero. This verifies reload/transport recovery through
the actual compiled Vue/Wry page; it does not provide WebKit retained-heap or
Windows lifecycle proof.

## Outstanding gates

- Comparable baseline/candidate release A–J matrices, >=10-minute load and
  >=2-minute cleanup; 30-minute DOM/image resource stability where applicable.
- Actual native root/page reload/reconnect/plugin-frame and native-window
  lifecycle cycles; retained-object inspection in WebKit and WebView2.
- Windows runtime validation, real LIVE and slow plugin/TTS isolation.
- Frame/layout/decoded-image profiling before deciding on image concurrency,
  caching or list windowing. No decoded-image cache or virtualization dependency
  has been introduced. Pinned 100-row rendering has an isolated native A/B.
- ACK/evaluate/parse metrics are instrumented. Alternative transports and batch
  sizes still require measured comparisons; existing ACK and queue budgets stay.

These gates remain open. Passing synthetic correctness tests does not close them.
