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

`.node` addons stay off the normal path: the loader's `napi-vm-node-api`
cargo feature (disabled by default) wires napi-vm's `node-api-host`
backend for real native workloads only. Native addons are trusted code,
never sandboxed — each one must be explicitly allowlisted with a SHA-256
digest from trusted host metadata.

## Reference fixture

`crates/tiktools-plugin-loader/tests/fixtures/napi-vm-echo` is the
minimal plugin (TS source plus committed tsc output), rebuilt with:

```bash
node_modules/.bin/tsc -p crates/tiktools-plugin-loader/tests/fixtures/napi-vm-echo/tsconfig.json
```

`crates/tiktools-plugin-loader/tests/napi_vm.rs` drives it through
discovery, start, `action`/`poll` calls, and stop, plus fail-closed
guests and missing-entry loads.

## Follow-ups (upstream napi-vm)

- `RustPluginHost::load_spec(...)`: let TikTools pass its parsed
  manifest (id/entry) instead of sharing one `plugin.json` shape, which
  removes the `apiVersion` compat key, the `name` regex constraint, and
  the `permissions` key collision.
- `RustLoadedPlugin::call_json(...)`: invoke the guest `call()` export
  (Promise-aware) without TikTools reaching the guest handle through
  `interpreter_mut()`.
- Rust capability modules for TikTools-owned APIs (`events`, `points`,
  `storage`, `audio`, `http`), granted per plugin from the manifest's
  `capabilities` list.
