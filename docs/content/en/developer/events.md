---
title: "Event normalization, batches and gaps"
docId: "developer.events"
translationKey: "developer.events"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "caded5529a62e826a91d2deb86936c29c709ac713c2a0cd021f467b1dfa2b6e7"
translationRevision: "caded5529a62e826a91d2deb86936c29c709ac713c2a0cd021f467b1dfa2b6e7"
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

Reconnect overlap is deduplicated only while the identified event remains in the current finite replay store. Queues, counters, history and replay caches remain bounded. This is not an unlimited event archive or a universal exactly-once guarantee. Capacity exhaustion and unavailable persistence have explicit health consequences.

## Recover and verify {#recover-and-verify}

Use sanitized gift/emote fixtures and test final-combo packets, missing IDs, cross-room identity and reconnect gaps. Inspect the current `Cargo.toml` signer revision rather than copying a hash from an old reliability report. Unknown upstream methods stay unknown until normalization and generated contracts deliberately add them.

## Operational details {#operational-details}

Current defaults (2026-11-13) are limits of this implementation, not an SLA. LIVE ingress holds 2,048 items and the effects lane 128; these are item counts, not byte budgets. Lifecycle events wait for capacity. Gifts/follows/shares wait at most one second before a gap-reported drop. Chat/likes/joins/other data drop immediately when saturated. Inspect ingress counters including `droppedCritical`, dropped like batches/units and effects losses; a dropped input can mean its effects never ran. Backfill repairs presentation only, never points or actions.

Deduplication holds at most 4,096 identified events for 300 seconds in process memory, surviving reconnects only while that store lives. TTL expiry, capacity eviction, missing IDs or restart remove that protection. Reconnection overlap is protected only within these bounds.

`ui.feed.backfill` takes `{version:1,after:0,limit:64}` with global numeric cursors and limits 1–512; inspect `oldestCursor`, `latestCursor`, `gap` and `truncated`, continuing from the last received event cursor. `ui.feed.creator` uses `{creatorId,streamId,after,limit}` with string cursors, a creator-specific stream, limits 1–64 and a byte budget. Do not interchange cursor types. On changed/evicted streams, future cursors, ingress gaps or `CreatorFeedUnavailable`, refresh authoritative presentation and explicitly mark missing history; never synthesize effects.

TikTok initialization performs session, room, gift catalog, signing-bundle and socket stages. Bundle configuration: `TIKTOOLS_TIKTOK_SIGNING_BUNDLE`, `TIKTOOLS_TIKTOK_BUNDLE_CACHE`, `TIKTOOLS_TIKTOK_BUNDLE_URL`. Defaults allow five attempts, initial 2 s backoff capped at 30 s. HTTP 429 uses Retry-After or an increasing 30–300 s minimum plus jitter, at most two rate-limit retries, with cooldown retained after cancellation/exhaustion. Other retries require transient transport failures. Inspect `Reconnecting { attempt, delay_ms }` and the failing stage; never attach cookies or private signing material to reports.

```sh
tiktools api call system.health '{}' --json
```
