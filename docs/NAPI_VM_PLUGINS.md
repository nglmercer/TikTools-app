# napi-vm plugins (`runtime: "napi-vm"`)

TypeScript-first plugins compiled to JavaScript ahead of time and executed
through `napi_vm::RustPluginHost` — napi-vm's pure-Rust interpreter. There is
no runtime transpiler and no Node.js dependency on this path.

## Package layout

```text
my-plugin/
  plugin.json
  src/index.ts
  dist/index.js
```

`entry` points at the compiled JavaScript (an ES module with a default
export). The host never reads `src/`; it ships for reviewability.

## Minimal manifest

```json
{
  "schemaVersion": 3,
  "id": "example.plugin",
  "name": "Example",
  "version": "1.0.0",
  "runtime": "napi-vm",
  "entry": "dist/index.js",
  "capabilities": [],
  "apiVersion": 1
}
```

TikTools parses and validates this file as the authority for its own
fields. napi-vm additionally reads its own compatibility subset (`name`,
`version`, `apiVersion`, `entry`) from the same file, which is why the
manifest carries `"apiVersion": 1` next to the TikTools fields. Until
napi-vm grows a spec-based load entry point, two constraints apply:

- `name` must also satisfy napi-vm's `/^[A-Za-z0-9][A-Za-z0-9._-]*$/`
  (TikTools display names with spaces fail closed at load).
- No napi-vm `permissions` object can be expressed: that key collides with
  TikTools' own string-list `permissions`, so omitting it keeps guest
  `fs`/`path`/capability requests deny-by-default. TikTools-owned host APIs
  will arrive as Rust capability modules granted per plugin instead.

## Guest interface

```ts
export interface TikToolsPlugin {
  onLoad?(context: PluginContext): void | Promise<void>;

  call(
    request: PluginCall,
    context: PluginContext,
  ): PluginCallResult | Promise<PluginCallResult>;

  onUnload?(context: PluginContext): unknown | Promise<unknown>;
}
```

The protocol is the unchanged `PluginCall` / `PluginCallResult` JSON
pair every runtime speaks: `PluginCall JSON -> guest call() ->
PluginCallResult JSON`. Promise results are awaited by the host. The
`context` passed to `call()` currently carries plugin identity
(`{pluginId, name, version}`) and is additive.

Current seam: napi-vm invokes `onLoad`/`onUnload` with its own context
shape (`{name, version}`, plus a `reason` on unload); only `call()`
receives the TikTools `PluginContext`. Write lifecycle hooks against
`name` (present in both) until the upstream context unifies.

Build with the repository TypeScript compiler, targeting plain ES modules:

```bash
node_modules/.bin/tsc -p my-plugin/tsconfig.json
```

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ES2020",
    "strict": true,
    "outDir": "dist",
    "rootDir": "src",
    "types": []
  },
  "include": ["src"]
}
```

## Threading and security

napi-vm's VM is `!Send`, so each plugin instance owns one dedicated VM
thread hosting its `RustPluginHost`; the `Send` loader handle only
forwards channel commands. Guest JavaScript runs inside the interpreter
with deny-by-default capabilities, which is why the runtime maps to
`PluginSecurityModel::Sandboxed`. Every host call gets a fresh
interpreter loop budget, so spinning guest code terminates with a
`RangeError` instead of wedging unload.

## Bundled native addons (napi-rs `.node`)

A `napi-vm` plugin may bundle napi-rs packages holding one `.node` binary
per platform. Guest code loads the package through `node:module`:

```ts
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { startListener, stopListener } = require('rdev-node');
```

```text
my-plugin/
  plugin.json
  dist/index.js
  node_modules/
    rdev-node/
      package.json
      index.d.ts
      node-rdev.win32-x64-msvc.node
      node-rdev.linux-x64-gnu.node
      node-rdev.linux-x64-musl.node
      node-rdev.darwin-arm64.node
