# Twitch Live reference plugin

Reference implementation of TikTools' plugin-first live platform architecture.

The plugin owns Twitch transport and mapping only. TikTools validates the ingestion batches and owns feed, automation, points, identity, replay protection, persistence and analytics.

## Build

```sh
cargo test
cargo build --release
```

The produced executable is `target/release/tiktools-twitch-plugin` (or the platform-specific executable name). Install/package it together with `plugin.json` using TikTools' local plugin installation flow.

## Configure

Set these plugin settings, or supply them to the connect action:

- `channel`: Twitch login name without `#`
- `oauthToken`: user OAuth token with the `chat:read` scope

Use the plugin's **Twitch Live** tab to connect, disconnect and refresh status.

## Event coverage

- Twitch chat → `live.chat`
- Twitch subscriptions/resubs/gifted subs → `live.subscription`
- Bits cheers → `live.gift` with `unit: twitch_bits`

Twitch IRC does not expose follows, joins, likes or shares, so this plugin does not synthesize them.

See `docs/internal/LIVE-PLATFORM-PLUGINS.md` for the provider contract and implementation checklist.
