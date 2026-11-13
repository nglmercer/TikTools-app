# User Guide

TikTools is organized around a live connection and a set of local dashboard tabs.

## Connect to TikTok LIVE

Open **Connect** and enter a creator handle.

### Guest mode

Leave the Cookie field empty. The host asks the upstream signer for an anonymous guest session and then connects to a matching live room. Guest sessions are short-lived and can be rate-limited.

### Authenticated mode

Paste the Cookie request header from an authorized browser session when a room needs authentication. The app forwards the value to the host for the connection and keeps it in memory only.

Treat the value as a password:

- Do not commit or log it.
- Do not paste it into issues or screenshots.
- Only connect to rooms you are authorized to monitor.

### Automatic live selection

**Pick a live automatically** uses the available discovery flow to find a live room for the creator, select a result, and connect with its room ID.

## Dashboard tabs

### Feed

Feed is the live event stream. It displays chat, gifts, likes, membership/follow activity, and social events. The filter controls, search, unread state, and auto-scroll behavior are local UI state for the current session.

Rows stay deliberately compact: avatar, name, a small kind glyph, the message, and a timestamp. Level, points, gift cost, message IDs, and the exact time live in the context panel instead of on every row, so the stream stays readable under load.

**Context panel.** The panel on the right has three modes:

- **Viewers** — the current top contributors.
- **Details** — the selected event, with everything the row hides.
- **Profile** — the selected viewer's tracked stats.

Click a row's message to open its details, or the avatar/name to open that user's profile. `Esc` or the close button dismisses the panel. On narrow windows the panel becomes a drawer over the stream rather than squeezing it. The panel never auto-opens on incoming events, and selecting an event does not pause the stream.

#### Explore LIVE

**Connect → Explore LIVE** browses rooms that are live right now. It is closed by default and only queries TikTok when you open it. Search is debounced, results are cached briefly, and each outcome is distinct: no matches, sign-in required, rate limited, or a transient failure with a retry button. **Connect** on a row connects to that specific room through the normal connection flow.

**Choose randomly** samples uniformly from the rooms already listed (excluding the current creator when there is an alternative) and shows a preview — nothing connects until you press Connect. It never selects a room you have not seen.

#### Connection status

The top nav shows the current state: Connected, Connecting, Reconnecting, Offline, or Connection Error. When the host reports it, a reconnect shows the attempt and the retry delay. A reconnect to the same creator — manual or automatic — keeps the current feed; only switching to a different creator starts a new feed.

### Points

Points are stored in SQLite and can be awarded for:

- Coins from gifts.
- Shares.
- Chat messages.
- Likes.
- Follows.
- Joins, when enabled.

Each rule has an enable switch. The points page also supports a subscriber bonus percentage, points-per-level threshold, manual point adjustments, reset actions, and a searchable leaderboard.

Point totals are clamped at zero when points are deducted. A viewer’s level is calculated from their total points and the configured points-per-level value.

### Analytics

Analytics summarizes the current in-memory session: event totals, engagement activity, and top viewers received from the live room. Resetting or ending the connection clears the session display; persisted points and creator history remain available.

### Behavior

Behavior is the current rule editor. It separates:

- **Actions**: reusable operations such as logging, HTTP, points, audio, or TTS.
- **Events**: triggers with filters, cooldowns, and a list of actions to run.

Filters in a rule must all pass. Use an `in` comparison inside one filter when several values should match. Use the built-in test controls before enabling a rule against a live room.

See [Automations](AUTOMATIONS.md) for the event model, templates, script actions, capability restrictions, and plugin boundary.

### Plugins

Plugins add optional action types and dependencies. Built-in actions are always available. Plugin actions appear only when their plugin is installed, enabled, and its dependency is available.

Review a runtime plugin manifest and its permissions before enabling it. Native
plugins are trusted in-process code; process plugins provide crash isolation,
while WASM provides a sandbox boundary whose WASI and host capabilities are
explicitly selected by TikTools.

### Widgets

Widgets are OBS Browser Source overlays for follows, gifts, chat, shares, and subscriptions. They render the same canonical live events the dashboard shows; they never connect to TikTok directly.

Each widget card shows a redacted OBS URL placeholder, a copy button, and a live preview. Copy places the complete URL — with its dedicated widget credential — on the clipboard; paste it into OBS as a Browser Source (transparent background is already handled). The token travels in the URL fragment (`#token=…`), which browsers never send to the server — do not move it into the query string.

Previews play one synthetic alert locally and need no live connection. The gateway status line reports the host-side state (missing, disabled, stopped, starting, credential-unavailable, assets-missing, unreachable, ready): enable and start the Event Gateway once, then copy the URLs. Copied URLs survive TikTools restarts, plugin disable/re-enable, and reinstalls; only resetting the gateway settings generates new credentials.

### Settings

Settings controls the interface language and dark/light theme. Preferences are saved in WebView local storage.

## Tray behavior

Closing the native window hides it instead of shutting down the process. Use the tray menu to show the window again or quit. A normal quit stops the live client, automation, and running plugins.

## What persists

| Data | Location | Lifetime |
| --- | --- | --- |
| Language, theme, creator handle, recent handles | WebView local storage | Across launches |
| Points, viewers, transactions, creator history | `data/tiktok-points.db` | Across launches |
| Workflows, behavior actions/events, plugin state | `data/tiktok-automation.db` | Across launches |
| Live feed, telemetry, run history, last event snapshot | Application memory | Current process/session |
| Session Cookie header | Application memory | Current connection/session only |
