---
title: "Current transports and platform restrictions"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "293f8c00709f3796e5d6bab335df654aef5d9d379c19d622934bf1f8a95591c0"
translationRevision: "293f8c00709f3796e5d6bab335df654aef5d9d379c19d622934bf1f8a95591c0"
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
| Direct remote | Authenticated HTTPS/WSS gateway and SDK/browser transports | TLS, hello, grants and feature gates |
| Native peer | `tiktools-peer`, encrypted iroh/QUIC stream | Peer identity is not RPC authority |
| Native WebRTC | `tiktools-rtc`, signed signaling and data channels | Negotiation, grant and route checks remain required |

The TypeScript transport interface lives at `src/web/shared/api/control-transport.ts`. Native peer configuration does not implicitly enable public third-party relays or discovery. WebRTC is Rust-native (`webrtc` 0.21.1), not a dependency on browser `RTCPeerConnection` in the desktop UI.

## Inspect the selected route {#inspect-the-selected-route}

Discover the connected host's supported methods and capabilities before pairing or displaying a controller action:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` observes the selected ICE candidate pair to classify direct versus relay. Ready state alone cannot prove a direct route. Cloud signaling/relay plumbing is separate from business authorization and must retain native grant checks.

## Expected result and status {#expected-result-and-status}

Local IPC and desktop transport are established paths. Native peer, WebRTC and controller integration have source and focused tests, but deployment and multi-PC acceptance remain experimental. Legacy text claiming there is no peer/relay implementation is historical. Android local IPC works; persistent remote profiles have no selected credential store, and desktop native addons/WebViews remain unavailable there.

## Recovery {#recovery}

Treat disconnect as possible uncertainty for accepted operations. Reconnect must invalidate private projections, renegotiate identity and resume or snapshot bounded streams. Never downgrade certificate checks or fall back silently to a local owner after remote failure. The [RTC implementation](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src) and actual feature graph supersede older transport plans.
