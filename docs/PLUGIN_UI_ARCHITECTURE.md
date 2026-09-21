# Plugin UI architecture

TikTools owns the secure host and generic APIs. Plugins own
plugin-specific behavior and UI. Simple UI is declarative; complex UI is
compiled separately and loaded in an isolated WebView. Nothing
plugin-specific is bundled into the main frontend.

```text
TikTools core/web
├── generic navigation
├── generic plugin host (declarative renderer + webview launcher)
├── generic settings/actions/options broker
└── NO plugin-specific components

plugins/sonicboom/
├── plugin.json          (manifest: actions, settings, pages, ui, entry)
├── backend/             (Rust process plugin: chat observer + speech)
├── shared/tts/          (speech policy shared by UI previews)
└── ui/                  (standalone Vite app → ui/dist/, isolated)
```

## Layers

```text
plugin.json
    │
    ▼
typed manifest (crates/tiktools-plugin-api/src/ui/)
    │ host validation at discovery
    ▼
snapshot: pluginPages (legacy nav) + pluginUis (typed descriptors)
    │
    ├─► declarative UI ──► generic Vue renderer (src/web/plugin-ui/)
    │                       PluginPage → PluginNode registry → host components
    │
    └─► webview UI ──► sandboxed frame inline in the tab
                        (native pop-out window on demand)
                        compiled plugin assets + restricted broker
```

Canonical contract: `crates/tiktools-plugin-api/src/ui/` (Rust types +
validation + `JsonSchema` derives). TypeScript mirror:
`src/plugin-ui/contracts.ts`. Validation parity is covered by fixture
tests on both sides (`ui::validation::tests`, `src/plugin-ui/*.test.ts`).

## UI modes

**Mode A — Declarative UI (default).** A plugin declares a `body` tree of
generic nodes. The host validates and renders it with its own components.
Works with no custom WebView anywhere; this is the fallback every plugin
gets for simple configuration.

**Mode B — Isolated custom WebView.** For plugins that genuinely need
arbitrary UI. The plugin UI is compiled separately (`ui/dist/`) and
renders **inline in the plugin tab** inside a `sandbox="allow-scripts"`
frame at an opaque origin — separate document, separate storage, no
access to the host — served from `tiktools-plugin://app/<plugin-id>/…`
with a strict CSP (no network, no subframes, framing limited to the
TikTools host itself). The tab also offers a pop-out action that opens
the same page in an isolated native window. Either way the document
never sees the privileged bridge:

```text
Main TikTools WebView (trusted) ──► Control API ──► AppCore
Plugin frame/window (restricted) ──► PluginUiBroker ──► AppCore
```

Pop-out windows get an initialization script that installs the narrow
`window.tiktools` surface on top of that window's own `window.ipc`
transport before page scripts run; inline frames use the `postMessage`
transport with a host shim bound to the frame's `contentWindow`. The
plugin window's transport is deliberately left in place (on WebKitGTK
Wry installs it as a non-configurable property before init scripts, so
deleting it aborts the script): the security boundary is the Rust
handler behind it. That handler is the restricted `PluginUiBroker` —
never the main window's full IPC router — so raw `window.ipc` posts
from a pop-out page are still confined to the broker allowlist and the
bound plugin id. Plugin JS is never imported into the main Vue runtime.
Shadow DOM is not a security boundary and is never used as one.

The plugin client picks its transport explicitly: native `window.tiktools`
when injected, `postMessage` when embedded in a frame, and an explicit
`native plugin broker is unavailable` failure on a top-level page with no
host (it never posts to `window.parent` there, which would message the
page itself and surface as a generic `broker error`).

## Broker protocol

Three sides share one versioned envelope — `{apiVersion: 1, id,
method, params}` → `{apiVersion: 1, id, ok, result|error}`, events as
`{apiVersion: 1, event, data}` — pinned by interop tests, never by
shared imports (the plugin package must stay buildable on its own):

- Plugin client: `plugins/sonicboom/ui/src/broker.ts`
- Native broker: `crates/tiktools-desktop/src/plugin_webview/broker.rs`
- Web host shim: `src/web/plugin-ui/plugin-webview-host.ts`

