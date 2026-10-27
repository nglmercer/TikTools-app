# TikTok likes: retrocompatible thresholds and batched recurring goals

## Contracts that must never change

- `tiktok.like` remains the trigger name.
- `event.data.count` is the number of likes in **this received TikTok packet**. Do not convert it to 1 or replace it with the running total.
- `event.data.total` is the TikTok **room-wide snapshot**, not a delta and not the application's observed counter.
- `event.data.aggregation.global` and `.user` contain the existing observed `{previous,total}` transitions; they are derived from accepted packet deltas.
- Previously saved filters (`gte`, `lt`, `eq`, etc.), rule-template version 1, stable goal-template IDs (`likes-goal-global`, `likes-goal-user`), and existing one-shot semantics are preserved. **No migration**.

## Trigger styles (explicit, not inferred)

| Rule | Example | Outcome |
| --- | --- | --- |
| Legacy packet filter | `data.count >= 150` | Once per qualifying packet, exactly as before |
| Legacy one-shot counter | `aggregation.global.previous < 150 AND aggregation.global.total >= 150` | Once on the first threshold crossing |
| New recurring counter | `aggregation.global crosses-multiple 150` | One run for **each** observed 150-like milestone |
| New per-user recurring counter | `aggregation.user crosses-multiple 150` | Each tracked user earns their own milestones |

New recurring rules are **opt-in**. The editor registers separate “Every X likes” condition templates alongside the unchanged one-shot presets. The web parser, editor, native matcher, and CLI template importer all recognize `crosses-multiple`; opening/saving a new filter must never silently turn it into `eq`.

## 150-like batch test

Received deltas: `45, 60, 45, 300`. Observed totals: `45, 105, 150, 450`.

- Recurring 150: **3 action runs** (at 150, 300, 450).
- Old one-shot 150: **1 action run** (at 150 only).
- Old `data.count >= 150`: **1 action run** (the 300-like packet).
- A room-wide `total` of e.g. 50,000 does not advance the local counter by 50,000.
- Replaying the same packet identity must not execute a milestone twice.

The Rust `e2e_batched_likes_trigger_saved_150_goals_without_breaking_old_templates` test sends synthetic native `LikeEvent`s through the production deduplication, observation, normalization and automation dispatch code, asserting real (non-dry-run) `core.log` action history. The TS `recurring-likes.test.ts` test verifies existing templates, the optional new condition, editor normalization and web template parsing.

The E2E regression requires the optional `native-tiktok` Cargo feature. Without it, Cargo reports `0 passed` because the test module is behind `#[cfg(feature = "native-tiktok")]`; a zero-test success is **not** validation. CI now checks that the native test is registered and executes it explicitly.

Run:
```sh
cargo test -p tiktools-core --locked --features native-tiktok --lib e2e_batched_likes_trigger_saved_150_goals_without_breaking_old_templates -- --nocapture
cargo test -p tiktools-core --locked like_goal_tests
bun test src/automation/behavior/recurring-likes.test.ts
bun run typecheck
cargo fmt --all -- --check
```

## Bounded execution and observability

One batch can cross multiple thresholds. The runtime runs the configured actions once for each crossed milestone, **up to 64 per record per packet**, and emits a warning if the batch crosses more. The limit prevents malicious or pathological batches from launching unbounded work; it is not a lossless deferred queue. Cooldowns are claimed once per packet; a nonzero cooldown may suppress later packets as before. Use `cooldownMs: 0` for exact recurring goals.

The existing bounded ingress can shed likes during saturation. Rust diagnostics expose `droppedLikeBatches` and `droppedLikeUnits`, and note an event gap. These are lower bounds on locally known losses, not a mechanism to reconstruct missing TikTok events. Upstream delivery can also be incomplete, particularly for per-user batches.

References for product semantics: BakoshApp documents per-user/global like targets, remainder carry-over, and repeated firing for large batches; TikFinity exposes per-event likeCount and per-user accumulated total. These are design comparisons, not guarantees about TikTok delivery.
