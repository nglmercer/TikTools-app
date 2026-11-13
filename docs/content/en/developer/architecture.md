---
title: "Application and service architecture"
docId: "developer.architecture"
translationKey: "developer.architecture"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "0387b43e15f5d1b8f166f3951620d66b5ae54c3e9d6f2ddf6030605703a541ba"
translationRevision: "0387b43e15f5d1b8f166f3951620d66b5ae54c3e9d6f2ddf6030605703a541ba"
order: 30
---

# Application and service architecture

## Goal and prerequisites

Place a change in the service that owns its state. Read [setup](setup.md) and understand the difference between an existing host and an explicit standalone host. The desktop constructs one `AppCore`, wraps it in one `ControlApi` and exposes that instance to the WebView, local CLI and agents.

## Trace an operation

1. Find the method's registration in `crates/tiktools-control-api/src/modules/`. Registration supplies schemas, policy and discovery metadata.
2. Follow the handler into `crates/tiktools-core/src/control/` and the owning service. Put domain validation and persistent mutations here rather than in a transport adapter.
3. Add the typed wrapper in `crates/tiktools-client`; its parity test compares SDK coverage with the registered surface.
4. Consume the method through `src/web/shared/api/control-client.ts`. Vue entities hold presentation state; the desktop does not hand the WebView a database or native integration object.

Winit owns the event loop, Wry the WebView and the tray integration the tray. `HostEmitter` and domain events carry serialized values out of core. The desktop forwards work through its event-loop proxy; WebView script evaluation stays on that thread. Tokio runs asynchronous host operations.

## Concrete boundary check

```sh
bun scripts/task.ts test web control-client
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
```

The first command checks the frontend client. The second runs the SDK integration target directly while preserving the lockfile; Cargo target flags belong on the `rust` passthrough rather than the narrower targeted-test grammar.

## Expected result and status

All clients observe the same authoritative operation and event path. `host --ipc`, `host --stdio` and `--standalone` intentionally create headless ownership; normal CLI commands attach to a running owner. The legacy `PageMessage`/`IpcRouter` path remains compatibility code and should not gain new message variants.

## Recovery

If two clients disagree, compare host identity, home directory, build features and discovery before adding another store. A missing plugin is a runtime catalog issue, not a reason to compile its implementation into core. Historical architecture documents contain retired frontend paths; follow the [current core](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/lib.rs) and [frontend map](frontend.md) when they differ.
