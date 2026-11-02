# Plugins

Plugins are runtime packages. TikTools scans built-in, user, and development
directories when it starts; a plugin does not need to exist when TikTools is
compiled and adding one never requires recompiling the host.

## Package layout

```text
my-plugin/
  plugin.json
  native/my-plugin.dll       # or a standalone process / .wasm entry
  assets/
```

The manifest is versioned and explicit:

```json
{
  "schemaVersion": 2,
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "runtime": "native",
  "entry": "native/my-plugin.dll",
  "protocolVersion": 1,
  "abiVersion": 1,
  "permissions": ["audio.output"],
  "capabilities": ["audio.play"],
  "actionTypes": []
}
```

The same typed request/result boundary is used by the native, process, and
future WASM runtime adapters. The low-level contracts are defined in
`crates/tiktools-plugin-api`; the developer-facing Rust SDK is
`crates/tiktools-plugin-sdk`; runtime loading is isolated in
`crates/tiktools-plugin-loader`.

The process adapter wraps each `PluginCall` in the existing framed
`PluginRequest` envelope. Native ABI v1 receives the serialized `PluginCall`
directly because the ABI table already carries the protocol version. Both
adapters return the same typed `PluginCallResult` JSON bytes.

## Runtime kinds

### Native

Native libraries are loaded with `libloading`. They are trusted code and can
crash the application or access the operating system directly. The C ABI is
deliberately small: plugins exchange pointers, lengths, status values, and
serialized JSON bytes. Rust containers, trait objects, and async futures never
cross the boundary. ABI v1 does not pass manifest metadata to `create`, so the
SDK's native context is explicitly limited to an unknown identity and empty
declared capability/permission sets; the host manifest remains the policy
authority. Libraries remain loaded until shutdown; restart TikTools after
replacing one.

### Process

Process plugins are standalone executables. The host launches the declared
entry and exchanges length-prefixed JSON on stdin/stdout. A process crash is
contained at the process boundary, but the executable still has the normal OS
permissions of its user and is not a security sandbox. The process runtime
does not interpret or silently launch JavaScript source files; a JavaScript
plugin must be packaged as its own executable, use the `napi-vm` runtime
(`docs/NAPI_VM_PLUGINS.md`), or use the bounded `napi-vm` automation
surface.

The example at `examples/audio-process-plugin` demonstrates a complete process
plugin. It returns a typed `audio-play` intent; it does not open the file
itself.

### WASM

WASM is an optional execution sandbox boundary for untrusted or cross-platform
logic. The current workspace exposes the runtime slot without adding Wasmtime
or Extism to normal builds. WASI is not an automatic safety switch: it is the
capability-oriented system interface the host chooses to expose inside WASM.
No WASI imports means a very narrow sandbox; a single preopened plugin-data
directory is scoped filesystem access; broad filesystem or network imports
weaken that boundary. The future direction is the WASM Component Model with
WASI Preview 2 / WASI 0.2-style interfaces plus TikTools host capabilities.

The low-level API exposes this mapping as `PluginSecurityModel`: native is
`Trusted`, process is `Isolated`, and WASM and napi-vm are `Sandboxed`. The
existing manifest `trust` strings remain schema-v2 compatible metadata and
are not a substitute for the runtime boundary.

### napi-vm

TypeScript-first plugins compiled to JavaScript ahead of time and executed
through napi-vm's pure-Rust interpreter (`napi_vm::RustPluginHost`), with
no Node.js dependency and no runtime transpiler. Each instance owns one
dedicated VM thread; guests speak the unchanged `PluginCall` /
`PluginCallResult` JSON protocol through a `call(request, context)`
export. Host capabilities are deny-by-default; `.node` addons stay
available only through the explicitly trusted, allowlisted, SHA-256
verified native path. See `docs/NAPI_VM_PLUGINS.md` for the package
layout, guest interface, and security notes.

## Capabilities and permissions

Manifests may declare capabilities and permissions:

```json
{
  "permissions": ["http", "audio.output", "points.read", "points.write"],
  "capabilities": ["http.request", "audio.play"]
}
```

The core capability broker is the policy boundary for capabilities requested
through the host protocol. Native libraries and process executables are still
trusted code with the user's OS permissions; a manifest declaration is never
an OS sandbox for code that can call the operating system directly. Use the
WASM runtime with explicit WASI/host imports for genuinely untrusted code.

