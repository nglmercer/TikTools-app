# Live platform adapters

TikTools has a provider-independent live boundary and two built-in providers:
TikTok (`native-tiktok`) and Twitch (`http`). YouTube, Facebook and Kick have
reserved platform IDs but are not advertised as installed adapters. Their
protocols and permissions still require provider implementations.

## Ownership

```text
tiktools-live             contracts, capabilities, registry and lifecycle
tiktools-tiktok/adapter   wrapper around the existing native TikTok client
tiktools-twitch           authenticated read-only Twitch IRC over TLS WebSocket
tiktools-core/live_platforms
                         publication, replay guard, points and analytics
```

An adapter implements `descriptor`, `normalize_target` and `run`. Only
application-registered adapters receive an `EventSink`. The sink stamps platform,
connection and resolved channel identity; RPC callers cannot inject events or
claim that they verified a channel. Add a provider crate and register it in
`PlatformRuntime::new`. Do not put provider switches in points or widgets.

The manager owns cancellation and tasks, bounds connections to 32, and uses a
bounded event queue. Disconnect targets one connection. Shutdown cancels producers,
drains admitted work and joins the consumer before the points writer shuts down.
Terminal snapshots remain available until explicitly disconnected. Providers
classify errors and implement their own retry budgets and rate-limit delays.

## Contracts and compatibility

`LiveEnvelope` version 1 includes platform, connection, channel, session scope,
string source event ID, provider/receive timestamps, history, viewer identity,
typed payload and provider extensions. Provider IDs always stay strings.
Session scope identifies the room; Twitch IRC does not provide a broadcast
session ID or prove that a channel is currently broadcasting.

`live.normalized` carries `{ event: LiveEnvelope }`. Automation and widgets use
the existing `live.event` transport with shared `live.chat`, `live.gift`,
`live.subscription`, `live.follow`, `live.like`, `live.share`, `live.join`,
`live.room_stats`, `live.connected` and `live.disconnected` event types.
Providers publish only events they can observe. Lifecycle snapshots use
`live.platform.connection`; ingestion loss uses `live.platform.gap`.

TikTok keeps its generation fences, ingress priorities, streak totals, replay
guard, durable effects, points, rankings, competitions and `tiktok.*` contracts.
Its native pipeline publishes normalized projections after replay admission.
The adapter's parallel stream never runs TikTok side effects again. One-way
trigger aliases let a shared rule match a TikTok event; a `tiktok.*` rule never
matches Twitch. Each matching rule executes once and sees its selected type.
Two separate rules referencing the same action remain two intentional executions.
Legacy payloads retain their fields and add common fields where needed.

An explicit catalog replaces prefix checks in source classification. Execution
authority remains private host state. Twitch runs local Owner rules without
delegation; creator-scoped remote grants and native ranking/competition
accounting remain TikTok-specific. New provider events do not enter those scopes.
The new RPC methods default to denied for remote callers.

`live.*` is now a reserved host event namespace. Plugins can subscribe to it,
but their own declared event types must use a plugin-owned namespace.

Rust owns the schemas. Regenerate the TypeScript contracts and event registry
with `bun scripts/generate-contracts.ts`; verify with `bun run check`.

## Identity, persistence and units

New viewers use provider-qualified stable IDs such as `twitch:2:42`.
`platform_viewers` maps `(platform, user_id)` to the points key and tracks the
handle/display name. Identical handles or provider IDs on different platforms
cannot share a balance. Existing TikTok balances and transaction keys remain
unchanged. An additive, idempotent migration labels existing viewers and creator
history as TikTok; ambiguous legacy handles are not merged.

`platform_channels` stores provider-qualified channels. `platform_analytics`
records daily counts by platform, channel and event type, with bounded queries.
Replay identity includes platform, channel, session, type and source ID. The
guard is bounded to ten minutes/50,000 events and survives transport reconnects
within one host process. It does not guarantee exactly-once processing across
host restarts. Historical events are presentation-only: no automation, points
or analytics effects.

Chat, follows, likes, shares and joins use the existing points configuration.
Monetary events retain a decimal amount string and an explicit unit. Bits never
use TikTok's per-diamond point conversion. To award points for Bits, create a
`live.gift` rule filtered by `event.platform` and `event.data.unit` with an
explicit points action. Gift widgets preserve the amount/unit.

## Twitch setup

Obtain a Twitch **user access token** with `chat:read` through a supported OAuth
flow. App access tokens are not accepted. The adapter validates tokens before
joining and at least hourly while active. Expired/revoked tokens require a new
token; the app does not embed a client secret or refresh externally issued tokens.

Open Connections → Live platforms, enter a channel login and token, and connect.
The form clears the token immediately. The UI creates an opaque credential
reference for startup and deletes it once the adapter owns a private copy.
Tokens never enter browser storage. Multiple Twitch channels and TikTok can run
together.

Capabilities: chat, subscription notices and Bits. Follows, likes, viewer counts,
sending chat and moderation are unavailable. The adapter requests IRC tags and
commands, verifies channel identity using `ROOMSTATE`, answers PING, handles
server reconnect requests, retries transient failures up to five times and
honors validation `Retry-After`. Bits messages produce separate chat and gift
events sharing a source ID, with distinct event types.

The credential API uses private atomic files under `data/platform-credentials`
(0600 on Unix; inherited per-user directory ACLs on Windows). References are
provider-bound random IDs. Tokens never appear in snapshots or error messages.
Headless callers may retain a reference until explicitly deleting it.

```sh
tiktools live platforms
tiktools live credential-store --platform twitch  # reads TIKTOOLS_PLATFORM_TOKEN
tiktools live platform-connect --platform twitch --target channel --credential-ref <reference>
tiktools live connections
tiktools live platform-analytics
tiktools live platform-disconnect --connection-id twitch-1
tiktools live credential-delete --platform twitch --credential-ref <reference>
```

Typed SDK and JSON-RPC methods: `live.platforms`, `live.connections`,
`live.platform.connect`, `live.platform.disconnect`,
`live.platform.credentials.store`, `live.platform.credentials.delete` and
`live.platform.analytics`.

## Verification

Tests cover connection isolation, colliding IDs, cancellation/backpressure,
HTTP token validation, scope rejection, rate limits, IRC authentication,
heartbeat, chat/subscription/Bits parsing, points replay/history suppression,
separate balances and analytics, credential permissions, compatibility aliases
and shared widget rendering. Network fixtures need local socket access.
An actual account/token is needed for a live Twitch smoke test; fixtures do not
establish live service delivery.

References:
- https://dev.twitch.tv/docs/chat/irc/
- https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/
- https://dev.twitch.tv/docs/authentication/validate-tokens/
