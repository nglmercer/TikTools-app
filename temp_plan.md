# TikTools Control Plane Completion

## Goal

Finish the headless/control-plane refactor so TikTools has exactly one authoritative runtime state and every client uses the same API.

Target architecture:

```text
                ONE AppCore
                    │
               ControlApi
                    │
          local JSON-RPC IPC
        ┌───────────┼───────────┐
        │           │           │
      CLI         WebView      agents
```

Do not duplicate business logic.

## Current State

Already implemented:

```text
crates/tiktools-control-api
crates/tiktools-cli

ControlApi
typed ControlRouter
rpc.discover
rpc.schema

NDJSON stdio
Unix socket
Windows named pipe

plugins.*
plugins.settings.*
processors.*
live.*
points.*
automation.*
media.*
system.info
system.health
system.snapshot
system.doctor
system.shutdown

DomainEvent
WebView JSON-RPC compatibility
```

Still incomplete:

```text
CLI creates its own AppCore
desktop does not expose persistent control IPC
Vue still uses legacy PageMessage/HostMessage
AppEvent::Ui(PageMessage) remains
RPC does not have full WebView feature parity
legacy IPC remains authoritative in many places
transport test may hang
new crates need formatting/verification
```

---

# 1. Desktop Must Own Control IPC

Modify desktop startup so its existing:

```rust
Arc<AppCore>
```

is shared by:

```text
WebView
ControlApi
local IPC server
```

Do NOT create another AppCore for control IPC.

Expected:

```rust
let core = Arc::new(AppCore::with_media_host(...));

let control =
    Arc::new(ControlApi::new(core.clone()));

start_control_ipc(control.clone());
```

The IPC listener must run on Tokio without blocking Winit.

Shutdown must stop:

```text
live connection
plugin polling
plugins
control IPC
desktop
```

Files likely involved:

```text
crates/tiktools-desktop/src/app.rs
crates/tiktools-desktop/src/window.rs
crates/tiktools-control-api/src/transport.rs
```

---

# 2. CLI Must Connect to Existing Runtime

Normal CLI commands must NOT create:

```rust
AppCore::new(...)
```

by default.

Current wrong model:

```text
tiktools plugin list
  -> new AppCore
```

Required:

```text
tiktools plugin list
  -> connect local IPC
  -> running AppCore
```

Implement client transport:

```text
crates/tiktools-control-api/src/client.rs
```

Suggested API:

```rust
pub struct ControlClient;

impl ControlClient {
    pub async fn connect() -> Result<Self, ClientError>;

    pub async fn call<P, R>(
        &mut self,
        method: &str,
        params: P,
    ) -> Result<R, ClientError>;
}
```

Unix:

```text
$TIKTOOLS_HOME/tiktools-control.sock
```

Windows:

```text
\\.\pipe\tiktools-control
```

CLI behavior:

```text
tiktools plugin list
    -> connect IPC

tiktools rpc ...
    -> connect IPC

tiktools --headless ...
    -> optional standalone AppCore

tiktools host --stdio
    -> standalone host

tiktools host --ipc
    -> standalone host only when desktop is not running
```

Do not silently start another AppCore when an IPC connection fails.

Return a clear error:

```json
{
  "code": "host_unavailable",
  "message": "TikTools control host is not running."
}
```

---

# 3. Full RPC Feature Parity

Every operation currently available through `PageMessage` must have a Control API equivalent.

Add missing domains.

## App state

```text
app.state.get
app.state.set
```

## Creator state

```text
creators.get
creators.recent
creators.history.clear
```

## Analytics

```text
analytics.summary
```

## Gifts

```text
gifts.list
gifts.debug
```

## Workflow graph/editor

Existing behavior automation API is not enough.

Add:

```text
workflows.list
workflows.get
workflows.save
workflows.delete
workflows.enable
workflows.disable

automation.nodes.list
automation.script.analyze
automation.context
```

## Plugin token provisioning

```text
plugins.token.provision
```

Credentials:

* never persist password
* never echo password
* never include password in logs/events/errors

## Media picker

Keep headless-safe:

```text
media.validate
media.play
```

