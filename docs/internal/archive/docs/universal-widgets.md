# Universal widgets and the optional OBS bridge

The Overlays page opens a portable package panel from its toolbar via
`src/widgets/sdk/UniversalWidgetPanel.vue`. It consumes the exact shared SDK
vendored under `src/universal`; `vendor-lock.json` records its version and hashes.
`node scripts/check-universal.mjs` verifies the installed copy without another
repository. Existing Vue widgets and gateway clients keep their existing routes.

Import the compiled `universal.gift-alert/1.0.0` folder built by the OBS repository
to preview the same package in either host. Export/import portable project or
preset JSON to exchange settings and supported text/image/group layouts. The
existing Vue style editor also uses the shared bounded history implementation.
Unsupported schema assertions, dependencies and capabilities fail explicitly.
Existing Vue designs are retained; arbitrary component/HTML source cannot be
silently converted into the portable model. Timeline data is portable metadata;
no new animation engine is claimed.

The existing `plugins-example-tiktools/plugins/event-gateway` optionally forwards its
`live.event` fanout to the independent OBS receiver. Set `obsBridge.enabled`, a
numeric loopback `address` (default `127.0.0.1:17453`), and `tokenEnv` (default
`OBS_EVENT_TOKEN`) in its normal plugin settings. Obtain a paired OBS credential
using explicit local administration. For TikTok namespaces the OBS owner must
grant the exact types and `trustedTikTok` permission; a custom-event producer
cannot impersonate TikTok. Pass the token through the process environment, never
browser storage or a URL. Disable/restart the gateway to change credentials.

Forwarding uses protocol negotiation, canonical schema validation, original
room/message identities, normalized cumulative gift data, bounded queues,
exponential reconnect delay and origin/hop checks. Authenticated GET
`/obs-bridge/status` reports counters. The gateway's own port remains 17452.
Stopping this bridge or TikTools does not stop OBS or its other event producers.

Shared contracts, fixtures and Rust validation are in `src/universal`. Gateway
contract/security tests run with:

```text
cargo test --manifest-path plugins-example-tiktools/plugins/event-gateway/Cargo.toml --locked
bun scripts/check-universal.mjs
bun test src/widgets/shared/universal.test.ts src/web/entities/overlay/lib/editor-history.test.ts
```

See the OBS repository's `docs/architecture/current-state.md`,
`docs/architecture/universal-widget-adrs.md` and `docs/UNIVERSAL_WIDGETS.md` for the
verified discovery, security protocol, pairing utility and migration guide.