## Media and audio

`media.pick` is the public host API for selecting an existing file or
directory. It returns a JSON `MediaSelection` containing a canonical path and
metadata. `audio.play` accepts a `MediaFileRef`, validates it again immediately
before playback, and streams the original file. TikTools does not copy media
bytes into app data or a plugin directory.

Process, WASM, and `napi-vm` code uses a serializable intent instead of a native
audio handle:

```json
{
  "summary": "requested host audio playback",
  "intents": [{
    "type": "audio-play",
    "data": {
      "fileRef": {"path": "/music/alert.wav"},
      "volume": 0.8,
      "overlap": "restart"
    }
  }]
}
```

The host also accepts the legacy single-value `playAudio` shape at the
compatibility boundary for existing packages:

```json
{
  "playAudio": {
    "fileRef": {"path": "/music/alert.wav"},
    "volume": 0.8,
    "overlap": "restart"
  }
}
```

The host accepts that intent only from a plugin declaring both
`audio.play` and `audio.output`. It canonicalizes the path, restricts it to
supported audio types, checks the size, and then hands it to the native audio
provider. No plugin receives file bytes or a native file descriptor.

## Actions and settings

An action plugin can declare JSON action descriptors in `actionTypes`. The
Vue UI renders title, fields, JSON schema, and UI hints; it never imports
plugin code. A `settingsSchema` and optional `settingsUiHints` let the host
render plugin settings without allowing arbitrary DOM or script injection.
Numeric fields accept an optional `range` kind with `min`, `max`, and `step`
to render a slider instead of a number input.

Keep action identifiers stable. Protocol and ABI versions are independent:

- protocol version describes JSON messages and capabilities;
- ABI version describes native FFI compatibility.

The host rejects incompatible versions before loading a native library.

Plugin configuration pages use the same trust model: manifests describe UI
as data and the host renders it — never plugin HTML, JS, or components in
the main WebView. See [Plugin UI architecture](PLUGIN_UI_ARCHITECTURE.md).

## Card display: icon, tags, and descriptions

The Plugins view renders each package as a card with an icon tile, tag chips,
and a short description; the Details panel shows the long description when
the manifest provides one:

```json
{
  "description": "Short pitch, clamped to two lines on the card.",
  "longDescription": "Long **pitch** with `code`.\n\n- first\n- second",
  "icon": "voice",
  "tags": ["tts", "voice", "chat"]
}
```

- `icon` names a host-registry icon (`keyboard`, `audio`, `voice`, `chat`,
  `webhook`, ...); see `src/web/components/icons/icon-registry.ts`. Plugins
  can only name icons, never supply SVG: unknown names fall back to a
  heuristic icon derived from the plugin id, name, tags, and action tags.
- `tags` are lowercase slugs (12 at most) shown as chips; when absent, the
  host derives chips from the plugin's action tags. Malformed entries are
  dropped and never fail discovery.
- `description` (4096 chars) and `longDescription` (16 KB) render as
  markdown-lite: `**bold**`, `*italic*`, `` `code` ``, `[label](https://…)`
  and bare http(s) links, paragraphs, and `- ` lists. Everything else —
  HTML, scripts, `javascript:` URLs — renders as inert text, because the
  host builds text nodes instead of parsing markup. The card snippet uses
  the inline subset; Details renders the full block subset.

## Event triggers

A plugin can declare its own event types (global hotkeys, timers, file
watchers) in `eventTypes`. Declared types appear in the event picker next
to the built-in triggers, work with filters/cooldowns/actions like any
other trigger, and stop matching while the plugin is disabled or
unavailable.

```json
{
  "capabilities": ["events.publish"],
  "eventTypes": [
    {
      "type": "hotkey.pressed",
      "title": {"default": "Hotkey pressed"},
      "fields": [
        {"path": "event.data.key", "kind": "text"}
      ],
      "sample": {"key": "ctrl+k"}
    }
  ]
}
```

Rules:

- type names are dotted lowercase (`hotkey.pressed`, `timer.tick`);
- the `tiktok.`, `points.`, and `plugin.` namespaces stay host-owned, so a
  plugin can never shadow a built-in trigger or the `plugin.emit` channel;
- `title.default` is required; `description`, `fields` (text/number/boolean
  paths under `event.data.*` or `event.user.*`), and a `sample` payload are
  optional and bounded like every other descriptor.
