---
title: "Performance and bounded work"
docId: "developer.performance"
translationKey: "developer.performance"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c60beed6316f8a58ee9d7251682c1ecebb501b53bbe6fadcf5a3b9166f22a723"
translationRevision: "c60beed6316f8a58ee9d7251682c1ecebb501b53bbe6fadcf5a3b9166f22a723"
order: 110
---

# Performance and bounded work

## Goal and prerequisites

Measure a concrete bottleneck while preserving event accounting and shutdown. Start with a fixed build, deterministic input and an isolated profile. Read [events](events.md) so a faster feed is not confused with more reliable persistence. Historical benchmark numbers are observations of their named build and machine, not current product guarantees.

## Measure the right boundary

The desktop outbox separates lossy presentation from reliable responses. Snapshots can coalesce; reliable overflow must fail visibly. LIVE ingress, plugin work, processor calls and automation admission each have their own bounds. Increasing one queue can move memory pressure or latency into the next stage.

The frontend runtime exposes aggregate diagnostics in development or when built with `VITE_TIKTOOLS_DIAGNOSTICS=1`. Its snapshot includes rendered rows, frame-interval percentiles, control-client pending calls, images and owned timers. JavaScript heap data can be unavailable on a particular WebView. Hidden pages and host visibility govern presentation interest, not business accounting.

## Run a focused comparison

1. Record build revision, platform, event mix, images enabled, visible tab and duration.
2. Establish a baseline with the same fixture and sample period.
3. Change one boundary, then compare throughput, p95/p99 latency, queue drops, gaps, retained memory and shutdown drain behavior.
4. Include a slow plugin or consumer and a saturated queue; an idle benchmark does not test backpressure.

```sh
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts rust test -p tiktools-core --features persistence,native-tiktok --test live_throughput --locked
```

Inspect feature requirements before enabling a native benchmark; ordinary focused unit checks need no real TikTok connection. Use `crates/tiktools-core/tests/live_throughput.rs` and the desktop outbox tests as implementation references.

## Expected result and status

Bounded projection and admission paths are implemented, with counters and gap reporting. The [current ordered writer](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/points/queue.rs) rejects saturated writes before changing memory and forbids synchronous write fallback. Older performance reports describe superseded ordering defects. Assess cache bounds and failure recovery against current implementation/tests rather than infer complete correctness from bounded UI memory.

## Recover

If frames improve while accounting health degrades, revert the boundary change and inspect writer/admission outcomes. If hidden-tab timers grow, follow [lifecycle cleanup](lifecycle.md). Keep raw input count, accepted count and loss classification in the report. A gap can recover display state; it cannot repair an effect that was rejected or became uncertain.
