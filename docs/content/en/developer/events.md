---
title: "Event normalization, batches and gaps"
docId: "developer.events"
translationKey: "developer.events"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
translationRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
order: 100
---

# Event normalization, batches and gaps {#event-normalization-batches-and-gaps}

## Goal and prerequisites {#goal-and-prerequisites}

Consume LIVE events without double counting or mistaking presentation recovery for execution. Understand [persistence](storage.md) and use sanitized fixtures rather than real account captures. `tiktools-tiktok` wraps the pinned signer/decoder dependencies and exports stable TikTools values; core does not expose protobuf objects as an application API.

## Follow the implemented pipeline {#follow-the-implemented-pipeline}

Ingress captures the original creator/room connection context before asynchronous work. Normalization produces chat, gift, like, social and lifecycle values. Core publishes originals and manages accounting/effect work separately. Processors may attach bounded derived evidence under `event.intel`; failures preserve raw events with degraded diagnostics. Generated automation contracts describe only emitted fields.

Backend replay identity includes creator, room, method, type and a nonzero message ID. Missing IDs cannot establish replay. Gift combo accounting uses sender/gift/group identity and cumulative maxima, so a final packet cannot award the same quantity twice. Unknown prices remain zero. Aggregate transitions can support a threshold crossing using `previous < threshold <= total`; they do not replace public originals.

## Implement a consumer {#implement-a-consumer}

1. Use types from `src/automation/contracts/generated/` or the widget domain envelope parser for that boundary.
2. Preserve delivery cursors through serialization and deduplicate overlap when joining realtime and backfill.
3. Process ordered batches without issuing one reactive render per event.
4. Surface explicit gap/overload signals. Recover presentation with bounded pages or a fresh snapshot, never by invoking points or actions again.

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts test bun automation
```

The [Rust contract source](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-core/src/contracts) generates the event registry, schemas and samples; do not reproduce complete event-field tables by hand.

## Expected result and status {#expected-result-and-status}

Reconnect overlap does not duplicate displayed deliveries or supported replay-protected accounting. Queues, counters, history and replay caches remain bounded. This is not an unlimited event archive or a universal exactly-once guarantee. Capacity exhaustion and unavailable persistence have explicit health consequences.

## Recover and verify {#recover-and-verify}

Use sanitized gift/emote fixtures and test final-combo packets, missing IDs, cross-room identity and reconnect gaps. Inspect the current `Cargo.toml` signer revision rather than copying a hash from an old reliability report. Unknown upstream methods stay unknown until normalization and generated contracts deliberately add them.
