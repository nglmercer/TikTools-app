# Declarative plugins (schema v3)

A declarative plugin integrates an HTTP server (SonicBoom, a webhook, any
JSON API) with **no plugin code at all**: the host interprets the manifest's
HTTP actions, option sources, templates, and pages through its own hardened
pipeline. `examples/sonicboom-server/plugin.json` is the reference package.

Schema v2 manifests keep working unchanged: declarative keys are only read
when `"schemaVersion"` is `3`, and `runtime: "declarative"` packages need no
`entry` file.

## Minimal manifest

```json
{
  "schemaVersion": 3,
  "id": "sonicboom.server",
  "name": "SonicBoom Server",
  "version": "1.0.0",
  "runtime": "declarative",
  "capabilities": ["http.request"],
  "permissions": ["network.bind"],
  "http": {
    "baseUrl": "{{ settings.serverUrl }}",
    "auth": { "type": "bearer", "tokenSetting": "apiToken" },
    "health": { "path": "/ready" }
  },
  "actionTypes": [
    {
      "id": "sonicboom.server.speak",
      "http": {
        "method": "POST",
        "path": "/api/tts/play?voice={{ config.voice }}",
        "headers": { "Content-Type": "text/plain" },
        "body": "{{ config.text }}"
      },
      "optionSources": {
        "voice": { "path": "/v1/voices" }
      }
    }
  ]
}
```

## Templates

`{{ event.* }}`, `{{ settings.* }}`, and `{{ config.* }}` render before
every request. Unknown paths render empty. The rendered base URL must stay
http(s) with a host, the rendered path must stay base-relative, and the
destination host must match the base host, otherwise the request is refused.

## The `http` block

| Key                   | Meaning                                                            |
| --------------------- | ------------------------------------------------------------------ |
| `baseUrl`             | Literal http(s) URL or `{{ settings.* }}` template (required).     |
| `timeoutMs`           | Default timeout, 100–120000 (default 10000).                       |
| `allowPrivateNetwork` | Opt in to private LAN hosts (default false).                       |
| `headers`             | Extra headers for every request (templates allowed).               |
| `auth`                | `none`, `bearer`, `header`, or `query` (see below).                |
| `health`              | `{ path, timeoutMs? }` probed by Test connection.                  |

Auth shapes:

- `{"type": "none"}` — loopback only.
- `{"type": "bearer", "tokenSetting": "apiToken", "scheme": "Bearer"}` —
  `Authorization: Bearer <token>`.
- `{"type": "header", "tokenSetting": "...", "header": "X-Token"}`.
- `{"type": "query", "tokenSetting": "...", "param": "token"}`.

## Trust policy

- **Loopback is trusted.** `localhost`, `*.localhost`, `127/8`, and `::1`
  need no permission and no token (a configured token is still sent).
- **Beyond loopback requires both.** The manifest must declare the
  `network.bind` permission, auth must be declared, and the token setting
  must be non-empty, otherwise the request is refused before sending.
- **LAN hosts additionally require** the manifest `allowPrivateNetwork`
  opt-in.
- Every fetch reuses the automation HTTP engine: no redirects, host
  pinning, 2 MiB cap, DNS re-resolution. Actions go through
  `execute_http_action`; option and probe fetches use the same sender.

## Secret settings

Flag sensitive settings in the schema or UI hints:

```json
"apiToken": { "type": "string", "secret": true }
```

The host stores the real value but only ever emits `••••••••` to the
WebView. Saving the placeholder preserves the stored secret; only a changed
value overwrites it. Tokens are also redacted from summaries, logs, and
error strings (query-auth URLs included).

## Dynamic options

A select field names its source (string or object form):

```json
"optionsFrom": "plugin-action-options:sonicboom.server.speak:voice"
```

```json
"optionsFrom": { "source": "plugin-action-options", "actionType": "...", "field": "voice" }
```

The host resolves the action's `optionSources[field]` endpoint, fetches it
with the plugin's base URL and auth, maps the body to `{value, label}`
pairs, and caches the list for 60 seconds (any settings save clears the
cache). Mapping defaults: items from `itemsPath`, a root array, or the
first of `items`/`voices`/`data`; value from `valuePath`/`id`/`value`;
label from `labelPath`/`name`/`label`/value.

Option documents may also advertise the server-side selection: a
top-level scalar `selected` wins, otherwise the first item carrying a
literal `is_selected: true` contributes its mapped value. The selection
travels with the options (`action-options` `selected`) so selectors can
show server truth instead of a locally remembered value. Documents
without either (voice lists, plain arrays) report no selection.

Option fetches never fail the surrounding surface: an unreachable
server, a 404 from an older server without the endpoint, or an
unmappable body yields empty options plus a display-safe error, and
every other source on the page keeps working. Each fetch sends exactly
one request; nothing retries, so a 429 surfaces as an error for the
operator instead of a retry storm.

A page-supplied `optionsUrl` is always ignored: only manifest-declared
endpoints are fetched, so the WebView cannot steer the host at new URLs.

## Actions

