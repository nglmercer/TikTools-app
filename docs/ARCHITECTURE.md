# Architecture

TikTools is a thin native host. Winit owns the window event loop, Wry owns the
WebView, and `tray-icon` owns the tray. The application core is independent of
all three.

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

The desktop builds one `AppCore`, wraps it in one `ControlApi`, and serves
every client from that instance: the WebView bridge, a persistent local IPC
endpoint (Unix socket / Windows named pipe), and through it the `tiktools`
CLI and agents. Standalone hosts (`host --stdio`, `host --ipc`,
`--standalone`) are explicit opt-ins for headless use.

```text
Vue WebView (control-client.ts + features/)
    │ window.ipc.postMessage(JSON-RPC)
    ▼
Wry IPC handler ── Tokio ── ControlApi ── AppCore
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
      LiveService   Points/SQLite   PluginManager
          │              │              │
          ▼              ▼        native/process/WASM
    tiktok-signer   host services
```

## Workspace boundaries

```text
tiktools-desktop
  ├── winit + wry + tray-icon
  ├── tiktools-control-api ── tiktools-core
  └── tiktools-core
        ├── tiktools-plugin-api
        ├── tiktools-plugin-sdk
        ├── tiktools-plugin-macros
        ├── tiktools-plugin-loader
        └── tiktools-tiktok
tiktools-cli ── tiktools-control-api ── tiktools-core
```

`tiktools-core` has no GUI dependency. Its services communicate outward with
`HostEmitter`, a small trait that carries serialized `HostMessage` values, and
with the `DomainEvent` bus every control client observes identically. The
desktop implementation sends host messages to the UI thread through
`EventLoopProxy`; only that thread calls `WebView::evaluate_script`.

Plugin calls converge at the SDK's typed request/result boundary. Core keeps
plugin lifecycle and polling in `plugin_runtime.rs`, capability-checked host
effects in `plugin_intents.rs`, and general automation orchestration in
`automation_runtime.rs`. The SDK compatibility decoder is the only place that
translates legacy `emit` and `playAudio` response keys.

## Desktop lifecycle

`crates/tiktools-desktop/src/app.rs` implements `ApplicationHandler`. It creates
a hidden Winit window and Wry WebView on the event-loop thread, dispatches
incoming IPC to Tokio, and shows the window after the mounted Vue app sends
`frontend-ready`. Close hides the window only while the tray is available; if
the tray cannot be created, close shuts down. The tray exposes Show and Quit.
Quit first asks `AppCore` to disconnect live transport, stop polling, and stop
plugins, then exits the event loop.

The WebView loads one of two sources:

- Development: `TIKTOOLS_DEV_URL` or `TIKTOOLS_FRONTEND_URL`.
- Release: `tiktools://app/index.html`, served from the executable-relative
  `web/` directory by Wry's custom protocol.

The asset handler canonicalizes paths and rejects traversal and symlink escapes.

## IPC contract

The contract is JSON-RPC through `ControlApi`: every client sends
`{"method": ...}` requests and receives result/error responses plus
`DomainEvent` notifications. The Vue bridge (`src/web/platform/control-client.ts`)
owns `window.ipc`; domain state lives in `src/web/features/`. Business
services never receive a WebView handle.

```text
Vue → JSON-RPC → ControlApi → AppCore operation/service
AppCore → DomainEvent → WebView / CLI / IPC / agents
```

`src/shared/messages.ts` remains the specification for host-push shapes.
Rust mirrors the legacy request side in
`crates/tiktools-core/src/ipc/messages.rs`, but the `IpcRouter`
`PageMessage` path is only an explicitly marked compatibility layer: each
legacy message adapts onto the same authoritative control operations, and
no new legacy variants are accepted.

## Event and live flow

`tiktools-tiktok` wraps the pinned Rust signer/discovery/WebSocket crates and
exports stable TikTools event values. It does not expose generated protobuf
objects to the core. The core:

1. resolves a creator through discovery, optionally bootstrapping an anonymous
   session;
2. creates the embedded signer backend and reconnecting WebSocket;
3. normalizes decoded chat, gift, like, member, social, and room-stat events;
4. updates points and SQLite;
5. runs the pre-filter processor pipeline, merging derived evidence under the
   reserved `intel` namespace (fail open: any processor failure passes the
   raw event through, stamped `degraded`);
6. emits the existing UI event shape and the enriched JSON automation event.

The event registry in `src/automation/contracts/generated/` is the editor-side
shape contract for those native events. Rust contract structs in
`crates/tiktools-core/src/contracts/` generate the JSON Schema, TypeScript
types, registry entries, and sample payloads. Its tests ensure every advertised
path exists in its sample payload; consumers must not recreate event
interfaces manually.

## Persistence

`rusqlite` owns the existing database names and table layout:

```text
data/tiktok-points.db       points, viewers, creators, app state, gifts
data/tiktok-automation.db   workflows, behavior records, plugin state
```

On startup, a checkout-local `data/` database is copied to the platform app-data
directory only when its destination is absent. Existing destination files are
never overwritten. JSON payloads remain JSON so existing workflow and behavior
records stay readable while their execution is moved into Rust.

## Automations

Rust owns workflow/behavior persistence, native event publication, built-in
action metadata, and bounded script execution. The editor remains Vue and
uses JSON descriptors supplied by the host. `napi-vm` receives only JSON values
(`event`, `inputs`, and `data`) and has no Node, filesystem, network, or WebView
objects.

The capability broker is explicit. HTTP, audio, TTS, points, storage, and
native integrations are host capabilities rather than framework permissions.

## Plugins

Plugin API and runtime are separate:

```text
versioned manifest/protocol/permissions
                 │
       ┌─────────┼─────────┐
       ▼         ▼         ▼
    native     process     WASM
   libloading  framed IO  optional
```

Plugin directories are scanned at runtime in built-in, user, and development
override order. No plugin id is compiled into TikTools. Native libraries expose
only a small serialized-message C ABI and stay loaded until shutdown. Process
plugins are standalone executables using length-prefixed JSON over stdio.

Plugins contribute actions (side effects), event sources (new triggers via
poll/emit), and processors (pre-filter enrichment of host events). Processors
declare `processorTypes`, require the `events.enrich` capability, answer
`enrich` calls with side-effect-free `{annotations, views, logs}` results,
and receive their host-rendered settings inside each request; the host merges
results under `event.intel` in deterministic order with strict deadlines and
circuit breaking. The textintel reference processor lives in
`examples/textintel-process-plugin/` with its own latency bench.

The processor pipeline keeps three boundaries. The contribution index
(`plugin_processors::index`) precomputes eligible processors off the hot
path and is rebuilt only on install, uninstall, enable, and disable. The
shared invoker (`plugin_invoker`) owns the single timed-call bridge used by
enrichment, actions, and the event poll. Per-processor health, metrics, and
settings caches (`plugin_processors::state`) keep one processor's failures
from affecting its siblings; stable projection into `event.intel`
(`plugin_processors::merge`) is a pure function of ordered outcomes.

## Platform code

Winit/Wry platform details are isolated under
`crates/tiktools-desktop/src/platform.rs`. Linux uses the GTK integration
required by WebKitGTK. The core, plugin API/loader, and TikTok crates do not
contain desktop conditionals.
