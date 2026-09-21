# Plugin UI architecture

TikTools renders plugin configuration UI with a host-owned declarative
system: **plugins describe UI as data; the TikTools frontend renders
trusted components.** The main WebView holds the privileged IPC bridge
(`window.ipc`), so arbitrary plugin JavaScript, HTML, Vue/React
components, and document CSS never execute there.

## Layers

```text
plugin.json
    │
    ▼
typed manifest (crates/tiktools-plugin-api/src/ui/)
    │ host validation at discovery
    ▼
Plugin UI descriptor
    │ legacy schema-v3 → generic adapter (src/plugin-ui/)
    ▼
generic Vue renderer (src/web/plugin-ui/)
    │  PluginPage → PluginNode registry → host components
    ▼
settings bindings / option sources / plugin actions / host services
```

Canonical contract: `crates/tiktools-plugin-api/src/ui/` (Rust types +
validation + `JsonSchema` derives). TypeScript mirror:
`src/plugin-ui/contracts.ts`. Validation parity is covered by fixture
tests on both sides (`ui::validation::tests`, `src/plugin-ui/*.test.ts`).

## UI modes

**Mode A — Declarative UI (default).** A plugin declares a `body` tree of
generic nodes. The host validates and renders it with its own components.

```json
{
  "uiVersion": 1,
  "pages": [
    {
      "id": "tts",
      "title": { "default": "Text to Speech" },
      "icon": "voice",
      "body": {
        "type": "stack",
        "children": [
          {
            "type": "select",
            "label": { "default": "Default voice" },
            "bind": "settings.defaultVoice",
            "optionsFrom": "plugin-action-options:sonicboom.server.speak:voice"
          },
          {
            "type": "range",
            "label": { "default": "Volume" },
            "bind": "settings.volume",
            "min": 0,
            "max": 1,
            "step": 0.01
          }
        ]
      }
    }
  ]
}
```

**Mode B — Isolated custom WebUI (future/advanced).** For plugins that
genuinely need arbitrary UI (waveform editors, node editors, mixers).
Custom UI runs in a **separate isolated WebView**, never in the main
privileged document, through a restricted `PluginUiBroker`:

```text
Main TikTools WebView (trusted) ──► Control API ──► AppCore
Plugin custom WebView (restricted) ──► PluginUiBroker ──► AppCore
```

The broker exposes only scoped operations (`settings.get/set`,
`actions.execute`, `options.get`, `events.subscribe`) with plugin
identity, capability checks, permission checks, and an operation
allowlist. Custom WebViews never receive raw `window.ipc`,
`ControlApi.call()`, `AppCore` references, filesystem/database handles,
or Wry handles. Shadow DOM is not a security boundary and is never used
as one. Mode B is not implemented yet: `crates/tiktools-plugin-api/src/ui/manifest.rs`
types the manifest fragment (`PluginUiManifest`) so manifests can grow
toward it without breaking.

## Node set (uiVersion 1)

Closed and generic: `stack`, `card`, `text`, `form`, `select`, `range`,
`checkbox`, `button`, `list`, `status`, `separator`, `connection`,
`tts-settings`. New domains compose these primitives — the renderer
registry (`PluginNode.vue`) never gains per-domain section kinds.
`tts-settings` is a host-rendered panel addressed by contribution id;
the generic renderer resolves it through host-injected `customNodes`
and never imports TTS UI itself.

## Bindings

Intentionally limited — no expressions, no `eval`, no `new Function`:

- `settings.<dotted-path>` — plugin settings values (read/write).
- `local.<name>` — page-local ephemeral state (never persisted).
- `source.<field>` — read-only selected value of the option source whose
  field segment matches (e.g. `source.voice`).

Prototype keys (`__proto__`, `prototype`) and non-identifier segments
fail closed.

## UI actions (allowlist)

`save-settings`, `plugin-action`, `refresh-source`, `test-connection`,
`open-media-picker`. There is no generic arbitrary-RPC action;
capability checks remain host-side.

## TTS contributions

Auto-TTS, speech dispatch, and voice policy key off explicit
`TtsContribution` records (`{ id, pluginId, actionType, voicesFrom,
outputsFrom? }`), never off walking UI sections. Schema-v3
`"kind": "tts"` sections generate one contribution plus a
`tts-settings` node in `legacy-v3-adapter.ts` — the only place that walk
exists. Future hosts may serve contributions directly via
`plugins.ui.describe` with a `plugin.ui.changed` event; `useTts` already
accepts explicit contributions that override the adapter-derived ones.

## Backward compatibility

Existing schema-v3 manifests (including SonicBoom) work unchanged:

| v3 `kind`  | generic nodes                                  |
|------------|-----------------------------------------------|
| `text`     | `text`                                        |
| `form`     | `form` (same schema/uiHints fallback)         |
| `connection` | `connection`                                |
| `list`     | `list` (+ built-in refresh button)            |
| `tts`      | `tts-settings` + generated `TtsContribution`  |

`PluginPageView.vue` is the thin compatibility wrapper: it adapts the v3
descriptor once and renders the generic `<PluginPage>` with a single
`PluginUiContext` (locale, settings, options, actions, connection,
media, provisioning, local state, form drafts, custom nodes). Visuals
are preserved exactly — verified by the Playwright baselines in
`tests/e2e/screenshots.spec.ts-snapshots/`.

## Security properties

- No `v-html`, no `innerHTML` for plugin content, no `eval`/`new Function`.
- Manifest text renders as text and form controls only.
- Plugin paths canonicalized; navigation restricted; secrets redacted.
- Capability validation stays host-side; process plugins get no native handles.
- UI actions are allowlisted data, never executable plugin code.
- Every new plugin UI feature needs a negative test proving a plugin
  cannot exceed its declared scope.