- a field may declare fixed `options: [{value, label?}]` (128 at most).
  Option-backed fields render as a dropdown in behavior conditions instead
  of free text, plus a record button that fills the value from keys pressed
  while it is armed (Escape cancels).

Publishing works two ways. While any action of the plugin runs, its `emit`
response intents may name one of its own declared types instead of falling
back to `plugin.emit`:

```json
{ "emit": [{ "type": "hotkey.pressed", "data": {"key": "ctrl+k"} }] }
```

For spontaneous events the host polls every running plugin that declares
event types once per second with `{"type": "poll"}`. The plugin answers with
the events observed since the previous poll:

```json
{ "events": [{ "type": "hotkey.pressed", "data": {"key": "ctrl+k"} }] }
```

Both paths require the `events.publish` capability and only accept types
from the plugin own manifest. Payloads must be objects under 64 KB (16
events per poll at most); anything else is dropped with a warning. The host
stamps identity, timestamp, chain depth, and connection context, then runs
the normal matching pipeline including the depth guard, so a hotkey can
trigger actions but can never recurse without bound.

### Long-running preparation progress

Plugins that download or load a model can declare the `ui.progress`
capability. After an action starts the plugin, the host polls that running
plugin and recognizes the reserved `plugin.progress` event without exposing it
as an automation trigger:

```json
{
  "events": [{
    "type": "plugin.progress",
    "data": {
      "status": "downloading",
      "progress": 0.42,
      "message": "Downloading model: 42%."
    }
  }]
}
```

`status` is one of `downloading`, `loading`, `ready`, or `failed`;
`progress` is optional and must be between `0` and `1`; and `message` is
bounded by the host. TikTools forwards valid updates as a typed UI progress
notification. Progress-only plugins are not started by the global poll until
one of their actions explicitly starts them.

## Generic domain-event observers

Plugins that need a background view of host events declare the
`events.subscribe` capability and an optional `eventSubscriptions` list:

```json
{
  "capabilities": ["events.subscribe"],
  "eventSubscriptions": ["live.*", "points.*", "plugin.*"]
}
```

Subscriptions support `*`, exact topics such as `points.changed`, and
namespace wildcards such as `live.*`. The host sends the plugin protocol call
`{"type":"event","event":{"topic":"...","data":{...}}}` using the
stable serialized envelope; the internal Rust `DomainEvent` is never exposed.
Delivery is asynchronous with one bounded queue per plugin, so a slow, stopped,
disabled, or crashed plugin cannot block TikTools. Reliable-stream gaps are
reported as `event.gap` envelopes when the host can no longer provide a
complete sequence; lossy events may be dropped.

This observer API is transport-neutral and is separate from `processorTypes`.
Processors remain latency-sensitive, pre-filter, side-effect-free enrichers;
they must not be used to start servers or forward a background event stream.

## Event processors

A plugin can enrich existing host events before automation filters run by
declaring `processorTypes`. Processors are the third plugin contribution
alongside actions (side effects) and event sources (new triggers):

```text
Action    side effects, may run long, automation-triggered
Processor no side effects, strict latency deadline, fail open, pre-filter
Poll      background, periodic, publishes spontaneous plugin events
```

```json
{
  "capabilities": ["events.enrich"],
  "processorTypes": [
    {
      "id": "textintel.analyze",
      "title": {"default": "Text Intelligence"},
      "eventTypes": ["tiktok.chat"],
      "stage": "pre-filter",
      "timeoutMs": 250,
      "failureMode": "pass-through",
      "inputs": [
        {"path": "event.data.comment", "role": "message"},
        {"path": "event.user.nickname", "role": "display-name"}
      ]
    }
  ]
}
```

Rules:

- processor ids are plugin-scoped dotted lowercase (`textintel.analyze`);
- `eventTypes` subscribes to host events, so unlike published types it may
  name `tiktok.*`, `points.*`, and `plugin.*`; an empty list matches every
  event type;
- only `stage: "pre-filter"` and `failureMode: "pass-through"` exist;
- `timeoutMs` is optional (default 250ms, at most 5000ms) and must stay far
  below the action timeout: processors run on the live-message hot path.
