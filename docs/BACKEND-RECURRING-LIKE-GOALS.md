# Rust backend: recurring batched-like goals

The native TikTok SDK reports a **batch delta** in `tiktok.like.data.count` and a **room-wide snapshot** in `data.total`. Do not interpret either as one physical like per websocket message.

The existing `aggregation.global` and `aggregation.user` objects have `previous` and `total` **observed** counters. To fire again at multiples of an interval, native Rust now accepts a filter:

```json
{"path":"event.data.aggregation.global","operator":"crosses-multiple","value":"15"}
```

The Rust predicate is `total > previous && total / 15 > previous / 15`. It matches on transitions `14 -> 15`, `29 -> 30`, and `30 -> 45`, and not `15 -> 15` or `15 -> 29`. For a per-viewer recurring goal, change `global` to `user`. A missing user scope fails closed. Zero/invalid step values fail closed.

Import a runnable backend-only sample through the CLI:

```sh
tiktools template import examples/templates/likes-every-15-global.json
```

Replace its `core.log` action with the desired action and adjust `value` as needed. The CLI and Rust template parser support this operator; this change intentionally **does not modify the web operator list or the one-shot web goal template**. Web-based editing will not preserve the new operator until a separate UI change.

One incoming packet can cross **several multiples**. The backend currently executes at most **one action set for that matching packet**; it does not split one TikTok event into artificial single-like events. If the product requires one action for *every* crossed multiple in a single packet, add a bounded native multi-crossing execution policy in a separate change.

On ingress saturation, high-volume like events are still sheddable to protect the WebSocket transport. Rust health diagnostics now expose `droppedLikeBatches` and `droppedLikeUnits`, and record a stream gap for lost like batches. These reported units are the lower bound of locally observed missing likes, not permission to replay missed actions. Use `tiktools rpc system.health '{}'` to diagnose. Unknown upstream losses may still be invisible.

The normalized original `tiktok.like` contract, points, UI payloads, and the existing one-shot goal behavior are unchanged. Always calculate recurring milestones from **accepted packet deltas**, never by adding the room-wide snapshot again.