Allowlisted methods: `settings.get`, `settings.set`,
`actions.execute`, `options.get`, `events.subscribe`,
`events.unsubscribe`, `host.locale`, `host.theme`. Anything else fails
closed. The broker binds one plugin id per window/frame and injects it
into every downstream call (`plugins.*` scoped operations); a
client-supplied `pluginId` that disagrees is rejected, so a plugin page
can only read its own settings/options and execute its own actions.
Broker action execution is always live (test buttons take real effect).

Crossing notes: `postMessage` payloads must be plain JSON — Vue
reactive Proxies are rejected by structured clone, so the plugin client
deep-declones every request before posting.

**Opaque-origin CORS rule: all plugin asset responses must be readable
by the opaque/`null`-origin sandboxed plugin document.** Inline plugin
frames run `sandbox="allow-scripts"` (no `allow-same-origin`), so the
document origin is opaque — and strict engines (observed on WebKitGTK)
CORS-check even same-scheme subresource loads from the custom protocol.
Without `Access-Control-Allow-Origin: *` on every served asset, the
bundle's external module scripts and stylesheets are rejected and the
frame stays blank with a bare 200 status. The desktop custom protocol is
NOT exempt by construction. This applies to every plugin-asset host:
the desktop `PluginAssetServer`/`SharedPluginAssetServer` (success AND
error responses, so a 404 surfaces as its real status instead of a
misleading CORS failure), the Vite dev middleware (`/__plugins/…`), and
`vite preview`. Error responses carry the same CORS/CSP headers as
successful ones; both desktop servers share one response builder so the
headers cannot drift.

## Backend (process plugin)

Speech is plugin-owned: `plugins/sonicboom/backend/` is a standalone
Rust binary speaking the framed process protocol
(`tiktools-plugin-sdk`). It observes `live.ui-event` chat, applies the
speech policy (comment triggers, user eligibility, affordability,
replay dedup — a faithful port of `shared/tts/`), and POSTs eligible
lines to the configured SonicBoom server. Settings come from the same
host `settings.json` the UI edits through the broker; the host exports
the resolved `TIKTOOLS_PLUGIN_DATA_DIR` so backends always find it.

The declarative `speak` / `set-output-device` actions and the
voice/output option sources run in the host through the automation HTTP
engine — no process call, no foreign code. Host-mediated effects stay
out of backend reach by design: affordability is checked against
delivered point totals, but charging has no host call and is not
performed (documented limitation).

## Node set (uiVersion 1)

Closed and generic: `stack`, `card`, `text`, `form`, `select`, `range`,
`checkbox`, `button`, `list`, `status`, `separator`, `connection`. New
domains compose these primitives — the renderer registry
(`PluginNode.vue`) never gains per-domain section kinds. There is no
TTS node type in the grammar (Rust or TypeScript).

## Bindings

Intentionally limited — no expressions, no `eval`, no `new Function`:

- `settings.<dotted-path>` — plugin settings values (read/write,
  full-JSON values with nesting).
- `local.<name>` — page-local ephemeral state (never persisted).
- `source.<field>` — read-only selected value of the option source whose
  field segment matches (e.g. `source.voice`).

Prototype keys (`__proto__`, `prototype`) and non-identifier segments
fail closed.

## UI actions (allowlist)

`save-settings`, `plugin-action`, `refresh-source`, `test-connection`,
`open-media-picker`. There is no generic arbitrary-RPC action;
capability checks remain host-side.

## Backward compatibility

Existing schema-v3 manifests work unchanged. The legacy `"kind": "tts"`
section degrades to a neutral status note (`This panel moved to the
plugin view.`) — domain panels left the main frontend:

| v3 `kind`  | generic nodes                                  |
|------------|-----------------------------------------------|
| `text`     | `text`                                        |
| `form`     | `form` (same schema/uiHints fallback)         |
| `connection` | `connection`                                |
| `list`     | `list` (+ built-in refresh button)            |
| `tts`      | `status` placeholder                          |

