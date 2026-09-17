# TikTools — Fix IPC, Events, and Dev Startup

Repository:

```text
https://github.com/nglmercer/TikTools-app
```

Branch:

```text
remake
```

## Goal

Make the control plane reliable:

```text
ONE AppCore
   │
ControlApi
   ├── WebView RPC
   ├── local IPC
   ├── CLI
   └── agents
```

No duplicated runtime state. No dropped events. No stale frontend/desktop processes.

## 1. Fix `scripts/start-dev.ts`

Current bug:

```text
script assumes port 3000
Vite may start on 3005
Rust still receives TIKTOOLS_DEV_URL=http://127.0.0.1:3000
```

Requirements:

* Never guess the Vite port.
* Start Vite programmatically or allocate a free port first.
* Use `strictPort: true`.
* Pass the actual URL to Rust.
* Do not consider another process on the requested port as the new Vite instance.
* Stop owned Vite process when desktop exits.

Expected:

```text
Vite -> actual URL
          │
          └── TIKTOOLS_DEV_URL
                    │
                 desktop
```

## 2. Prevent stale desktop instances in dev

Current single-instance behavior can cause the newly compiled desktop to exit while an old desktop continues running.

Requirements:

* Dev launcher must detect an existing TikTools desktop/control host.
* Fail clearly instead of silently using the old process.
* Never mix:

  * old desktop
  * new Vite
  * stale control IPC host

## 3. Fix Windows control IPC ownership

Do not use `ControlClient::connect()` as a lock.

Add an OS ownership primitive, e.g.:

```text
Local\TikTools.ControlHost
```

Rules:

```text
desktop
  -> acquire control-host mutex
  -> start named pipe

standalone host
  -> acquire same mutex
  -> fail if already owned
```

Only one control server may own the production endpoint.

## 4. Add Windows named-pipe connection retry

Current client performs one immediate `open()`.

Add bounded retry/backoff for transient errors such as:

```text
ERROR_FILE_NOT_FOUND
ERROR_PIPE_BUSY
```

Suggested:

```text
retry every 25-50ms
max ~2-5 seconds
```

Return `host_unavailable` only after the retry budget expires.

## 5. Make `DomainEvent` authoritative

UI/IPC event delivery must not depend on automation execution.

Wrong:

```text
TikTok
 -> automation slot
 -> processors
 -> remember_automation_event
 -> DomainEvent
```

Required:

```text
TikTok
 -> normalize
 -> DomainEvent immediately
      ├── WebView
      ├── IPC/CLI/agents
      └── automation pipeline
            -> may independently drop/throttle
```

Never drop control/UI events because automation concurrency is saturated.

## 6. Complete live-event migration

Remove frontend dependence on legacy pushes where a domain event exists.

Migrate:

```text
live-event
room-stats
connection state
points changes
plugin lifecycle/progress
creator changes
```

toward:

```text
control.onTopic(...)
```

Keep legacy `HostMessage` only as temporary compatibility.

Avoid sending the same event through both paths.

## 7. Give Rust `ControlClient` event subscriptions

Current Rust client skips:

```json
{"method":"event"}
```

Implement one reader task:

```text
IPC socket
   │
reader task
   ├── response id -> pending RPC promise
   └── event       -> broadcast/event channel
```

Provide API similar to:

```rust
let client = ControlClient::connect().await?;
let mut events = client.subscribe();

while let Some(event) = events.recv().await {
    // ...
}
```

Support multiple concurrent RPC requests.

Do not discard events.

## 8. Fix `processors.status` contract

Backend returns a snapshot object.

Frontend currently expects:

```ts
ProcessorStatusEntry[]
```

Make both sides typed and identical.

Preferred:

```ts
interface ProcessorStatusResult {
  processors: ProcessorStatusEntry[];
}
```

Then:

```ts
const result =
  await control.call<ProcessorStatusResult>(
    'processors.status',
    {},
  );

processors.value = result.processors;
```

Avoid untyped `Value` RPC results when possible.

## 9. Publish consistent domain events

Every state mutation must publish the same event regardless of origin.

Example:

```text
manual points adjustment
live TikTok award
automation adjustment
plugin action adjustment
```

should all produce:

```text
points.changed
```

Same rule for:

```text
plugin.*
workflow.*
creator.*
live.*
analytics.*
```

## 10. Prevent WebView event flooding

Do not call:

```rust
webview.evaluate_script(...)
```

once for every high-rate event.

Add bounded batching/coalescing.

Target:

```text
Rust events
 -> bounded queue
 -> batch per UI tick/frame
 -> one evaluate_script
```

Example JS boundary:

```js
window.__tiktools_receive_batch__([
  event1,
  event2,
  event3
]);
```

Coalesce disposable snapshots such as:

```text
room stats
leaderboard
analytics updates
processor metrics
```

Never use an unbounded queue.

## 11. Move blocking work off Tokio workers

Audit synchronous operations inside async RPC handlers:

```text
SQLite
filesystem
plugin scanning
plugin install/uninstall
archive extraction
remove_dir_all
large metadata operations
```

Use:

```rust
tokio::task::spawn_blocking(...)
```

where appropriate.

Do not block:

```text
Winit thread
Tokio event workers
live event pump
IPC reader
```

## 12. Make IPC startup failure visible

If desktop control IPC cannot start:

Do not only log:

```text
control IPC server exited
```

Either:

* fail desktop startup, or
* expose explicit degraded health and retry.

CLI/agents must never silently become unavailable while GUI appears healthy.

## 13. Improve tests

Add tests for:

```text
actual Vite port propagation
stale desktop detection
Windows pipe retry
one control-host owner
concurrent RPC calls
event subscription
live event delivery independent of automation slots
processors.status exact response shape
points.changed from live and manual mutations
WebView batching
```

Parity tests must validate behavior/result shape, not only method existence.

## Acceptance criteria

All must pass:

```bash
bun run lint
bun run typecheck
bun test
bun run build:web

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo check --workspace --locked
```

Manual dev test:

```bash
bun run scripts/start-dev.ts
```

Must show exactly one real Vite URL and desktop must use that exact URL.

Then verify:

```text
WebView RPC works
CLI talks to same AppCore
two IPC clients share state
live.connect works
live events reach WebView + IPC
automation saturation does not drop control events
desktop shutdown closes IPC cleanly
no stale processes are reused
```

Do not add feature-specific hacks. Fix the transport, event, ownership, and contract boundaries generically.
