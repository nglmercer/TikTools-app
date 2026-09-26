# Global hotkeys napi-vm example

A TypeScript TikTools plugin that publishes `hotkey.pressed` events for
global shortcuts and key sequences through the bundled `rdev-node`
native addon. Behaviors trigger on them like any other event: match
chords with `eq` on `event.data.key` or `event.data.modifiers`, phrases
with `contains` on `event.data.sequence`.

The guest watches the OS keyboard, tracks modifiers plus a rolling
8-key sequence, and answers the host `poll` call with everything observed
since the previous tick. It never sends keystrokes anywhere; it only reports
what was pressed. This example replaces the retired Rust process plugin;
unlike it, the guest cannot reach compositor-authorized portal chords,
raw evdev devices, or session detection, so on Wayland it only observes
what rdev sees (see `docs/HOTKEYS_LINUX.md`).

Layout:

```text
hotkey-napi-plugin/
  plugin.json          manifest (runtime napi-vm, trust trusted, nativeAddons, nativeLibs)
  native-libs.lock.json  committed content pins for the declared libraries
  tsconfig.json        ahead-of-time tsc build (no runtime transpiler)
  src/index.ts         plugin shell: listener lifecycle, poll, actions
  src/hotkeys.ts       pure ported core: key state, chords, bind parsing
  dist/index.js        committed tsc output (the host entry)
  node_modules/        staged at build/dev/pack time, never committed
    rdev-node/
      package.json
      index.d.ts
      node-rdev.{target}.node
```

Build the guest:

```bash
node_modules/.bin/tsc -p examples/hotkey-napi-plugin/tsconfig.json
```

Guest unit tests (the pure core is dependency-free, so these run with
no binding and no host):

```bash
bun test examples/hotkey-napi-plugin/
```

Stage the native package from a local rdev-node checkout (preferred
for local iteration on host builds):

```bash
cargo run -p tiktools-plugin-sdk --bin tiktools-plugin-stage-native -- \
  --package rdev-node \
  --source ../rdev-node \
  --plugin-dir examples/hotkey-napi-plugin \
  --root node_modules/rdev-node
```

Or fetch it from its pinned provider (works without the checkout, and
for any target — this is also what the release and cross-target builds
use):

```bash
cargo run -p tiktools-plugin-sdk --features providers --bin tiktools-plugin-stage-native -- \
  --provider all \
  --manifest examples/hotkey-napi-plugin/plugin.json \
  --plugin-dir examples/hotkey-napi-plugin \
  --target linux-x64-gnu \
  --overwrite
```

Re-pin the lockfile after bumping a `nativeLibs` version (review the
diff: it is the trust moment):

```bash
cargo run -p tiktools-plugin-sdk --features providers --bin tiktools-plugin-stage-native -- \
  --provider all \
  --manifest examples/hotkey-napi-plugin/plugin.json \
  --pin
```

After staging, `bun run prepare:dev-plugins` picks the example up into
`.dev-plugins/hotkeys`, and reloading the Plugins view (or restarting
TikTools) starts it. No host recompilation or plugin registration is
required.

The host accepts only the `events.publish` capability for these events,
only for the `hotkey.pressed` and `hotkey.status` types declared in this
manifest, and stamps identity, depth, and connection context itself.

Example filters on a `hotkey.pressed` event:

```text
event.data.key        eq        k
event.data.modifiers  eq        ctrl
event.data.sequence   contains  g o
```

Plugin actions (call from automations or the plugin inspector):

- `hotkey.bind` with config `{shortcuts, sequencesNeeded}`: watch the
  given chords and toggle raw input. Portal chords are acknowledged but
  inapplicable in the napi-vm build; raw input covers all keys.
- `hotkey.status`: overall backend status line.
- `hotkey.diagnostics`: backend entries, bindings, and queue counters.
