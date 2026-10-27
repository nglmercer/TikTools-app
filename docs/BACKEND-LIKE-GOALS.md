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
