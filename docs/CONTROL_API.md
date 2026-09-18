# TikTools Control API

One authoritative operation path for every client:

```text
client -> ControlApi -> AppCore service
```

Clients: WebView, CLI (`tiktools`), JSON stdio (`tiktools host --stdio`),
local IPC (desktop host or `tiktools host --ipc`), integration tests, AI
agents.

There is exactly one runtime: the desktop builds one `AppCore`, wraps it in
one `ControlApi`, and serves every client (WebView bridge, local IPC, CLI)
from that same instance. Standalone hosts (`host --stdio`, `host --ipc`,
`--standalone`) are explicit opt-ins for headless use.

## Protocol

JSON-RPC-style NDJSON: one request per line, one response per line.

Request:

```json
{"jsonrpc": "2.0", "id": "1", "method": "plugins.list", "params": {}}
```

Success:

```json
{"jsonrpc": "2.0", "id": "1", "result": {}}
```

Error:

```json
{"jsonrpc": "2.0", "id": "1", "error": {"code": "plugin_not_found", "message": "Plugin `x` is not installed."}}
```

Event notification (interleaved when streaming is enabled):

```json
{"jsonrpc": "2.0", "method": "event", "params": {"topic": "plugin.progress", "data": {}}}
```

Limits: 1 MiB per request line, 256 KiB params, 150 s per request.

## Discovery

```bash
tiktools rpc rpc.discover '{}'
tiktools rpc rpc.schema '{"method": "plugins.settings.set"}'
```

`rpc.discover` returns every method with its description, `sideEffect`
flag, `destructive` and `requiresDesktop` risk flags, and JSON Schemas for
params/result, so agents can work without hardcoded knowledge.

## CLI

```bash
tiktools plugin list
tiktools plugin settings set demo.plugin serverUrl=http://localhost:8080
tiktools automation list
tiktools live status
tiktools points leaderboard
tiktools rpc plugins.list '{}'
echo '{"id":1,"method":"plugins.list"}' | tiktools rpc --stdio
```

All commands accept `--json` (stdout = JSON only, stderr = diagnostics,
exit 0 ok / 1 operation error / 2 usage / 3 transport failure).

Commands run against the running host over local IPC by default and fail
with `host_unavailable` when no host is running; they never silently start
a second runtime. `--standalone` opts one command into an isolated
in-process runtime instead. `tiktools workflow list` manages graph
workflows; every other method is reachable through `tiktools rpc`.

## Headless host

```bash
tiktools host --stdio [--events]   # AppCore without Winit/Wry/tray
tiktools host --ipc                # Unix socket / Windows named pipe
```

The IPC endpoint is `$TIKTOOLS_HOME/tiktools-control.sock` on Unix and
the per-user `tiktools-control-<user-sid>` named pipe on Windows, speaking
the same NDJSON protocol as stdio. The desktop host serves this endpoint
itself from its one `AppCore`; `host --ipc` refuses to start when a host is
already listening. The Unix socket is created owner-only (`0600`); the
Windows pipe carries a current-user-only DACL. Integration tests isolate
Windows pipes with `TIKTOOLS_IPC_NAME`.

## Methods

`rpc.discover`, `rpc.schema`, `system.info|health|snapshot|doctor|shutdown|ping`,
`plugins.list|get|install|uninstall|enable|disable|start|stop`,
`plugins.install.set`, `plugins.settings.get|set|reset`,
`plugins.health|options|action.execute`, `plugins.token.provision`,
`processors.list|status|test`, `live.connect|pick|disconnect|status`,
`points.config.get|set`, `points.viewer.get`, `points.adjust`,
`points.leaderboard|reset`, `app.state.get|set`, `creators.get|recent`,
`creators.history.clear`, `analytics.summary`, `gifts.list|debug`,
`workflows.list|get|save|delete|enable|disable`,
`automation.list|get|create|update|delete|enable|disable|test|context|runs|snapshot`,
`automation.nodes.list`, `automation.script.analyze`,
`media.validate|play|pick`.

The Vue frontend speaks JSON-RPC through `src/web/platform/control-client.ts`
(one bridge owns `window.ipc`, request ids, timeouts, `rpc-response`
routing, and `event` dispatch) with domain state in `src/web/features/`.
WebView JSON-RPC (`{"method": ...}`) is answered as
`{"type": "rpc-response", ...}`, and domain events stream to the WebView as
`event` notifications on the same channel. The legacy `{"type": ...}`
`PageMessage` path remains only as an explicitly marked compatibility
layer; every legacy message is an adapter over the same control
operations, and no new legacy variants are accepted. The
`tests/parity.rs` suite fails if any legacy operation lacks a Control API
equivalent.

## Safety rules

- Operations only: no database handles, plugin instances, WebView
  objects, filesystem handles, or native TikTok objects cross RPC.
- Secrets stay redacted; settings placeholders preserve stored values.
- Behavior-record tables are a closed allowlist (`behavior_events`,
  `behavior_actions`); table names never come from caller input.
- `plugins.action.execute` defaults to a dry run (`live: true` executes).
