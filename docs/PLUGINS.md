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
permissions of its user and is not a security sandbox. The host does not
interpret or silently launch JavaScript source files; a JavaScript plugin must
be packaged as its own executable or use the bounded `napi-vm` automation
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
`Trusted`, process is `Isolated`, and WASM is `Sandboxed`. The existing
manifest `trust` strings remain schema-v2 compatible metadata and are not a
substitute for the runtime boundary.

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

## Installation

Create a `.plugin` archive containing `plugin.json` and the declared entry,
then install it through the Rust host:

```bash
cargo run -p tiktools-desktop -- --install-plugin ./my-plugin.plugin
cargo run -p tiktools-desktop -- --install-plugin ./my-plugin.plugin --replace
```

Installation uses a temporary directory, validates the manifest and optional
`checksums.json`, rejects traversal and symlink escapes, and atomically moves
the package into the user plugin directory. The usual runtime directory is
`%LOCALAPPDATA%/TikTools/plugins` on Windows,
`~/.local/share/TikTools/plugins` on Linux, and
`~/Library/Application Support/TikTools/plugins` on macOS.

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
client. Keep payloads bounded and shutdown idempotent. The low-level
packaging path is also available without Bun or Node:

```bash
cargo install --git https://github.com/nglmercer/TikTools-app \
  --branch remake --package tiktools-plugin-sdk \
  --features packager --bin tiktools-plugin-pack --locked
tiktools-plugin-pack --manifest plugin.json --entry target/release/plugin \
  --output dist/plugin.plugin
```

The packager validates the schema-v2 manifest, stages the declared entry and
standard asset directories, writes SHA-256 checksums, and creates the ZIP-based
`.plugin` archive. Project repositories only need to build their entry and
invoke this common CLI.
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
bun run build:plugins
bun run build:plugin -- --plugin audio-process-plugin
bun run build:plugin -- --plugin audio-process-plugin --target x86_64-pc-windows-msvc
bun run build:plugins -- --target aarch64-apple-darwin
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