```

No generated napi-rs loader is required and no TikTools-specific
wrapper: TikTools selects the exact host `.node` from the declared
package root, napi-vm internally allowlists and pins that file, and the
package name is exposed as a native CommonJS alias that `createRequire`
resolves directly to the binary.

### Manifest

Native code is never active without an explicit per-plugin declaration,
and only `napi-vm` manifests may carry one: `nativeAddons` on any other
runtime fails validation instead of being silently ignored. Each entry
names a package and its root — nothing else:

```json
{
  "nativeAddons": [
    {
      "package": "rdev-node",
      "root": "node_modules/rdev-node"
    }
  ]
}
```

Validation fails discovery on any deviation: the package must be a
valid bare name, the root must be a relative path with no `..` that
stays inside the plugin directory, and duplicate package names are
rejected. There are no per-target artifact maps and no manifest hashes.

Enabling native code additionally requires `"trust": "trusted"`. The
default napi-vm manifest is `Sandboxed`, so a native plugin visibly opts
out of the sandbox; `Sandboxed` and `Untrusted` manifests fail closed on
every install source. The single choke point is
`native_addons_allowed(manifest, source)` in the loader.

### Authorization

The host determines the canonical target — including the real Linux
libc (`linux-x64-gnu` vs `linux-x64-musl`, detected from the running
binary, never assumed) — and before `host.load` selects the exact host
binary from each declared package root, authorizes that one file, and
aliases it under the declared package name through napi-vm's
`configure_napi_addons`. napi-vm preflights the authorized file against
the host binary format at load. A missing package root, a missing host
binary, or several binaries matching the host all fail the load.
Undeclared `.node` files are refused even when they sit inside the
plugin directory, and a plugin with no declaration runs in the
pure-Rust VM with no native backend at all.

Native addons execute with TikTools process privileges and are not
sandboxed. Manifest and checksum validation is an authorization boundary,
not a sandbox: internally pinned hashes and `checksums.json` prove the
files are the ones that were packaged, never that the native code is
trustworthy. Paths outside the plugin root are never trusted, and the
host never runs npm lifecycle scripts or installs dependencies.

### Guest environment

Native selection is host-side, so guests receive no platform or libc
facts and need none: no `process`, no `fs`, no `child_process`, no
`process.report`, no `process.env`. The guest reaches the addon only
through the package alias.

### Declarative native libraries (`nativeLibs`)

`nativeAddons` authorizes loading; `nativeLibs` declares where the
bytes come from. Each entry pins one package version and names its
provider — `npm` by default, `github` for release assets:

```json
{
  "nativeLibs": [
    { "package": "rdev-node", "version": "1.0.5" },
    {
      "package": "other",
      "version": "0.3.0",
      "provider": "github",
      "repo": "owner/other",
      "tag": "v0.3.0",
      "binary": "other-native"
    }
  ]
}
```

Rules mirror `nativeAddons`: napi-vm manifests only, unique package
names, and versions are exact pins — ranges (`^1.0.0`, `latest`) are
rejected. A `github` entry requires `repo` (`owner/name`), the release
`tag`, and the napi `binary` stem used to build per-target asset names
(`index.js`, `index.d.ts`, `{binary}.{target}.node`); those keys are
rejected on `npm` entries so a half-migrated declaration fails instead
of fetching from the wrong source.

Content hashes live in the committed `native-libs.lock.json` next to
`plugin.json`, written by the staging tool's `--pin` mode:

```bash
cargo run -p tiktools-plugin-sdk --features providers \
  --bin tiktools-plugin-stage-native -- \
  --provider all --manifest my-plugin/plugin.json --pin