An action descriptor with an `http` block runs in the host; the
`requiredCapabilities` gate still applies (`http.request`). Test runs
describe the request without sending. `emitResponseAs` passes through to
the HTTP engine.

Host surfaces may also run actions immediately
(`execute-plugin-action`): the TTS voice tester runs the speech action,
and the TTS audio output selector runs the switch action below. Only
actions declaring a `text` field require spoken text; textless actions
run with the given config:

```json
{
  "id": "sonicboom.server.set-output-device",
  "requiredCapabilities": ["http.request"],
  "fields": [
    {
      "key": "device",
      "kind": "select",
      "value": "default",
      "optionsFrom": "plugin-action-options:sonicboom.server.set-output-device:device"
    }
  ],
  "http": {
    "method": "POST",
    "path": "/api/audio/output",
    "headers": { "Content-Type": "application/json" },
    "body": "{\"device\":\"{{ config.device }}\"}",
    "timeoutMs": 10000
  },
  "optionSources": {
    "device": {
      "path": "/api/audio/devices",
      "itemsPath": "devices",
      "valuePath": "id",
      "labelPath": "name",
      "timeoutMs": 8000
    }
  }
}
```

The same descriptor also surfaces in automations, where the `device`
field renders as a select fed by the live endpoint — workflows can
switch outputs mid-stream without touching settings.

## Templates

```json
"templates": [
  {
    "id": "chat-tts",
    "title": { "default": "Chat to TTS" },
    "eventType": "tiktok.chat",
    "requiredNodeTypes": ["trigger.event", "action.http"],
    "params": { "type": "object", "properties": { "voice": { "default": "M1" } } },
    "workflow": { "nodes": [{ "type": "trigger.event" }, { "type": "action.http", "config": {} }] }
  }
]
```

Valid templates merge into the template modal after the builtins with a
schema-driven options form; `{{ params.* }}` substitutes at creation while
`{{ event.* }}` survives for runtime. Invalid entries are skipped with a
warning. Templates only surface while their plugin is installed, enabled,
and available.

## Pages

```json
"pages": [
  {
    "id": "connection",
    "title": { "default": "Connection" },
    "icon": "radio",
    "sections": [
      { "kind": "text", "text": { "default": "..." } },
      { "kind": "connection" },
      { "kind": "form" },
      { "kind": "list", "optionsFrom": "plugin-action-options:..." }
    ]
  }
]
```

Section kinds form a fixed host-rendered widget set: `text` (plain text),
`form` (settings schema, full schema when omitted), `connection` (centered
connection card), `list` (rows from an option source), `tts` (voice policy
and test panel). Unknown kinds are rejected at validation, and manifest
strings are never parsed as markup: no `html`, `script`, `component`, or
equivalent can reach the WebView. Pages become navigation tabs
(`plugin:<pluginId>:<pageId>`) with allowlisted icons, and disappear with
their plugin — except connection-only pages (a `connection` section plus
optional intro `text`): those stay out of the rail and render inline in
the Connections tab server list through the same shared connection card,
reusing the page icon, so every server-style plugin doesn't mint a
duplicate minimal tab.

A `tts` section needs `actionType` (speech action) and `voicesFrom`
(voice option source). It may also declare `outputsFrom`, an option
source whose action switches the server-side audio output:

```json
{
  "kind": "tts",
  "actionType": "sonicboom.server.speak",
  "voicesFrom": "plugin-action-options:sonicboom.server.speak:voice",
  "outputsFrom": "plugin-action-options:sonicboom.server.set-output-device:device"
}
```

The marker feeds the Audio output selector and addresses the switch
action at once: the host loads options plus the server-reported
selection from the source, and choosing a device runs the named action
immediately (`execute-plugin-action`) with `{device: "<id>"}`. Success
re-reads the selection from the server; failure shows the error and the
selector falls back to the last confirmed server value — nothing about
the output is ever persisted locally. When the fetch fails (older
server without the endpoint, offline server), the selector hides behind
an "unavailable" note with a refresh button while voices, speech, and
the rest of the panel keep working.

A `connection` section embeds the full settings form in one centered card:
primary fields, an explicit **Test connection** button, and a status line,
with `advanced` hint fields collapsed under Advanced options. The first
string field declaring `format: "uri"` is validated inline as the server
URL; probe failures (unreachable server, missing token, runtime errors)
render as the card banner. Edits autosave on blur and after a short
debounce, confirmed by the host settings echo (`Saving…` / `Saved` /
`Error saving`); a passing probe collapses the card to a compact summary
with **Test again** and **Edit settings**.

## Settings defaults

Scalar `default` entries in `settings.schema.properties` apply to every
missing key on load: probes, actions, option sources, and the display echo
all see them, so first-run behavior matches a saved config. Stored values
always win, and secret keys are never defaulted — absent secrets stay
absent. The save echo overlays the same defaults; the settings file keeps
exactly what was sent.

## Connection probing

The Plugins card shows **Connect** for plugins declaring `http.health`;
the dialog edits settings and probes the endpoint, reporting latency or a
redacted error. Probing is generic IPC (`test-plugin-connection` /
`plugin-connection-result`) shared by every declarative integration.