- the host grants the first call of a process generation cold-start grace
  (10s floor): building the engine lazily on first enrich is expected, and
  `timeoutMs` only bounds steady-state calls afterwards. A timed-out process
  instance is retired so the next call respawns instead of wedging.

The host sends `{"type": "enrich", "request": {processorId, event, settings}}`
only to plugins that declare `processorTypes` and the `events.enrich`
capability, so existing plugins never observe the new call and keep protocol
v1 / ABI v1. `settings` carries the plugin's host-rendered settings object,
reloaded after every UI save, so processors stay configurable on every
runtime without file access. The plugin answers with constrained
`{annotations, views, logs}` JSON and no side-effect intents: `emit`,
`events`, `intents`, `playAudio`, `points`, `http`, and `storage` keys are
rejected, and oversized results (16 annotations/views at most, 32 KB total)
are rejected too.

Raw event fields are never modified. The host merges annotations under
`event.intel.providers.<pluginId>`, views under
`event.intel.providers.<pluginId>.views`, and additionally promotes the
host-defined stable keys `comment` and `user` to top-level `event.intel.*`
so filters and templates stay provider-neutral. Stable keys resolve by
first-writer-wins in selection order: the first processor whose contribution
validates against the stable contract owns the whole key, and unknown keys
never promote. Identity fields such as `event.user.uniqueId` are immutable;
spoken or normalized forms only appear under `intel`.

Views are canonical for spoken text: stable `tts` fields project only from
text views, never from annotation payloads. Emit skip-policy views
(`speak: false` with a `reason`) instead of dropping them, or TTS falls back
to the raw message and the policy is silently lost. Pronunciation evidence
travels separately from text selection (`TextPronunciation`), so it can never
overwrite the rendered text's language or confidence.

Failure always fails open: timeouts, crashes, invalid responses, missing
capabilities, open circuits, and full queues pass the original event through
and stamp `event.intel.processing.status = "degraded"`. Selection order is
higher priority first, then deterministic `(plugin id, processor id)`. Each
event runs at most four concurrent calls under their own deadlines, and all
events share sixteen global slots: events arriving while every slot is busy
shed load with an `Overloaded` outcome instead of queueing. Health and
circuit breaking are per processor (not per plugin), with the same
1s/2s/5s/10s/30s backoff as polling, so one failing processor never silences
its siblings. Use `test-processor` and `get-processor-status` over IPC — or
the Processors tab in the Plugins view — to preview one processor against a
sample live event or inspect health and latency metrics.

Eligibility is precomputed, not scanned per event: the host keeps a
contribution index plus an in-memory activation snapshot, rebuilt only on
install, uninstall, enable, and disable. The enrich hot path filters the
cached index by event type and never touches manifests or SQLite.

The editor's autocomplete follows the same availability through a
manifest-declared `autocomplete` section: the host validates each entry,
stamps it with namespaced identity, and ships it in the behavior snapshot
only while the plugin is installed, enabled, and available — so a disabled
plugin's suggestions vanish from the field picker and template autocomplete
on the next snapshot, with no plugin-specific code in the web layer. A
processor that promotes the shared stable keys declares those prefixes, and
may push extra suggestion fields for paths the static registry cannot know
(such as a provider-namespaced verdict):

```json
"autocomplete": [{
  "id": "stable-views",
  "prefixes": ["event.intel.comment.", "event.intel.user."],
  "fields": [{
    "path": "event.intel.providers.textintel.comment.moderation.blocked",
    "kind": "boolean",
    "label": {"default": "Moderation blocked"}
  }],
  "triggers": ["tiktok.chat"]
}]
```

Claims are restricted to the plugin's own enrichment namespace: the stable
prefixes `event.intel.comment.` / `event.intel.user.` plus the plugin's own
`event.intel.providers.<ownId>.` subtree. Core paths (`event.data.*`,
`event.user.*`), host-stamped `event.intel.processing.*`, and other plugins'
subtrees are rejected at merge, so a manifest can never hide suggestions it
does not own. Live `providers.<pluginId>.*` paths need no declaration: the
web layer attributes them by naming convention and gates them on the synced
plugin state (`src/automation/autocomplete-registry.ts`).

SDK sketch:

