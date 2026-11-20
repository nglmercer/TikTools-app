---
title: "Current transports and platform restrictions"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
translationRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
order: 200
---

# Current transports and platform restrictions {#current-transports-and-platform-restrictions}

## Goal and prerequisites {#goal-and-prerequisites}

Select the transport that actually exists for the target build and platform. Read [Control API](control-api.md) and [collaboration](collaboration.md). A transport carries serialized operations; it must not introduce a second core or grant authority by itself.

## Choose the boundary {#choose-the-boundary}

| Path | Current implementation | Important limit |
| --- | --- | --- |
| Desktop WebView | Wry IPC and shared control client | GUI/thread lifetime belongs to desktop |
| Local CLI | Unix socket or Windows named pipe; explicit stdio host | Same application home and owner |
| Local browser | Native loopback adapter plus `browser-transport.ts` | Paired native host, not a mock or public remote server |
| Authenticated HTTPS/WSS gateway | SDK/browser transports with TLS, hello, grants and feature gates | Remote commands need an explicit grant |
| Installable remote-control plugin | `tiktools.remote-control` over a WSS relay using `remote.controller.ticket` | Plugin-owned; no remote code is compiled into the host |

The TypeScript transport interface lives at `src/web/shared/api/control-transport.ts`. Remote control is an installable plugin, not part of the host binary: every command crosses `HostCallPolicy` (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`). This build has no native peer/RTC transport and no Cloud Teams host link.

## Inspect the selected route {#inspect-the-selected-route}

Discover the connected host's supported methods and capabilities before pairing or displaying a controller action:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
```

Pairing and grants are owned by the plugin: a controller ticket is short-lived and single-use, and `remote.grant.list` / `remote.grant.revoke` are the authoritative view. A relay connection proves transport only, never RPC authority.

## Expected result and status {#expected-result-and-status}

Local IPC and desktop transport are established paths. Remote control works through the installable plugin and an authenticated relay; it is not a second host runtime. Login codes are relayed to connected devices with a five-minute TTL, with an OAuth fallback when no account is configured. Android local IPC works with the instrumented `mobile` tooling (`info` / `exec` / `tty`); desktop native addons and WebViews remain unavailable there.

## Recovery {#recovery}

Treat disconnect as possible uncertainty for accepted operations. Reconnect must invalidate private projections, renegotiate identity and resume or snapshot bounded streams. Never downgrade certificate checks or fall back silently to a local owner after remote failure. Expired controller tickets and revoked grants must fail closed.

## Operational details {#operational-details}

Keep grants least-privilege and time-bounded; re-check them after every reconnect. Diagnose identity, ticket expiry, grant scope, relay reachability and frame size separately. Large frames fail framing validation rather than becoming partial RPCs. Test changed/revoked sessions with fixtures before real two-device acceptance.
