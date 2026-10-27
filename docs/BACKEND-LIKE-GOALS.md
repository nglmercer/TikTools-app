# TikTok like batch goals: compatibility and validation

TikTok's `LikeEvent.count` is the **batch increment**, while `LikeEvent.total` is a room-wide snapshot. Never add the room snapshot as an increment. The existing normalized `event.data.aggregation.global` and `.user` counters expose observed `{previous,total}` transitions accumulated from admitted batches.

## Existing rules (unchanged)

| Rule | Existing filters | Behavior |
| --- | --- | --- |
| Room goal of 150 likes | `aggregation.global.previous < 150` AND `aggregation.global.total >= 150` | Fires **once** when observed room likes first reach or skip over 150 |
| Viewer goal of 150 likes | `aggregation.user.previous < 150` AND `aggregation.user.total >= 150` | Fires **once** for each viewer whose observed likes cross 150 |
| Per-packet count of 150 likes | `event.data.count >= 150` | Fires once for every received batch containing 150 or more likes |

All use existing operators (`lt` and `gte`). This PR adds no new filter operator, no stored-rule migration, no new condition templates, and no changes to the existing IDs `likes-goal-global` and `likes-goal-user`.

## Simulated integration regression

Send native TikTok packets from one user, with deltas `45 + 60 + 45 + 300 = 450` and unrelated room snapshot totals. Expected real automation action runs:
- Room goal at 150: **one**
- Viewer goal at 150: **one**
- Old per-packet `count >= 150` rule: **one**, from the 300-like packet
- Duplicate replay of the final message: **zero additional runs**

A separate test verifies observed `LikeEvent.count` accumulation and that a room snapshot does not inflate the counter. Ingress loss diagnostics include `droppedLikeBatches` and `droppedLikeUnits`, distinguishing dropped packets from filter mismatches.

## Validation

The native E2E test requires the **optional** Cargo feature `native-tiktok`. Running without it reports zero tests and is not meaningful validation. CI explicitly lists this test first so it cannot silently pass when filtered out.

```sh
cargo test -p tiktools-core --locked --features native-tiktok --lib e2e_batched_likes_trigger_saved_150_goals_without_breaking_old_templates -- --nocapture
bun test src/automation/behavior/like-goals.test.ts
bun run typecheck
cargo fmt --all -- --check
```

CI and local re-runs of these commands are required before merging.


## Feed presentation: one visible row for a short like burst

The native backend publishes each `live.ui-event` unchanged **except** for two
new optional, presentation-only fields when the per-user counter is available:

- `likeUserTotal`: observed cumulative likes from this viewer in the current creator/room session (not TikTok's room `total` snapshot).
- `likeViewerKey`: a stable viewer ID, used to avoid grouping different viewers.

The original `likeCount` and `i18nParams.count` remain the incoming packet size.
No data is dropped from the authoritative feed history or the automation, points
or analytics pipeline.

The frontend displays the viewer's `likeUserTotal` and folds **visual-only**
rows for the same viewer and room when packets arrive within three seconds.
It updates the latest timestamp/message ID, preserves the original row ID,
and moves the row to the latest visible position. After three seconds of no
updates, a new row may appear but still displays the cumulative viewer total.
Rows without a reliable viewer identity or a safe cumulative total are not
merged and display their original per-packet count.

Likes are **not** TikTok gift streak/combo events. Existing gift grouping
remains unchanged. The inspector distinguishes the last packet count from the
session viewer total.

Visual regression coverage: `src/web/entities/live-event/model/live-feed-state.test.ts`
and `src/web/entities/live-event/lib/event-text.test.ts`. The native E2E
test also checks the UI projection reports `45, 105, 150, 450` alongside
the untouched packet sizes `45, 60, 45, 300`.

```sh
bun test src/web/entities/live-event/model/live-feed-state.test.ts src/web/entities/live-event/lib/event-text.test.ts
```


## Live likes leaderboard

The core maintains a separate, bounded **top-ten likes** ranking for each
creator and room. Only admitted `LikeEvent.count` increments contribute,
including multiple packets from the same stable viewer identity. TikTok's
room-wide `LikeEvent.total` is never added as a delta. Duplicate replay
is rejected before tallying.

- New lossy domain topic: `live.likes-ranking`, carrying `creatorId`,
  `roomId`, and a sorted `leaders` array with `rank`, `score`,
  `uniqueId`, `nickname`, `userId`, and optional `avatarUrl`.
- A snapshot is published on room connection/reconnection and when a
  viewer inside the top ten changes score. This is independent of UI
  rendering, points, and existing automation templates.
- The `Espectadores` panel supports **General / Me gusta**; the latter
  displays the core ranking and can open a viewer profile.
- Counts are observed **per live room**, not an all-time account tally.
  Rankings do not mix different creators or rooms.
- The feed text uses short copy (`envió 35 me gusta`), but preserves
  the last packet count separately in the event inspector.

Regression tests are in `live_events/totals.rs`,
`live_events/mapping.rs`, `live-feed-state.test.ts`, and
`FeedPage.test.ts`.

```sh
cargo test -p tiktools-core --locked --features native-tiktok --lib like_leaderboard_ranks_only_observed_packet_deltas_per_room
bun test src/web/entities/live-event/model/live-feed-state.test.ts src/web/pages/feed/ui/FeedPage.test.ts
```