```rust
use tiktools_plugin_sdk::prelude::*;

#[derive(Default)]
struct TextProcessor;

impl Plugin for TextProcessor {
    fn enrich(
        &mut self,
        _context: &PluginContext,
        request: EventEnrichmentRequest,
    ) -> PluginResult<EventEnrichmentResult> {
        let comment = request
            .event
            .pointer("/data/comment")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let mut result = EventEnrichmentResult::default();
        result.annotations.insert(
            "comment".to_owned(),
            serde_json::json!({"normalized": comment.trim()}),
        );
        Ok(result)
    }
}

tiktools_process_plugin!(TextProcessor);
```

The reference implementation is
`examples/textintel-process-plugin/`: a crash-isolated process plugin that
analyzes comments and viewer names with a pinned offline textintel engine
(language, normalization, composition, Unicode, obfuscation, spam, rebus,
spoken/TTS views, nickname phonetics). Its latency bench
(`cargo bench --bench processor_latency` from the example directory) keeps
the 250ms deadline honest: cache-cold chat with a warm engine measures
~3-25ms with ~10ms at the 500-character cap on the reference machine, so
the default deadline holds roughly 10x headroom. The one-time engine build
itself (hundreds of milliseconds to seconds) is covered by cold-start
grace, not by `timeoutMs`. Re-run the bench on your target
hardware before raising `timeoutMs`, and never share the processor process
with slow model preparation: each plugin instance is served by a single
worker thread with a bounded queue, so a slow call delays (but never
starves) later calls to the same plugin, and requests that expire while
queued are discarded instead of executed.

## Installation

Plugins install from three kinds of sources through one transactional
installer (`crates/tiktools-plugin-installer`):

| Spec | Source | Signature policy |
| --- | --- | --- |
| `./acme.tts.plugin`, `file:<path>` | local archive | verified when a trusted publisher signed it, otherwise allowed only after explicit sideload consent |
| `https://host/acme.plugin` | web address (HTTPS; `http://localhost` for development) | same as local archives |
| `acme.tts`, `acme.tts@1.2.0`, `acme.tts@^1.2`, `official:acme.tts` | registry | must be signed by a publisher key the registry delegated |

Every install is reviewed first: `plugins.packages.inspect` resolves,
downloads, and verifies the package and returns a preview (permissions,
capabilities, security model, signature, size, compatibility, and the change
from the installed version) with a `consentToken`. `plugins.packages.install`
then runs a job that only succeeds when the token still matches the exact
bytes and access that were reviewed. The desktop review dialog, the CLI, and
agents use this flow for registry and URL specs. Local archive paths given to
`tiktools plugin install` or `--install-plugin` keep the direct developer path
(add `--review` to the CLI command to go through the review instead):

```bash
tiktools plugin inspect acme.tts
tiktools plugin install acme.tts --yes
tiktools plugin install ./acme.tts.plugin --replace
tiktools plugin install https://cdn.example/acme.tts.plugin --yes --allow-unsigned
tiktools plugin outdated
tiktools plugin update acme.tts --yes
tiktools plugin rollback acme.tts
tiktools plugin uninstall acme.tts --purge
cargo run -p tiktools-desktop --locked -- --install-plugin acme.tts
```

Installation extracts into `<plugins>/.staging`, rejects traversal, symlinks,
duplicate paths, and oversize archives, verifies every file against
`checksums.json` and the publisher signature, checks host compatibility
(`engines.tiktools`, protocol, ABI, target, runtime availability), stops the
running instance, swaps the package in atomically, and restarts it. A failure
at any stage leaves the previous package untouched. The replaced version is
kept under the plugin store for `rollback` (two versions by default).

Store layout:

```text
<root>/plugins/<id>/                       active package (discovery reads only this)
<root>/plugin-data/<id>/                   plugin data; kept on update, removed by uninstall --purge
<root>/plugin-store/receipts/<id>.json     origin, digest, signature, granted access, update policy
<root>/plugin-store/versions/<id>/<ver>/   retained versions for rollback
<root>/plugin-store/cache/sha256/...       content-addressed downloads (1 GB cap)
<root>/plugin-store/registries.json        configured registries and update defaults
```

`TIKTOOLS_PLUGIN_STORE_DIR` overrides the store directory. Packages that were
installed before receipts existed are adopted on startup. Registries, update
checks, auto-update policies, and advisories are described in
[Plugin registry protocol](PLUGIN_REGISTRY_PROTOCOL.md); building and
publishing packages is covered in [Plugin authoring](PLUGIN_AUTHORING.md).

