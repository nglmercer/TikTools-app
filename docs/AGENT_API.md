# TikTools Agent API

One control surface for humans, scripts, and AI agents, with a typed Rust
SDK underneath and a CLI on top:

```text
        ┌───────────┐
        │  WebView  │
        └─────┬─────┘
              │
        ┌─────▼─────┐
        │ Control   │
        │ API       │
        └─────┬─────┘
              │  (ControlApi owns AppCore)
   ┌──────────┴──────────┐
   │                     │
┌──▼──────────────┐ ┌────▼──────────┐
│ TikToolsClient  │ │ direct host   │
│ SDK             │ │ tests         │
└──┬──────────────┘ └───────────────┘
   │
   ├──── CLI
   ├──── agents
   └──── integration tests
```

The invariant is:

```text
Control API registry = Typed client coverage = Agent-callable control surface
```

It is enforced, not documented: `tests/parity.rs` in `tiktools-client`
fails CI when a registered method lacks a typed wrapper (or vice versa),
and `tiktools api verify` checks the same invariant live against a
running host, catching CLI/host version skew at deploy time.

Routing rules:

- `TikToolsClient` is the only caller of `ControlApi` from the
  outside; `ControlApi` remains the only caller of `AppCore` methods.
- The CLI never constructs `AppCore` or calls `ControlApi`/`AppCore`
  methods on command paths. The only exceptions are standalone execution
  and headless host startup, which build `ControlApi` via the existing
  constructor.
- The typed client crate (`crates/tiktools-client`) depends only on
  `tiktools-control-api` plus `serde`/`tokio`: no desktop, no direct
  core dependency. Param/result types are re-exported through
  `tiktools_control_api::modules`.

## Typed Rust SDK (`tiktools-client`)

```rust
use tiktools_client::TikToolsClient;
use tiktools_control_api::modules::plugins::PluginIdParams;

// Against the running host over local IPC.
let client = TikToolsClient::connect().await?;

// Or in-process (hosts, tests): TikToolsClient::direct(api).

let plugins = client.plugins_list().await?;
let one = client
    .plugins_get(PluginIdParams { plugin_id: "demo".into() })
    .await?;

// One stream type on both backends.
let mut events = client.subscribe();
while let Ok(event) = events.recv().await {
    println!("{event:?}");
}
```

- 68 methods, one module per domain (`automation_list`,
  `plugins_action_execute`, `rpc_discover`, ...). Methods whose params
  are `Empty` take no argument.
- `call` / `call_value` are the generic escape hatches for runtime-chosen
  methods; `validate_params` dry-validates params against the registry's
  own typed shapes with host-exact errors.
- `ControlEvent::Domain` carries authoritative events;
  `ControlEvent::Gap { lost }` means the stream skipped reliable events
  and the consumer must resync instead of reconstructing them.

## CLI: `tiktools api`

Domain commands (`plugin`, `automation`, `points`, `live`, `media`,
`system`, `workflow`, `processor`) resolve to one typed client call each;
`api` is the generic surface reaching 100% of the registry.

```bash
tiktools api discover                      # every method, live from the host
tiktools api schema plugins.settings.set   # description, flags, JSON schemas
tiktools api call plugins.list
tiktools api call points.adjust '{"uniqueId":"amy","delta":5}'
tiktools api events --topics 'live.*,points.changed' --max-events 10
tiktools api verify --sandbox              # coverage + smoke, isolated
```

Global flags: `--json` (stdout carries JSON only), `--standalone`
(isolated in-process runtime instead of IPC). Exit codes are stable:
0 ok, 1 operation error, 2 usage, 3 transport failure. Usage errors are
validated before any connection attempt, so they never depend on host
state.

Agent-safe `call` flags:

- `--dry-run` resolves the method and validates params without executing.
- Destructive methods (delete/reset/uninstall/shutdown) refuse to run
  without `--confirm` (`confirmation_required`, exit 1).
- `--timeout SEC` bounds the call (default is the 150 s host budget).
- `--explain` prints the call plan (method, description, flags, redacted
  params) to stderr; stdout keeps the pure result.
- `--format json|human` overrides `--json` for one command.

`api events` prints one `{topic, data}` NDJSON object per line until
`--max-events`, `--timeout`, or Ctrl-C (exit 0). Gaps print as
`{"topic":"event.gap","data":{"lost":N,"resync":true}}`.
Topic filters accept exact names, `*`, and `prefix.*` (which covers the
bare `prefix` too); `--topics` is repeatable and comma-separated.

## Verification

```bash
tiktools api verify --sandbox   # safe anywhere: throwaway runtime
tiktools api verify              # against the running host (or --standalone)
tiktools api verify --coverage   # registry-vs-SDK check only
tiktools api verify --smoke      # 21 read-only calls only
```

`--sandbox` forces an isolated `TIKTOOLS_HOME` that is removed
afterwards, so CI and agents verify without a host and without touching
real data. Reports print human-readable by default and machine-readable
with `--json`; any failure exits 1.

## Secrets

- `api call` output is secret-redacted by default (session cookies,
  passwords, tokens, API keys, credentials, ...); only
  `--secrets-visible` opts out, and `--redact` documents the default
  explicitly. `--explain` always redacts.
- `live connect` / `live pick` read the session cookie from
  `--session-cookie` or `TIKTOOLS_SESSION_COOKIE` (the flag wins). The
  value never appears in usage text, errors, or explanations.
- Plugin settings stay on the host-safe endpoints: `plugins.settings.get`
  returns schema plus redacted values, and saving the placeholder back
  preserves stored secrets. Never raw SQL or debug stores.

## Host-safe endpoints

Verification and agents use the control surface exclusively. The smoke
list is read-only by construction; mutating-method coverage belongs to
the host test suite, never to a CLI pointed at live data.
