# @tiktools/plugin-ui

Broker client for TikTools plugin pages that use `"ui": { "mode": "webview" }`.
A plugin UI never talks to the main TikTools window. It reaches the host only
through this broker:

- in the desktop app the host injects `window.tiktools` into the isolated
  plugin WebView, and the broker calls it directly;
- in a development or sandboxed iframe the broker uses a versioned
  `postMessage` envelope that the parent shim answers;
- on a top-level page with no host every call fails with
  `native plugin broker is unavailable` instead of messaging the page itself.

Plugin identity is always supplied by the host; the UI never sends a plugin id.

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
await broker.setSettings({ ...settings, serverUrl: 'http://127.0.0.1:8080' });
const outcome = await broker.executeAction('acme.tts.speak', { text: 'hello' });
const { options, selected } = await broker.getOptions('acme.tts.speak:voice');
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
```

| Method | Host operation |
| --- | --- |
| `getSettings()` / `setSettings(values)` | `settings.get` / `settings.set` for this plugin |
| `executeAction(action, config)` | `actions.execute`, restricted to this plugin's own actions |
| `getOptions(source, refresh?)` | `options.get`, restricted to this plugin's option sources |
| `subscribe(event, listener)` | `events.subscribe` / `events.unsubscribe` |
| `getLocale()` / `getTheme()` | `host.locale` / `host.theme` |

`BROKER_API_VERSION` is `1`. Envelopes with another version are ignored, so a
UI built against a newer broker fails closed on an older host.