### Publishing metadata

Optional manifest fields describe a package for registries and the review
dialog. Display fields are sanitized, identity and compatibility fields are
strict:

```json
{
  "publisher": "acme",
  "authors": [{ "name": "Acme Labs", "url": "https://acme.dev" }],
  "license": "MIT",
  "homepage": "https://acme.dev/tts",
  "repository": "https://github.com/acme/tts-plugin",
  "engines": { "tiktools": ">=0.4.0, <0.6.0" },
  "dependencies": { "acme.voices": "^2.0" },
  "categories": ["audio", "chat"],
  "changelog": "CHANGELOG.md"
}
```

A package whose `engines.tiktools` range excludes the running host is reported
unavailable at discovery and refused by the installer. `dependencies` are
validated and shown, not yet installed automatically.

## SDK guidance

Rust plugin authors should depend on `tiktools-plugin-sdk` and use
`tiktools_plugin_sdk::prelude::*`. The SDK owns framing, protocol validation,
typed calls/results, common errors, and response helpers. A process plugin can
be as small as:

```rust
#[derive(Default)]
struct ExamplePlugin;

impl Plugin for ExamplePlugin {}

tiktools_process_plugin!(ExamplePlugin);
```

Native plugins can use `tiktools_export_native_plugin!(ExamplePlugin);` for the
same reviewed FFI bridge. Do not depend on Wry, Winit, SQLite, or the TikTok
client. Keep payloads bounded and shutdown idempotent.

The author CLI `tiktools-plugin` (`crates/tiktools-plugin-devkit`) scaffolds,
validates, builds, packs, signs, and publishes packages without Bun or Node:

```bash
cargo install --git https://github.com/nglmercer/TikTools-app \
  --branch remake --package tiktools-plugin-devkit --locked
tiktools-plugin new acme.tts --template process-rust
tiktools-plugin build --release
tiktools-plugin pack --manifest plugin.json --entry target/release/plugin \
  --output dist/plugin.plugin
```

The packager (`crates/tiktools-plugin-package`) validates the manifest, stages
the declared entry plus `assets/`, `dist/`, `locales/`, the webview UI
directory, the changelog, and `node_modules/` for napi-vm, writes SHA-256
checksums, and creates the ZIP-based `.plugin` archive. The same crate reads
archives for the host, so author-side and host-side validation cannot drift.
`tiktools-plugin-pack` and `tiktools-plugin-stage-native` remain available as
aliases with their previous flags. TypeScript authors use
`@tiktools/plugin-sdk` (`packages/plugin-sdk`) for napi-vm guests and
`@tiktools/plugin-ui` (`packages/plugin-ui`) for webview pages. The
`tiktools-plugin-api` crate remains available for other languages and custom
runtime adapters.

## Target-aware builds

Compiled `native` and `process` plugins are platform-specific. The canonical
packaged target identifiers are:

```text
win32-x64-msvc
win32-arm64-msvc
linux-x64-gnu
linux-arm64-gnu
darwin-x64-darwin
darwin-arm64-darwin
```

Build with the shared script (defaults to the host target):

```bash
bun scripts/task.ts plugins --all
bun scripts/task.ts plugins --plugin audio-process-plugin
bun scripts/task.ts plugins --plugin audio-process-plugin --target x86_64-pc-windows-msvc
bun scripts/task.ts plugins --all --target aarch64-apple-darwin
```

The script passes `--target <rust-triple>` to Cargo, reads
`target/<rust-triple>/<profile>/` for the entry (`.exe` is derived from the
requested target, never from the build host), and names archives
`<id>-<version>-<plugin-target>.plugin`. `tiktools-plugin-pack --target`
injects exactly that target into the packaged `plugin.json` without rewriting
the source manifest:

```bash
tiktools-plugin-pack --manifest plugin.json --entry target/.../plugin \
  --target linux-x64-gnu --output dist/my-plugin-1.0.0-linux-x64-gnu.plugin
```

WASM plugins stay target-independent (`"targets": []`) and omit the target
suffix. The loader treats `targets == []` as portable and otherwise requires
the current platform target, reporting `plugin has no build for this
platform` when nothing matches. The Rust triple ↔ plugin target mapping
lives in `scripts/lib/plugin-targets.ts`; do not invent alternate names such
as `winx64` or `linux64`.