Desktop-only capability:

```text
media.pick
```

If unavailable:

```text
capability_unavailable
```

Do not force headless environments to emulate a file dialog.

---

# 4. Move Legacy WebView Handlers onto AppCore Control Operations

`handle_page_message()` must stop implementing business logic independently.

Legacy compatibility is acceptable temporarily, but each legacy message should become an adapter.

Bad:

```rust
PageMessage::AdjustPoints => {
    // business logic here
}
```

Required:

```rust
PageMessage::AdjustPoints { ... } => {
    match self.points_adjust(...) {
        ...
    }
}
```

Do this for every legacy operation.

The authoritative operation implementation must live in:

```text
AppCore control operations
```

not:

```text
PageMessage handler
CLI
WebView
```

---

# 5. Vue Must Use JSON-RPC

Create frontend bridge:

```text
src/web/platform/control-client.ts
```

Responsibilities:

```text
window.ipc
JSON serialization
request ids
pending promises
timeouts
rpc-response
event notifications
transport errors
```

API:

```ts
const control = createControlClient();

await control.call(
  "plugins.settings.get",
  { pluginId }
);

control.on("plugin.progress", handler);
```

No Vue component/composable should directly call:

```ts
window.ipc.postMessage(...)
```

outside this bridge.

---

# 6. Split useAppController

Reduce `useAppController.ts`.

Move domains into:

```text
src/web/features/
  connection/
  points/
  plugins/
  processors/
  automation/
  media/
  analytics/
  creators/
  tts/
```

Each feature should expose its own API/store.

Example:

```ts
usePlugins()
usePoints()
useLive()
useAutomation()
```

Do not create another giant global message switch.

---

# 7. Migrate Host Events

Keep:

```rust
DomainEvent
```

Expand it as needed:

```text
plugin.installed
plugin.uninstalled
plugin.started
plugin.stopped
plugin.progress
plugin.settings-changed

live.connected
live.disconnected
live.event

points.changed

workflow.changed

creator.changed
analytics.updated

shutdown
```

WebView, CLI streaming, local IPC, tests and agents should consume the same events.

---

# 8. Remove UI Concept from Core Event Bus

Remove:

```rust
AppEvent::Ui(PageMessage)
```

Core event buses must contain domain events only.

If legacy WebView message observation is still required during migration, keep it outside the domain event bus.

Target:

```rust
pub enum DomainEvent {
    ...
}
```

Then remove the old `AppEvent` when no longer needed.

---

# 9. Remove Legacy IPC After Parity

Once Vue uses ControlApi and all operations have parity:

remove or deprecate:

```text
PageMessage
HostMessage request-response patterns
IpcRouter legacy routing
AppEvent::Ui
large useAppController receive switch
```

Keep only genuinely useful push/event messages if needed.

Target WebView path:

```text
Vue
 -> JSON-RPC
 -> ControlApi
 -> AppCore

AppCore
 -> DomainEvent
 -> WebView
```

---

# 10. Typed Contracts Everywhere

Do not add raw unvalidated RPC methods.

Every method needs typed:

```rust
Params
Result
```

and:

```rust
Serialize
Deserialize
JsonSchema
```

Example:

```rust
#[derive(
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct AnalyticsSummaryParams {
    pub creator_unique_id: Option<String>,
    pub start_day: Option<i64>,
    pub end_day: Option<i64>,
    pub limit: Option<i64>,
}
```

Register with:

```rust
router.register_typed::<Params, Result, _, _>(
    "...",
    "...",
    false,
    handler,
);
```

---

# 11. Agent-Friendly Discovery

`rpc.discover` must list all methods.

Metadata must contain:

```json
{
  "name": "plugins.settings.set",
  "description": "...",
  "sideEffect": true,
  "paramsSchema": {},
  "resultSchema": {}
}
```

Add optional metadata if useful:

```json
{
  "destructive": false,
  "requiresDesktop": false
}
```

Agents must be able to understand the API without reading source code.

---

# 12. Stable Errors

Use machine-readable codes.

Examples:

```text
invalid_params
method_not_found
plugin_not_found
automation_not_found
host_unavailable
capability_unavailable
conflict
timeout
unavailable
internal
request_too_large
```

Do not make agents parse human strings.

---

# 13. Security

Never expose:

```text
database handles
plugin runtime objects
WebView handles
native file handles
native TikTok objects
raw secret values
passwords
tokens
```

Maintain:

```text
secret redaction
settings placeholder preservation
bounded requests
bounded params
timeouts
closed DB table allowlists
plugin capability checks
```

IPC endpoint must be per-user only.

On Unix ensure socket permissions are restricted.

On Windows use a user-scoped named pipe security policy where possible.

---

# 14. Fix Transport Tests

Current transport tests must terminate reliably.

Test:

```text
request
response
domain event
EOF
shutdown
server task exit
```

Avoid hanging on:

```text
duplex split halves
event receiver waiting
shutdown race
writer remaining open
```

Use explicit timeout assertions:

```rust
tokio::time::timeout(...)
```

No test may wait forever.

---

# 15. Add IPC Client Integration Test

Test real host/client behavior.

Flow:

```text
start IPC server
connect ControlClient
rpc.discover
plugins.list
points.adjust
system.snapshot
system.shutdown
server exits
```

Use isolated:

```text
TIKTOOLS_HOME
```

---

# 16. Add WebView Parity Tests

Build a parity test list from all legacy `PageMessage` variants.

Every important operation must map to a Control API method.

Fail the test when a legacy operation has no Control API equivalent.

This prevents future drift.

---

# 17. One Runtime Ownership Test

Add a test proving CLI client and WebView/control API see the same state.

Example:

```text
running host:
  points.adjust alice +10

client 1:
  points.viewer.get alice
  => 10

client 2:
  points.viewer.get alice
  => 10
```

There must not be two AppCore instances.

---

# 18. CLI Modes

Normal:

```bash
tiktools plugin list
```

uses running IPC host.

Machine:

```bash
tiktools --json plugin list
```

stdout JSON only.

Raw:

```bash
tiktools rpc plugins.list '{}'
```

uses running IPC host.

Headless standalone:

```bash
tiktools host --stdio
tiktools host --ipc
```

Optional isolated execution can be explicit:

```bash
tiktools --standalone ...
```

Never make standalone the default for normal commands.

---

# 19. Documentation

Update:

```text
docs/CONTROL_API.md
docs/ARCHITECTURE.md
docs/DEVELOPMENT.md
README.md
```

Clearly document:

```text
one AppCore
ControlApi
IPC host
CLI client
WebView client
DomainEvent
legacy migration status
```

Remove documentation that incorrectly implies migration is complete before it actually is.

---

# 20. Verification

Run and fix everything:

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
bun run test
bun run build:web

git diff --check
```

No hanging tests.

No formatting errors.

No warnings.

---

# Definition of Done

The refactor is complete only when:

```text
[ ] Desktop owns one AppCore
[ ] Desktop starts ControlApi local IPC
[ ] CLI connects to running host by default
[ ] CLI does not create separate AppCore by default
[ ] stdio headless host works
[ ] local IPC works on Unix
[ ] local IPC works on Windows
[ ] full PageMessage feature parity exists in ControlApi
[ ] Vue uses JSON-RPC ControlApi
[ ] frontend direct window.ipc calls are centralized
[ ] DomainEvent is authoritative
[ ] AppEvent::Ui(PageMessage) removed
[ ] legacy IpcRouter removed or limited to explicit compatibility layer
[ ] PageMessage no longer contains authoritative business logic
[ ] snapshot is secret-safe
[ ] doctor is structured
[ ] rpc.discover exposes complete method schemas
[ ] transport tests terminate
[ ] IPC integration tests pass
[ ] WebView parity tests pass
[ ] workspace fmt/check/clippy/tests pass
[ ] Bun lint/typecheck/tests/build pass
```

## Core Rule

There must be exactly one operation path:

```text
client
  ↓
ControlApi
  ↓
AppCore operation/service
```

Never maintain separate implementations for:

```text
CLI
WebView
agent
stdio
IPC
```
