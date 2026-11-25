# Twitch Live reference plugin

Reference implementation of TikTools' plugin-first live platform architecture.

The plugin owns Twitch transport and mapping only. TikTools validates the ingestion batches and owns feed, automation, points, identity, replay protection, persistence and analytics.

## Build

```sh
cargo test
```

From the TikTools repository root, use the plugin devkit to compile and pack the current platform in one step:

```sh
cargo run -p tiktools-plugin-devkit --bin tiktools-plugin --locked -- \
  build examples/plugin-twitch --release --out-dir dist/plugins
```

The command prints the generated archive, normally `dist/plugins/tiktools.twitch-<version>-<target>.plugin`. Install it into the running host with the ordinary CLI:

```sh
target/debug/tiktools --json system info  # exit 0: host is running; exit 3: no IPC host
target/debug/tiktools plugin install dist/plugins/tiktools.twitch-0.1.0-linux-x64-gnu.plugin --replace
```

Adjust the archive name for the version/target printed by `build`. Do not copy `plugin.json` and the executable by hand; the installer validates and stages the package and refreshes the running host.

## Configure

Set this plugin setting, or supply it to the connect action:

- `channel`: Twitch login name without `#`

No OAuth token is needed. This example intentionally uses Twitch's historical anonymous read-only IRC login (`justinfanNNNN`) because it is a compact reference implementation rather than a production chatbot. It can receive room-broadcast public chat, subscription notifications and Bits tags. It cannot send messages, use moderation features or receive follows, channel-point redeems or private monetary data.

Use the plugin's **Twitch Live** tab to connect, disconnect and refresh status.

## Production migration note

The anonymous login is a legacy/undocumented Twitch IRC behavior and may change or disappear without notice. Before reusing this provider in production, migrate the transport to Twitch's officially supported paths:

- authenticated IRC with a user access token that has the `chat:read` scope: <https://dev.twitch.tv/docs/chat/irc/>
- EventSub and the Twitch API, which Twitch recommends for newer chat integrations: <https://dev.twitch.tv/docs/chat/>

External donations such as Streamlabs, StreamElements, PayPal or Ko-fi are not Twitch IRC events. Capture them through the relevant provider's API or alert socket with that provider's credentials instead of treating public alert-bot text as authoritative payment data.

## Event coverage

- Twitch chat → `live.chat`
- Twitch subscriptions/resubs/gifted subs → `live.subscription`
- Bits cheers → `live.gift` with `unit: twitch_bits`

Twitch IRC does not expose follows, joins, likes or shares, so this plugin does not synthesize them.

See `docs/internal/LIVE-PLATFORM-PLUGINS.md` for the provider contract and implementation checklist.
