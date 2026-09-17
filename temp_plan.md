# TikTools — Finish Remaining IPC/Event Reliability Fixes

Repository:

```text
nglmercer/TikTools-app
```

Branch:

```text
remake
```

Current reviewed commit:

```text
386094e0f280a19a18075a2983c5f56b8f45cac8
```

## Goal

Finish the control-plane refactor and eliminate the remaining event-loss, WebView queue, IPC ownership, and blocking-I/O problems.

Target:

```text
ONE AppCore
   │
ControlApi
   ├── WebView
   ├── IPC
   ├── CLI
   └── agents

DomainEvent = authoritative push/event channel
```

---

# 1. Fix WebView DomainEvent `Lagged`

Current bug:

```rust
match events.recv().await {
    Ok(event) => event,
    Err(_) => break,
}
```

`tokio::broadcast::RecvError::Lagged(_)` is recoverable.

Required:

```rust
match events.recv().await {
    Ok(event) => {
        // forward
    }

    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
        tracing::warn!(count, "WebView domain event receiver lagged");
        continue;
    }

    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
}
```

A temporary burst must never permanently disable WebView events.

Add a regression test.

---

# 2. Never Drop RPC Responses from WebView Queue

Current bounded queue may eventually do:

```rust
queue.pop_front();
```

This can drop:

```text
rpc-response
errors
lifecycle events
important state transitions
```

Implement message priority.

Suggested classes:

```text
CRITICAL — never drop
  rpc-response
  live.connected
  live.disconnected
  plugin lifecycle
  errors
  shutdown

COALESCABLE
  room.stats
  leaderboard snapshot
  analytics.updated
  processor metrics
  automation context

DROPPABLE
  high-rate UI/live feed events when queue is saturated
```

Do not use blind FIFO eviction.

If the queue contains only critical messages, apply backpressure or grow only within a second hard safety limit rather than dropping RPC responses.

Add tests proving RPC responses survive saturation.

---

# 3. Fix WebView Batch Tail Stall

Current:

```text
MAX_BATCH_PER_TICK = 128
```

On Windows/macOS:

```text
flush 128
remaining messages > 0
ControlFlow::Wait
no new event
remaining messages can stall
```

Required:

After a batch:

```rust
if !self.pending_host_messages.is_empty() {
    schedule_another_ui_wake();
}
```

Preferred design:

```text
queue not empty
  -> send/retain FlushWebviewBatch event
  -> next Winit turn
  -> flush next batch
```

Do not busy-loop.

Add a test with >128 queued messages proving all batches eventually drain.

---

# 4. Coalesce New Domain Events Too

Current coalescer mainly understands legacy:

```json
{"type":"room-stats"}
```

It must also understand:

```json
{
  "method": "event",
  "params": {
    "topic": "room.stats"
  }
}
```

Coalesce domain topics such as:

```text
room.stats
analytics.updated
processor metrics/status snapshot
leaderboard snapshot if represented as a domain topic
automation context snapshot
```

Never coalesce:

```text
live.ui-event
rpc-response
plugin lifecycle
errors
shutdown
```

Create one message-classification function shared by queue policy and tests.

---

# 5. Clear IPC Health When Retry Successfully Binds

Current logic clears:

```rust
core.set_ipc_error(None)
```

only after `run_ipc_shared()` returns successfully.

But a healthy server normally remains inside its accept loop until shutdown.

Required architecture:

```text
bind/claim endpoint
   ↓
server ready callback/signal
   ↓
core.set_ipc_error(None)
   ↓
accept loop
```

Possible API:

```rust
run_ipc_shared_with_ready(
    control,
    || core.set_ipc_error(None),
)
```

or separate:

```rust
let server = bind_ipc(...)?;
core.set_ipc_error(None);
server.run().await;
```

Health must transition:

```text
degraded -> ok
```

as soon as IPC is actually listening again.

Add test.

---

# 6. Strengthen Unix Control-Host Ownership

Current Unix ownership relies mostly on socket bind/stale socket cleanup.

Add a real per-user lock held for server lifetime.

Preferred:

```text
$TIKTOOLS_HOME/tiktools-control.lock
```

using:

```text
flock / fs2 / equivalent advisory file locking
```

Flow:

```text
acquire ownership lock
   ↓
inspect/remove stale socket
   ↓
bind Unix socket
   ↓
hold lock until server exits
```

Never unlink a possibly-live socket before acquiring exclusive ownership.

Keep:

```text
socket permissions = 0600
```

Add tests:

```text
first owner succeeds
second owner fails
stale socket cleanup works
lock released after shutdown
```

---

# 7. Complete Blocking-I/O Audit

Move synchronous persistence/filesystem work out of async Tokio handlers.

Audit at minimum:

```text
app.state.*
creators.*
workflows.*
plugins.settings.*
points.*
gifts.*
analytics.*
plugin install/uninstall
filesystem scans
archive/file operations
```

Example:

```rust
let result = tokio::task::spawn_blocking(move || {
    core.workflow_save(graph)
})
.await
.map_err(...)?;
```

Do not unnecessarily wrap pure in-memory operations.

Target rule:

```text
SQLite / filesystem / archive / sync plugin I/O
    -> spawn_blocking

network async
    -> normal async

pure memory
    -> direct
```

