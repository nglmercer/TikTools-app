# Live platform plugins

TikTools treats live platforms as plugin-owned transports, not as built-in providers. A provider plugin owns only:

- transport, authentication, reconnect and rate-limit behavior;
- provider parsing and validation of provider-specific wire data;
- mapping into the versioned `tiktools-live` ingestion contract;
- provider-specific configuration UI in its own plugin page.

The host remains authoritative for everything that can affect users or money-like counters:

- the `live.ingest` capability and reserved `live.*` namespace;
- ingestion schema/session/payload validation and bounded deduplication;
- platform, session, connection, timestamp, identity and authority stamping;
- feed projection, automation dispatch, points, persistence and analytics;
- compatibility from native `tiktok.*` events into generic `live.*` triggers.

A plugin must never write points, feed rows, analytics or `live.*` automation events directly. It emits `platform.live.ingest` batches. If the manifest does not declare both `events.publish` and `live.ingest`, the host drops the batch.

## Ingestion contract

The shared crate `crates/tiktools-live` is the contract and template for new providers. Batches use `schemaVersion: 1` and up to 96 messages, but plugins should also account for the host plugin-event byte limit; the Twitch reference uses batches of 32 IRC-derived messages.

Message kinds:

- `session_start`: declares a plugin-scoped stream, channel and supported capabilities.
- `session_state`: reports reconnecting/error state with bounded retry metadata.
- `session_end`: ends the stream, optionally with a structured provider error.
- `gap`: explicitly reports lost provider events.
- `event`: carries one canonical `LiveEventInput`.

Canonical payload kinds are `chat`, `follow`, `subscription`, `gift`, `like`, `share`, `join`, `room_stats`, `connected` and `disconnected`. Provider-specific metadata belongs in `extensions` under a provider key; monetary amounts keep their original decimal string and `unit` (for example `twitch_bits`) and are never coerced into TikTok diamonds.

Provider user IDs must be stable provider IDs, not display names or lossy JavaScript numbers. The host derives collision-resistant platform-qualified keys.

## Minimal manifest shape

```json
{
  "id": "example.platform",
  "runtime": "process",
  "entry": "example-platform-plugin",
  "capabilities": ["events.publish", "live.ingest"],
  "eventTypes": [
    { "type": "platform.live.ingest", "title": { "default": "Live ingestion" } }
  ],
  "ui": {
    "apiVersion": 1,
    "mode": "declarative",
    "pages": [{ "id": "main", "title": { "default": "Example Platform" }, "body": { "type": "stack", "children": [] } }]
  }
}
```

The page is surfaced as a normal plugin tab by the existing shell navigation. Connection controls should be plugin actions (`connect`, `disconnect`, `status`); secrets belong in the plugin settings schema with `"secret": true` or in action fields and must never appear in emitted events or logs.

## Twitch reference implementation

`examples/plugin-twitch` is the reference provider. It demonstrates:

- anonymous read-only Twitch IRC over TLS WebSocket;
- reconnect handling;
- bounded IRC parsing;
- mapping `PRIVMSG` to `live.chat`;
- mapping subscription `USERNOTICE` messages to `live.subscription`;
- mapping Bits cheers to `live.gift` with `unit: "twitch_bits"` and no diamond conversion;
- bounded local queues, explicit gaps and batched host ingestion.

Build it independently because installable examples are intentionally outside the app workspace:

```sh
cd examples/plugin-twitch
cargo test
cargo build --release
```

Then package/copy `plugin.json` and the produced `tiktools-twitch-plugin` executable using the normal local plugin installation flow. Configure `channel` in the plugin settings or the connect action. No OAuth token is needed because the reference intentionally uses Twitch's historical anonymous read-only IRC login. That behavior is legacy/undocumented and can change without notice; production providers should migrate to authenticated IRC with a user token scoped `chat:read` or to Twitch's recommended EventSub/API path.

## Provider implementation checklist

1. Depend on `tiktools-live` and emit exactly its versioned batch shape.
2. Declare `events.publish`, `live.ingest` and `platform.live.ingest`.
3. Keep provider IDs stable and input bounded before sending.
4. Declare truthful capabilities in `session_start`; the host rejects events outside them.
5. Use provider source IDs whenever available so host deduplication can suppress reconnect replays.
6. Never invent unsupported event types or convert monetary units.
7. Queue locally with a hard bound and emit `gap` instead of growing memory.
8. Own reconnects in the plugin; report state instead of silently hiding interruptions.
9. Keep secrets out of events, extensions, logs and summaries.
10. Test malformed input, replayed source IDs, reconnects and batch boundaries before publishing the plugin.