`PluginPageView.vue` is the thin compatibility wrapper: it adapts the v3
descriptor once and renders the generic `<PluginPage>` with a single
`<PluginUiContext>`. When the snapshot's typed `ui` descriptor selects
webview mode for the open page, the view renders the inline frame
instead (`PluginInlinePage.vue` + `PluginFrame.vue` + the `postMessage`
shim), with a pop-out action for the native window; only a host that
cannot serve plugin assets (plain browser) falls back to the desktop
notice (`PluginWebviewPage.vue`). Visuals are verified by the Playwright
baselines in `tests/e2e/screenshots.spec.ts-snapshots/`.

## Lifecycle

Plugin windows never outlive their runtime: `plugin.stopped` (covers
disable) and `plugin.uninstalled` notifications close every window the
plugin owns. Shutdown drops the window manager with the app.

Native teardown is explicit and ordered: the child WebView drops first,
then (on Linux) a GDK display sync flushes its container destroy, and
only then does the parent window handle drop. (`Drop::drop` runs before
field destructors, so the order comes from explicit `Option::take`
calls, never from struct field order.) `CloseRequested` never destroys
handles re-entrantly: the entry is marked closing and torn down by a
deferred command outside window-event dispatch, so duplicate closes are
harmless. When the server already destroyed the parent (`Destroyed`
without `CloseRequested`), the dead handle is forgotten instead of
destroyed twice; the normal close path releases everything.

## Layout contract

The host tab gives the inline iframe the full area below the topbar
through a flex chain (`plg--webview` → `plg-scroll--webview` →
`plg-stack--webview` → `plg-frame`, all flex with `min-height: 0`, no
fixed heights). The plugin document is the single content scroller: its
`html`/`body` lock to the viewport while `#app` fills it and scrolls
vertically. The same stylesheet serves the inline iframe and the native
pop-out; long configuration screens scroll instead of clipping, and the
host never measures the frame DOM (the sandbox forbids it).

## Security properties

- No `v-html`, no `innerHTML` for plugin content, no `eval`/`new Function`.
- No plugin JS in the main WebView (verified: the main bundle carries
  no TTS/plugin markers; the UI suite builds the plugin separately).
- Manifest text renders as text and form controls only.
- Plugin paths canonicalized; navigation restricted to the owning
  plugin origin; secrets redacted; entry confined to `ui/`.
- Capability validation stays host-side; process plugins get no native
  handles; broker methods are an allowlist with ownership injection.
- UI actions are allowlisted data, never executable plugin code.
- Inline frames stay `sandbox="allow-scripts"`: no `allow-same-origin`
  (that would un-opaque the origin and widen the trust boundary).
  Opaque-origin resource loading is fixed with explicit CORS/CSP
  headers on the asset servers, never by relaxing the sandbox.
- Every new plugin UI feature needs a negative test proving a plugin
  cannot exceed its declared scope.

## Windows WebView2 protocol rewrite

Wry serves custom protocols on Windows through WebView2, which cannot
handle arbitrary schemes: `{scheme}://{rest}` reaches the WebView as
`http://{scheme}.{rest}`. Concretely, `tiktools-plugin://app/…`
becomes `http://tiktools-plugin.app/…`, and the main document
`tiktools://app/…` becomes `http://tiktools.app/…`. A
`tiktools-plugin:` CSP scheme source never matches the rewritten
`http:` URLs, so both are pinned explicitly and covered by
platform-independent unit tests (which assert the rewritten URL forms
without needing Windows):

- Plugin CSP resource directives list both `tiktools-plugin:` and
  `http://tiktools-plugin.app`; `frame-ancestors` lists the packaged
  host plus both rewritten main-app hosts.
- The plugin HTML meta CSP repeats the same sources (header and meta
  intersect — both must allow every load).
- The packaged main-app CSP frames both `tiktools-plugin:` and
  `http://tiktools-plugin.app`.
- Navigation allowlists accept both the custom-scheme and the rewritten
  `http:` forms, still confined to the owning plugin id.

## Known gaps

- Windows process-plugin entries: the loader resolves the manifest
  `entry` literally; the dev stager and release packager rewrite the
  entry with the `.exe` suffix on Windows (repo-wide convention).
- The packaged main-frontend CSP still lists `http://[::1]:*`, which
  Chromium rejects as an invalid CSP source (ignored). The plugin CSP
  omits it.
- Process backends cannot charge points (no host call); the SonicBoom
  observer checks affordability without deducting.
