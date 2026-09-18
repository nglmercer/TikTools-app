# TikTools — Fix Final Remaining Reliability Issues

Reviewed head:

```text
57e80055f2e7da0d744c77562c6520e3314f5cc9
```

## Goal

Finish the control-plane reliability work.

Current architecture is mostly correct. Fix only the remaining event-delivery and transport edge cases below.

---

## 1. Make Reliable Domain Events Actually Reliable

Current `EventBus` uses two bounded `broadcast` channels:

```rust
reliable: broadcast::Sender<DomainEvent>,
lossy: broadcast::Sender<DomainEvent>,
```

The lossy/reliable split prevents live-feed floods from overwriting control events, but the reliable lane itself can still return:

```rust
RecvError::Lagged(n)
```

This can lose:

```text
points.changed
live.connected
live.disconnected
live.error
plugin.started
plugin.stopped
workflow.changed
shutdown
```

Do not silently lose authoritative events.

Preferred design:

```text
DomainEvent
   ├── reliable lane
   │     -> guaranteed delivery or explicit resync
   │
   └── lossy lane
         -> bounded/coalesced/feed
```

Implement one of:

### Preferred

Per-subscriber bounded reliable queues with backpressure/disconnection semantics.

### Acceptable

Keep bounded broadcast, but whenever reliable lag occurs emit an explicit gap/resync signal.

Example:

```json
{
  "method": "event.gap",
  "params": {
    "lost": 12,
    "resync": true
  }
}
```

Clients must then refresh authoritative state.

Never pretend a lagged reliable stream is complete.

---

## 2. Fix IPC Event Lag Handling

Current IPC path effectively does:

```rust
receiver.recv().await.ok()
```

This converts:

```rust
RecvError::Lagged(n)
```

into:

```text
None
```

and silently loses information.

Replace with explicit handling:

```rust
match receiver.recv().await {
    Ok(event) => send_event(event),

    Err(RecvError::Lagged(count)) => {
        tracing::warn!(count, "control IPC event subscriber lagged");
        send_gap_notification(count).await?;
    }

    Err(RecvError::Closed) => {
        // terminate event stream
    }
}
```

Do the same for:

```rust
drain_events()
```

Do not silently stop draining when `try_recv()` returns `Lagged`.

Add tests proving:

```text
event burst
↓
Lagged
↓
connection remains alive
↓
client receives gap/resync notification
↓
later events still arrive
```

---

## 3. Add Client Resync Handling

Extend both Rust `ControlClient` and WebView `control-client.ts` to understand an event-gap notification.

Example:

```text
event.gap
```

Expose a Rust API such as:

```rust
ControlEvent::Domain(event)
ControlEvent::Gap { lost: u64 }
```

Frontend should support:

```ts
control.onGap(() => {
  void refreshAuthoritativeState();
});
```

At minimum resync:

```text
live status
points leaderboard/config
plugin state
creator state
workflow/automation state as needed
```

Do not attempt to reconstruct missing reliable events from guesses.

---

## 4. Bound the Reliable WebView Queue Safely

Current reliable WebView lane:

```rust
reliable: VecDeque<String>
```

never drops messages, but is unbounded.

That changes failure mode from message loss to unlimited memory growth if WebView becomes stuck.

Keep reliable messages non-droppable, but add a byte/backlog safety policy.

Example:

```text
MAX_RELIABLE_MESSAGES = 4096
MAX_RELIABLE_BYTES = 16 MiB
```

When exceeded:

```text
DO NOT DROP RPC RESPONSES
DO NOT DROP STATE TRANSITIONS

instead:
  mark WebView transport unhealthy
  stop accepting new WebView RPC
  reject pending/new RPC with transport failure where possible
  recreate/reload/fail the WebView cleanly
```

A broken UI transport must fail loudly rather than consume unlimited memory.

Track both:

```rust
reliable.len()
reliable_bytes
```

Add tests with large critical-message bursts.

---

## 5. Fix Malformed WebView JSON-RPC Routing

Current logic parses once in:

```rust
is_control_rpc(&raw)
```

If JSON is malformed, it returns false, so malformed RPC-shaped input can fall into the legacy router.

Then this intended error path is unreachable:

```rust
Err(error) => RpcResponse::error(
    RpcId::extract_from_prefix(&raw),
    ...
)
```

Refactor to parse exactly once.

Target:

```rust
match serde_json::from_str::<serde_json::Value>(&raw) {
    Ok(value) => {
        if value.get("method").is_some() {
            control.execute_value(&value).await
        } else {
            legacy_dispatch(value).await
        }
    }

    Err(error) => {
        if is_probably_control_rpc(&raw) {
            send_rpc_error(
                RpcId::extract_from_prefix(&raw),
                ApiError::invalid_params(
                    format!("invalid JSON: {error}")
                ),
            );
        } else {
            tracing::warn!(%error, "invalid legacy WebView message");
        }
    }
}
```

Avoid:

```text
parse
↓
classify
↓
parse again
```

The Winit callback should only:

```text
size-check
special-case frontend-ready
move payload to Tokio
```

Do parsing/classification off the UI thread.

---

## 6. Keep WebView Callback Lightweight

Audit:

```rust
with_ipc_handler(...)
```

Do not perform expensive JSON parsing on the Winit thread.

Preferred:

```text
Wry callback
   ↓
raw size check
   ↓
frontend-ready tiny fast path
   ↓
Tokio task
      -> parse
      -> classify
      -> execute
```

Protect the UI loop from malicious or very large valid JSON.

---

## 7. Add Explicit Event Stream Health

Expose event-stream status through:

```text
system.health
```

Include fields similar to:

```json
{
  "events": {
    "status": "ok",
    "reliableGaps": 0,
    "lastGapAt": null
  }
}
```

After a reliable gap:

```text
status = degraded
reliableGaps += 1
```

After successful resync, health may return to OK if appropriate.

Agents must be able to tell whether their local state may be stale.

---

## 8. Tests

Add regression/stress tests for:

```text
reliable-event burst causes explicit gap, not silent loss
lossy flood never causes reliable gap
IPC continues after Lagged
post-gap events still arrive
ControlClient exposes gap notification
WebView triggers resync on gap
reliable WebView queue never silently drops messages
reliable queue over-limit fails transport cleanly
malformed JSON-RPC returns correlated error id
malformed legacy payload does not enter ControlApi
large valid JSON parsing does not run on Winit thread
```

All async tests need bounded timeouts.

---

## 9. Verification

Run:

```bash
cargo fmt --all -- --check

cargo check \
  --workspace \
  --all-features \
  --locked

cargo clippy \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  -- -D warnings

cargo test \
  --workspace \
  --locked

bun run lint
bun run typecheck
bun test
bun run build:web

git diff --check
```

Then manually verify:

```bash
bun run scripts/start-dev.ts
```

Test:

```text
WebView RPC
CLI RPC
2+ concurrent IPC clients
live event burst
points burst
plugin lifecycle burst
WebView temporarily stalled
IPC client temporarily stalled
recovery after event gap
shutdown
restart
```

---

# Definition of Done

```text
[ ] reliable DomainEvents cannot disappear silently
[ ] reliable lag produces explicit gap/resync signal
[ ] IPC does not swallow Lagged
[ ] Rust ControlClient reports gaps
[ ] WebView client reports/resyncs gaps
[ ] reliable WebView queue has bounded memory
[ ] reliable messages are never silently evicted
[ ] broken WebView transport fails loudly
[ ] malformed JSON-RPC receives correlated error
[ ] JSON parsing is off Winit thread
[ ] system.health reports event-stream degradation
[ ] stress tests cover all failure paths
[ ] Rust checks pass
[ ] Bun checks pass
```

## Core Invariant

```text
Authoritative event:
    delivered
       OR
    client explicitly told it missed data and must resync

Never:
    silently dropped
```

Lossy feed traffic may be dropped/coalesced.

RPC responses and authoritative state transitions may not.