---

# 8. Complete DomainEvent Migration

Add domain equivalents for remaining legacy connection pushes.

Add:

```text
live.reconnecting
live.error
```

Suggested variants:

```rust
LiveReconnecting {
    attempt: u32,
    delay_ms: u64,
}

LiveError {
    phase: String,
    message: String,
}
```

Frontend should consume:

```ts
control.onTopic('live.reconnecting', ...)
control.onTopic('live.error', ...)
```

Remove corresponding frontend legacy push subscriptions after migration.

---

# 9. Stop Sending Duplicate Legacy Events

Once frontend consumes the domain equivalent, stop emitting duplicate legacy messages for that state.

Examples to remove when safe:

```text
HostMessage::LiveEvent
HostMessage::RoomStats
HostMessage::PointsAwarded
HostMessage::GiftCatalog
HostMessage::PluginProgress
```

Do this incrementally only after the frontend no longer depends on them.

Keep compatibility only where genuinely required.

Goal:

```text
one event mutation
   ↓
one DomainEvent
   ↓
all clients
```

---

# 10. Fix New Viewer Handling in `points.changed`

Current frontend logic:

```ts
const index = leaderboard.value.findIndex(...);

if (index < 0) return;
```

But `points.adjust` may create a new viewer.

Fix with one of:

### Preferred

Make `points.changed` carry enough information to construct/update a full viewer record.

or:

### Acceptable

If viewer is missing:

```ts
void refresh();
```

Do not silently ignore a newly created viewer.

Add regression test:

```text
empty leaderboard
points.adjust("new-user", 10)
points.changed received
new-user appears
```

---

# 11. Add Raw WebView IPC Size Limit

Before:

```rust
serde_json::from_str(&raw)
```

validate:

```rust
if raw.len() > MAX_REQUEST_BYTES {
    return/send request_too_large;
}
```

Use the same limits as local IPC:

```text
MAX_REQUEST_BYTES
MAX_PARAMS_BYTES
```

Do not let WebView bypass transport limits.

Add malformed/oversized request tests.

---

# 12. Improve Dev Stale-Host Detection

Current dev launcher probes control IPC.

Also make startup robust against stale desktop ownership.

Requirements:

```text
existing control host -> fail
existing desktop single-instance owner -> fail
Vite process from current launcher only
actual selected Vite port only
```

Prefer a small desktop/control probe with PID/version info if possible.

Dev startup must never produce:

```text
new Vite + old desktop
old IPC host + new frontend
```

---

# 13. Make IPC Client Reader Fail Loudly on Bad Wire Data

Current reader silently skips malformed JSON/event payloads.

Improve diagnostics:

```rust
tracing::warn!(..., "invalid control IPC response");
```

For malformed responses matching a pending ID, fail the pending request instead of leaving it until timeout where possible.

Do not crash the connection for one malformed event notification unless framing is corrupt.

---

# 14. Add Event-Loss / Queue Stress Tests

Add tests for:

```text
broadcast Lagged does not terminate WebView event forwarder
>128 WebView messages fully drain
queue saturation does not drop rpc-response
domain snapshots coalesce
live events remain ordered enough for UI use
event subscription continues after bursts
IPC reconnect clears degraded health
Unix ownership lock
new viewer points.changed
oversized WebView RPC rejected
```

Use bounded timeouts on every async test.

No test may hang indefinitely.

---

# 15. Verification

Run:

```bash
cargo fmt --all -- --check

cargo check --workspace --all-features --locked

cargo clippy \
  --workspace \
  --all-targets \
  --all-features \
  --locked \
  -- -D warnings

cargo test --workspace --locked

bun run lint
bun run typecheck
bun test
bun run build:web

git diff --check
```

Then manually test:

```bash
bun run scripts/start-dev.ts
```

Verify:

```text
one Vite process
one desktop host
actual Vite URL passed to desktop
CLI connects to same AppCore
two concurrent IPC clients work
domain events continue after event bursts
automation saturation does not drop live domain events
WebView survives >512-event burst
RPC responses never disappear
new point viewers appear
IPC health recovers after temporary failure
desktop shutdown releases IPC ownership
```

---

# Definition of Done

```text
[ ] Lagged broadcast does not stop WebView events
[ ] RPC responses cannot be evicted from WebView queue
[ ] >128 queued messages always continue draining
[ ] domain-event snapshots are coalesced correctly
[ ] IPC health clears after successful rebind
[ ] Unix has real exclusive ownership locking
[ ] blocking SQLite/filesystem operations are off Tokio async workers
[ ] reconnect/error use DomainEvent
[ ] duplicate legacy pushes removed where migrated
[ ] new points viewers update correctly
[ ] WebView IPC has request-size limits
[ ] stale dev desktop/host detection is reliable
[ ] stress/regression tests cover event bursts
[ ] Rust checks/tests pass
[ ] Bun checks/tests/build pass
```

## Core Rule

Do not fix these with feature-specific patches.

Keep the architecture:

```text
state mutation
   ↓
AppCore
   ↓
DomainEvent
   ↓
ControlApi transport
   ↓
WebView / CLI / agents
```

RPC responses and authoritative state transitions must never be silently dropped.
