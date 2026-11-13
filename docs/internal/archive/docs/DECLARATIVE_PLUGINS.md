# Declarative plugins (schema v3)

A declarative plugin integrates an HTTP server (SonicBoom, a webhook, any
JSON API) with **no plugin code at all**: the host interprets the manifest's
HTTP actions, option sources, templates, and pages through its own hardened
pipeline. `plugins-example-tiktools/examples/sonicboom-server-declarative/plugin.json` is the reference package.

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
| `timeoutMs`           | Default timeout for option, health and manifest fetches, 100–120000 (default 10000). Action `http` blocks accept up to 180000 (see [Actions](#actions)). |
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

## Host option sources

Plugins can also consume host-owned, read-only data through an allowlisted
source id. The only host source today is the persisted gift catalog:

```json
"optionsFrom": "host.gifts"
```

```json
"optionsFrom": { "source": "host.gifts" }
```

The host resolves it internally from the same catalog `gifts.list`
serves — no plugin fetch, no endpoint, no extra catalog. Values are gift
names (what event filters store); labels match; `meta` carries `giftId`,
`diamondCount`, and `iconUrl` for rich rendering. Duplicate names collapse
first-wins in catalog order, exactly like the event gift picker, and an
empty catalog yields empty options (never an error), so forms degrade to
their empty state plus manual entry. Anything else under `host.*` is
rejected by validation on both sides, so plugins cannot probe host state.

A field that names `host.gifts` can render the shared gift picker instead
of a plain select through a validated UI hint — either the explicit
marker or the kind shorthand:

```json
"giftName": { "kind": "select", "optionsFrom": "host.gifts", "picker": "gift" }
```

```json
"giftName": { "kind": "gift", "optionsFrom": "host.gifts" }
```

The picker is the existing event `GiftPicker` (icon, name, diamond price,
search, manual entry) fed by the field's dynamic options. Unknown `picker`
values are ignored. Compact `fields` descriptors use the same keys:

```json
{ "key": "giftName", "kind": "select", "optionsFrom": "host.gifts", "picker": "gift", "value": "" }
```

## Actions

An action descriptor with an `http` block runs in the host; the
`requiredCapabilities` gate still applies (`http.request`). Test runs
describe the request without sending. `emitResponseAs` passes through to
the HTTP engine.

### Action request budgets

An action's `http.timeoutMs` accepts 100–180000 ms. The wider ceiling exists
because a local synthesizer — for example a Qwen3-GGUF voice-clone server —
can legitimately take minutes. It applies only to plugin-declared actions:
generic, user-authored `action.http` automation nodes stay capped at
120000 ms, so no unrelated action inherits a multi-minute deadline.

When one action serves several backends with different speeds, declare a
provider-aware table instead of a single number. The host resolves
`config.<field>` against `values`, falling back to `defaultMs`, so a fast
provider is never forced to wait out a slow provider's budget:

```json
"http": {
  "path": "/api/tts/play",
  "timeoutMs": 15000,
  "timeoutByField": {
    "field": "provider",
    "defaultMs": 15000,
    "values": { "supertonic": 15000, "pocket-tts": 60000, "qwen3-gguf": 180000 }
  }
}
```

Resolution precedence, largest scope first: `timeoutByField` matched value →
its `defaultMs` → the action's static `timeoutMs` → the manifest
`http.timeoutMs` → 10000. Every candidate is validated at discovery (a bad
shape or an out-of-range value fails the manifest), and the executor clamps
the resolved budget again as defence in depth. The effective deadline is the
smallest of this budget, the executor ceiling, and the server's own
generation deadline.

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

Action descriptors double as picker cards. Two optional presentation
keys control the card; both are display-only and ignored by older hosts:

```json
{
  "id": "minecraft.server.spawn-mob",
  "title": {"default": "Spawn Mob"},
  "description": {"default": "Spawn a Minecraft entity near a player."},
  "tag": "minecraft",
  "icon": "sparkles",
  "category": "Mobs"
}
```

`icon` names a host-registry icon (`gift`, `terminal`, `chat`, …);
unknown names fall back to a heuristic glyph — plugins can only name
icons, never supply artwork, markup, or components. `category` is a short
grouping label rendered under the title. The picker searches titles,
descriptions, tags, ids, categories, and plugin ids, so one plugin with
several actions reads as a browsable template gallery with no extra
manifest format.

`plugins-example-tiktools/plugins/minecraft-server/plugin.json` is the reference package: six
structured CommandAPI actions (give-item, spawn-mob, apply-effect,
change-weather, send-message, run-command) with curated selects, bounded
numbers, and connection settings — no health probe, because the
CommandAPI documents no health endpoint. It stages as a development
plugin automatically (`bun scripts/task.ts prepare-plugins`).
`plugins-example-tiktools/examples/profiles/minecraft-giftwall.tikprofile.json` is a ready-made
rule pack on top of it: 14 gift rules plus follow/share announcements,
every rule flashing a `/title` + subtitle.

`plugins-example-tiktools/plugins/peak-game/plugin.json` drives PEAK through PEAKapi, the local
BepInEx REST API of the game (spawn item, entity and hazard, despawn one
API-owned object, clear them all). It reads the item, entity and hazard
catalogs as dynamic selects, probes `/health` for the `gameplayReady`
readiness signal, and imports the token and the API port from the game's
own `BepInEx/config/org.peak.rest-api.cfg`, so the connection card can
find the installation by itself.

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

Schema-v3 pages keep working unchanged: the host adapts each section kind
to the generic declarative UI contract (`src/plugin-ui/`) at render time —
`text`/`form`/`connection`/`list` become generic nodes, and `tts` becomes
a host-rendered panel plus an explicit TTS contribution. See
[Plugin UI architecture](PLUGIN_UI_ARCHITECTURE.md) for the node set,
bindings, action allowlist, and the future isolated-WebView mode for
fully custom plugin UI.

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

## Structured JSON and execution keys

Actions may opt into `http.bodyJson` instead of the legacy string `body`.
Objects/arrays are rendered structurally and serialized once; strings are escaped
by the JSON serializer. A binding `{ "$value": "config.itemId" }` preserves the
resolved JSON type. `{ "$number": "config.quantity", "min": 1, "max": 10,
"integer": true, "fallback": "settings.quantity" }` converts and validates
numeric form input. Missing required bindings fail before sending. A `fallback`
is a scope path used for missing/blank input.

`bodyVariants: { "field": "positionMode", "values": { "random_player":
{ "position": { "mode": "random_player" } } } }` merges the selected variant
into the base body before rendering; the field resolves from config then settings.
This permits nested conditional payloads without raw JSON interpolation.
Legacy string bodies retain their behavior.

`http.idempotency: { "enabled": true, "header": "Idempotency-Key" }` generates
one cryptographically random 128-bit execution key in the prepared request.
Different executions, including within one event, receive different keys. There
are no automatic gameplay retries. Any future retry must reuse the prepared
request, including body and key. Opt-in calls share a fail-fast four-request /
sixteen-starts-per-second budget; no additional pending queue is created.

A health descriptor may declare `readinessField` naming a boolean response field.
A false/missing value reports the server as reachable but gameplay unavailable,
with the game readiness diagnostics. R.E.P.O. uses `gameplayReady`.

`"strategy": "service-info"` selects a staged probe for servers that separate
network access, authorization and model loading. `path` answers a public
discovery document (`{auth: {required}, setup_complete, model: {status},
features: {playback}}`) and the optional `authProbePath` names any protected,
model-independent endpoint that proves the saved credential works:

```json
"health": { "strategy": "service-info", "path": "/api/info", "authProbePath": "/api/protected", "timeoutMs": 5000 }
```

The connection card then shows sign-in, setup and model-loading states from
the probe instead of a plain latency check. The legacy strategy name
`sonicboom` is a deprecated alias kept for already-installed manifests.

## Import a token from local configuration

A plugin may offer **Import token from configuration…** on the existing shared
connection card, independently of admin-login `tokenProvisioning`:

```json
"http": {
  "baseUrl": "{{ settings.serverUrl }}",
  "auth": { "type": "bearer", "tokenSetting": "apiToken" },
  "tokenImport": {
    "format": "ini",
    "fileName": "org.repo.rest-api.cfg",
    "section": "HTTP",
    "key": "AuthenticationToken",
    "minLength": 43
  }
}
```

`auth.tokenSetting` must name a setting marked `secret: true` in
`settings.schema.properties`; otherwise the host does not offer import.
`fileName` is a filename hint in the native dialog, never a filesystem path.
The reusable INI parser supports UTF-8 (optional BOM), CRLF/LF, whole-line
`#`/`;` comments and unquoted `key = value` entries. Values preserve embedded
`=` (including Base64 padding). Section and key names are case-sensitive.
Duplicate matching keys, empty/whitespace/control-containing tokens, invalid
UTF-8, non-files and files over 64 KiB are rejected without echoing contents.
`minLength` defaults to 1; all tokens are capped at 4096 bytes. The manifest
validator bounds every parser option. Additional file formats require host
parser support; plugins cannot supply executable readers.

The `plugins.token.import` control operation accepts `{ "pluginId": "repo.game" }`
and opens the existing host-owned native picker with configuration-file filters.
It accepts no filesystem path or file contents from the caller. The default
`file` mode reads only the file the operator selects, on a blocking worker. Plugins
can additionally opt into the bounded discovery described below. Only one import
dialog may run at a time.
The result uses `{pluginId, ok, error}`: cancellation is `ok: false` with no error.
File contents and extracted tokens stay inside the host. Existing settings are
merged through the normal secret persistence, cache invalidation and change-event
path. Disabled, unloaded or changed plugins cannot finish an outstanding import.
The UI reloads masked settings and tests the connection after success; it blocks
import during pending edits and removes the settings form while importing to
avoid racing autosaves with the new secret.

`plugins-example-tiktools/plugins/repo-game/plugin.json` and `plugins-example-tiktools/plugins/peak-game/plugin.json` opt in.
SonicBoom retains its admin login flow and Minecraft stays unchanged. A future
plugin with an INI token can opt in with its own filename/section/key and secret
token setting, using the same connection UI and executor. Token import does not
disable authentication or modify the source server configuration.

### Automatic connection and game-folder setup

An INI token import can declare where its local game configuration resides and
how to import its endpoint:

```json
"discovery": {
  "steamFolder": "REPO",
  "modManagerGame": "REPO",
  "relativePath": "BepInEx/config/org.repo.rest-api.cfg"
},
"connection": {
  "setting": "serverUrl",
  "section": "HTTP",
  "hostKey": "Host",
  "portKey": "Port"
}
```

These keys live inside `http.tokenImport`. Paths must be relative, at most eight
components, without traversal, and end in `fileName`. Steam/mod-manager game names
must be single folder names. The URL destination must be a declared URI setting.
R.E.P.O. and PEAK opt in; SonicBoom's admin login and Minecraft are unchanged.

`plugins.token.import` retains its file-picker default and supports:

- `{pluginId, mode: "auto"}`: return `{pluginId, ok: false, error: null, candidates}`
  with opaque IDs and installation/profile labels. Even a single match requires
  selecting **Use this configuration**. Discovery never reads the token.
- `{pluginId, mode: "candidate", candidateId}`: re-discover the allowlisted source
  and import it. Stale/unknown IDs fail without writing settings; callers cannot
  send filesystem paths.
- `{pluginId, mode: "folder"}`: native folder picker for the installation or active
  mod profile. Only the declared configuration suffix is checked; selecting its
  intermediate BepInEx/config folder also works.
- `{pluginId, mode: "file"}`: existing configuration-file picker.

Known roots cover standard Linux/Windows/macOS Steam locations, Linux Steam
Flatpak, `steamapps/libraryfolders.vdf`, and known r2modmanPlus-local / Thunderstore
DataFolder profiles under user configuration directories. Custom roots can use the
folder picker. There is no recursive disk search: scans cap libraries and profiles
at 128 entries per root, candidates at 64, Steam metadata at 128 KiB and imported
configurations at 64 KiB. Canonical paths must stay inside the installation/profile
or selected folder; symlink escapes are rejected. No search runs on LIVE events.

Token and endpoint are validated before one merged settings write. Ports must be
1–65535. Hosts must be `localhost`, `127.0.0.1`, `::1`, `0.0.0.0` or `::`; wildcard
listen addresses map to loopback client URLs, so importing never sends credentials
to a remote address. Missing host and port together keep the existing URL; a
partial or invalid endpoint fails without changing the token or URL. Other server
settings, including authentication, remain untouched.

The UI reloads masked settings and probes after import. `plugins.health` exposes
`reachable` separately from optional `gameplayReady`; a reachable API with no ready
level is explicitly labelled as connected but not gameplay-ready. Existing `ok`
and error semantics remain compatible. The Rust SDK retains its file-import
method and adds `plugins_token_setup` for the optional modes.
