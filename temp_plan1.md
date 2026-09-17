# TikTools Headless Control API Refactor

## Goal

Refactor TikTools so the WebView is only one client.

The app must be fully controllable without WebView through:

* CLI
* JSON over stdio
* local IPC
* tests
* AI agents

All clients must use the same `AppCore` operations.

## Architecture

```text
AppCore
  ↓
ControlApi
  ↓
ControlRouter
  ├─ CLI
  ├─ JSON stdio
  ├─ Unix socket / Windows named pipe
  └─ WebView
```

Do not duplicate business logic between CLI, IPC, and WebView.

## New modules

```text
crates/tiktools-control-api/
  src/
    lib.rs
    request.rs
    response.rs
    error.rs
    router.rs
    registry.rs

    modules/
      system.rs
      plugins.rs
      settings.rs
      live.rs
      points.rs
      automation.rs
      processors.rs
      media.rs
```

## Core API

Create:

```rust
pub struct ControlApi {
    core: Arc<AppCore>,
    router: ControlRouter,
}
```

Main entry point:

```rust
pub async fn execute(
    &self,
    request: RpcRequest,
) -> RpcResponse
```

Business logic must remain inside `AppCore` services.

`ControlApi` is only an adapter.

## Protocol

Use JSON-RPC-style messages.

Request:

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "plugins.list",
  "params": {}
}
```

Success:

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {}
}
```

Error:

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "error": {
    "code": "plugin_not_found",
    "message": "Plugin not found"
  }
}
```

Event:

```json
{
  "jsonrpc": "2.0",
  "method": "event",
  "params": {
    "topic": "plugin.progress",
    "data": {}
  }
}
```

## Typed methods

Do not implement all methods using raw `serde_json::Value`.

Each method should have typed params/results.

Example:

```rust
#[derive(Serialize, Deserialize)]
pub struct PluginSettingsGet {
    pub plugin_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct PluginSettingsResult {
    pub plugin_id: String,
    pub values: BTreeMap<String, Value>,
}
```

## Router

Avoid one giant `match`.

Use registration:

```rust
router.register(
    "plugins.settings.get",
    plugins::settings_get,
);

router.register(
    "plugins.settings.set",
    plugins::settings_set,
);
```

Each domain registers itself:

```rust
PluginsModule::register(&mut router);
PointsModule::register(&mut router);
AutomationModule::register(&mut router);
LiveModule::register(&mut router);
```

## Initial methods

```text
rpc.discover
rpc.schema

system.info
system.health
system.snapshot
system.doctor
system.shutdown

plugins.list
plugins.get
plugins.install
plugins.uninstall
plugins.enable
plugins.disable
plugins.start
plugins.stop

plugins.settings.get
plugins.settings.set
plugins.settings.reset

plugins.health
plugins.options
plugins.action.execute

processors.list
processors.status
processors.test

live.connect
live.disconnect
live.status

points.config.get
points.config.set
points.viewer.get
points.adjust
points.leaderboard

automation.list
automation.get
automation.create
automation.update
automation.delete
automation.enable
automation.disable
automation.test

media.validate
media.play
```

## Discovery API

Implement:

```text
rpc.discover
```

Return method metadata:

```json
{
  "methods": [
    {
      "name": "plugins.settings.set",
      "description": "Update plugin settings",
      "sideEffect": true,
      "paramsSchema": {},
      "resultSchema": {}
    }
  ]
}
```

This allows agents to discover the API dynamically.

## CLI

Create a thin wrapper around `ControlApi`.

Examples:

```bash
tiktools plugin list
tiktools plugin enable demo.plugin

tiktools plugin settings get demo.plugin

tiktools plugin settings set demo.plugin \
  serverUrl=http://localhost:8080

tiktools automation list
tiktools automation test workflow-id

tiktools live status

tiktools points leaderboard
```

All commands must support:

```bash
--json
```

JSON mode rules:

* stdout = JSON only
* stderr = diagnostics
* no ANSI
* no progress bars
* stable exit codes

## Raw RPC CLI

Support:

```bash
tiktools rpc plugins.list '{}'
```

And:

```bash
echo '{"id":1,"method":"plugins.list"}' \
  | tiktools rpc --stdio
```

Use NDJSON:

```text
one JSON request per line
one JSON response per line
```

## Headless mode

Implement:

```bash
tiktools host --stdio
```

This must create `AppCore` without:

* Winit
* Wry
* WebView
* tray

Use existing headless abstractions such as `NoopMediaHost`.

## Local IPC

After stdio works, add persistent local IPC.

Unix/macOS:

```text
Unix domain socket
```

Windows:

```text
named pipe
```

Use exactly the same JSON protocol as stdio.

Do not reuse the current single-instance `SHOW` protocol.

## Event architecture

Remove UI-specific concepts from the core event bus.

Do not use:

```rust
AppEvent::Ui(PageMessage)
```

Move toward domain events:

```rust
DomainEvent::PluginInstalled
DomainEvent::PluginProgress
DomainEvent::LiveConnected
DomainEvent::LiveEvent
DomainEvent::PointsChanged
DomainEvent::WorkflowChanged
DomainEvent::Shutdown
```

Consumers:

```text
DomainEvent
  ├─ WebView
  ├─ CLI watch
  ├─ local IPC subscriptions
  ├─ tests
  └─ agents
```

## Snapshot

Implement:

```text
system.snapshot
```

Return safe observable state:

```json
{
  "live": {},
  "plugins": [],
  "processors": [],
  "automations": [],
  "pointsConfig": {},
  "health": {}
}
```

Never expose secret settings.

## Doctor

Implement:

```text
system.doctor
```

Return structured diagnostics:

```json
{
  "ok": false,
  "checks": [
    {
      "id": "plugin.demo.connection",
      "status": "failed",
      "message": "Connection refused"
    }
  ]
}
```

## Security

Never expose internal objects through RPC:

* database handles
* plugin instances
* WebView objects
* filesystem handles
* native TikTok objects

Expose operations only.

Secrets must remain redacted.

Validate all input at the Control API boundary.

Add size limits and timeouts.

## Testing

Add headless integration tests using isolated:

```text
TIKTOOLS_HOME
```

Test flow:

```text
start headless host
→ plugins.list
→ configure plugin
→ health check
→ create automation
→ test automation
→ snapshot
→ shutdown
```

The same tests should work through stdio RPC.

## Migration order

1. Create `tiktools-control-api`.
2. Add typed request/response/router.
3. Implement `plugins.*`.
4. Implement `system.snapshot` and `system.doctor`.
5. Add CLI.
6. Add `host --stdio`.
7. Add automation/points/live methods.
8. Add local IPC.
9. Migrate WebView to Control API.
10. Remove legacy `PageMessage`/`HostMessage` where no longer needed.

## Rule

There must be one authoritative operation path:

```text
client
→ ControlApi
→ AppCore service
```

Never:

```text
CLI logic
WebView logic
Agent logic
```

with separate implementations.