```

Pinning is the trust moment: the tool resolves each declaration,
downloads the artifacts, and records the npm tarball integrity plus
every `.node` SHA-256 (or every GitHub asset SHA-256). Review the
lockfile diff before committing it. Fetching replays the lockfile and
verifies every byte before staging; a declaration/lockfile mismatch, a
missing pin, or tampered bytes all fail the fetch.

### Staging native packages

Declared package roots are populated by the shared
`tiktools-plugin-stage-native` tool (`crates/tiktools-plugin-sdk`),
which validates the name, requires at least one `.node` artifact, and
copies `package.json` plus the artifacts into
`<plugin>/<root>`. Every flow stages through that one binary:

- **dev** (`scripts/dev-plugins.ts`): prefers a sibling source checkout
  (clone `rdev-node` beside the repository); when a checkout is missing
  but the example declares `nativeLibs` with a committed lockfile, the
  host binaries are fetched from their pinned provider instead.
- **packaged builds** (`scripts/build-plugin.ts`): same sibling-first
  rule on host builds. Declared and pinned libraries additionally
  unlock cross-target builds (`--target x86_64-pc-windows-msvc` from a
  Linux host serves that target's binary from the provider); without a
  lockfile, cross-target napi-vm builds are still rejected.
- **release** (`scripts/package-release.ts`): fetches every declared
  library for the release target from its pinned provider — no sibling
  checkout needed on CI.
- **install** (plugin installer): when an archive declares `nativeLibs`
  and ships its lockfile but lacks the install host's binary, the
  installer backfills exactly that binary from its pinned provider
  before completing. Complete trees never touch the network.

Provider fetches keep only the requested target's `.node` (plus the
package manifest and companion sources), so provider-built trees are
single-target; sibling checkouts stage whatever they carry.

### Packaging

`tiktools-plugin-pack` includes `node_modules/**` for `napi-vm` plugins
(and only for them) with no platform filtering and no binary stripping:
whatever the staging flow placed under the declared roots ships in the
archive, covered by `checksums.json` like every other file. A declared
`nativeLibs` entry additionally ships its `native-libs.lock.json` under
the same checksums, so the installer can replay the pins. Symlink
rejection and root containment still apply.

### Event loop and shutdown

The VM owner thread waits on its command channel with a short timeout
and pumps the napi-vm event loop after every call and while idle, so
native TSFN/async callbacks run without an arriving plugin request.

Shutdown order: stop accepting calls, run guest `onUnload`, dispose the
napi-vm plugin, shut down the native runtime, then exit the VM thread.
The host never terminates threads an addon detached itself: persistent
addon resources must expose their own stop/unsubscribe API, which the
guest calls from `onUnload` (the fixture calls `stopListener()` there).

## Real-world example

`examples/hotkey-napi-plugin` is the production native plugin: a
TypeScript guest driving `rdev-node` for global hotkeys (see
`docs/HOTKEYS_LINUX.md`). Build and test it with:

```bash
node_modules/.bin/tsc -p examples/hotkey-napi-plugin/tsconfig.json
bun test examples/hotkey-napi-plugin/
bun run scripts/build-plugin.ts -- --plugin hotkey-napi-plugin --host
```

## Reference fixture

`crates/tiktools-plugin-loader/tests/fixtures/napi-vm-echo` is the
minimal plugin (TS source plus committed tsc output), rebuilt with:

```bash
node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-echo/tsconfig.json
```

`crates/tiktools-plugin-loader/tests/napi_vm.rs` drives it through
discovery, start, `action`/`poll` calls, and stop, plus fail-closed
guests and missing-entry loads.

`crates/tiktools-plugin-loader/tests/fixtures/napi-vm-native` is the
native variant: a bundled `rdev-node` package (native-only manifest
plus platform binaries — see the fixture `README.md`) backed by the
napi-rs fixture crate in `tests/fixtures/native-tsfn`, rebuilt with:

```bash
node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-native/tsconfig.json
```

`crates/tiktools-plugin-loader/tests/napi_vm_native.rs` builds the
fixture `.node` offline, stages a multi-platform tree, and covers
exact-host loads, alias require, undeclared/missing/ambiguous/untrusted
rejections, GNU vs musl selection, TSFN delivery while idle, and clean
shutdown with reload.

## Follow-ups (upstream napi-vm)

- `RustPluginHost::load_spec(...)`: let TikTools pass its parsed
  manifest (id/entry) instead of sharing one `plugin.json` shape, which
  removes the `apiVersion` compat key, the `name` regex constraint, and
  the `permissions` key collision.
- Rust capability modules for TikTools-owned APIs (`events`, `points`,
  `storage`, `audio`, `http`), granted per plugin from the manifest's
  `capabilities` list.
- Hotkey host capability modules (Wayland portal chords, evdev raw
  input) restoring what the retired process plugin did guest-side;
  see `docs/HOTKEYS_LINUX.md`.
