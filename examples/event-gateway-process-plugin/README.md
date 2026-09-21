# TikTools Event Gateway process plugin

This optional example plugin subscribes to the generic `DomainEvent` stream
through `events.subscribe` and exposes it locally:

- `GET http://127.0.0.1:17452/health`
- `GET http://127.0.0.1:17452/events` as authenticated NDJSON
- `ws://127.0.0.1:17452/ws` as authenticated JSON WebSocket messages
- `GET http://127.0.0.1:17452/widgets/follow/` as the Follow Alert OBS page
- `GET http://127.0.0.1:17452/widgets/gift/` as the Gift Alert OBS page

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

## Widget pages

`/widgets/follow/` and `/widgets/gift/` serve the OBS alert widgets when
their assets are present. Build and stage them from the repository root:

```bash
bun run build:widgets
bun run sync:gateway-widgets
```

The sync copies `dist/widgets/` into this example's `dist/widgets/`, which
both the plugin packager and the dev-plugin staging copy beside the gateway
executable. The gateway probes `<executable-dir>/widgets/` then
`<executable-dir>/dist/widgets/`; the `widgetsDir` setting overrides both.
Without assets the widget routes answer 404 and the event APIs keep working.

Widget pages carry no secrets, so they stay unauthenticated (loopback-only,
behind the standard origin check like `/health`). The widget token travels in
the page URL fragment (`#token=...`), which browsers never send to the
server; see the TikTools Widgets settings page, which builds the full OBS
URL including the fragment from the gateway port and token.
