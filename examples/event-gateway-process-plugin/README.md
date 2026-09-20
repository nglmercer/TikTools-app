# TikTools Event Gateway process plugin

This optional example plugin subscribes to the generic `DomainEvent` stream
through `events.subscribe` and exposes it locally:

- `GET http://127.0.0.1:17452/health`
- `GET http://127.0.0.1:17452/events` as authenticated NDJSON
- `ws://127.0.0.1:17452/ws` as authenticated JSON WebSocket messages

The plugin owns all transport behavior. TikTools core only sends serialized
`{ "topic": "...", "data": ... }` envelopes through the plugin protocol.

The bind address is loopback-only. Remote/LAN binding is rejected explicitly;
an allowed origin must be listed in the plugin settings. The first start
generates a token in the plugin's settings file. HTTP clients use
`Authorization: Bearer <token>` (or a `token` query parameter for `/events`).
WebSocket clients send:

```json
{"type":"auth","token":"..."}
{"type":"subscribe","topics":["live.*","plugin.*"]}
```

Build the executable and place it beside `plugin.json` under the package
directory:

```bash
cargo build --release --manifest-path examples/event-gateway-process-plugin/Cargo.toml
```

Disabling or uninstalling the plugin stops the process and removes the
gateway; TikTools itself does not open any HTTP or WebSocket listener.
