# TikTools Control API

One authoritative operation path for every client:

```text
client -> ControlApi -> AppCore service
```

Clients: WebView, CLI (`tiktools`), JSON stdio (`tiktools host --stdio`),
local IPC (`tiktools host --ipc`), integration tests, AI agents.

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
flag, and JSON Schemas for params/result, so agents can work without
hardcoded knowledge.

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

## Headless host

```bash
tiktools host --stdio [--events]   # AppCore without Winit/Wry/tray
tiktools host --ipc                # Unix socket / Windows named pipe
```

The IPC endpoint is `$TIKTOOLS_HOME/tiktools-control.sock` on Unix and
the `tiktools-control` named pipe on Windows, speaking the same NDJSON
protocol as stdio.

## Methods

`rpc.discover`, `rpc.schema`, `system.info|health|snapshot|doctor|shutdown`,
`plugins.list|get|install|uninstall|enable|disable|start|stop`,
`plugins.settings.get|set|reset`, `plugins.health|options|action.execute`,
`processors.list|status|test`, `live.connect|pick|disconnect|status`,
`points.config.get|set`, `points.viewer.get`, `points.adjust`,
`points.leaderboard`, `automation.list|get|create|update|delete|enable|disable|test|context`,
`media.validate|play`.

`live.pick` and `automation.context` exist for WebView parity; the
frontend still speaks the legacy `{"type": ...}` IPC, which the Rust host
now funnels through the same control operations before emitting UI
messages. WebView JSON-RPC (`{"method": ...}`) is accepted on the same
bridge and answered as `{"type": "rpc-response", ...}`.

## Safety rules

- Operations only: no database handles, plugin instances, WebView
  objects, filesystem handles, or native TikTok objects cross RPC.
- Secrets stay redacted; settings placeholders preserve stored values.
- Behavior-record tables are a closed allowlist (`behavior_events`,
  `behavior_actions`); table names never come from caller input.
- `plugins.action.execute` defaults to a dry run (`live: true` executes).
