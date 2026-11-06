# Plugin authoring

`tiktools-plugin` is the author CLI. It scaffolds a package, validates it with
the same code the host uses, builds and packs it, signs it, and publishes it to
a static registry.

```bash
cargo install --git https://github.com/nglmercer/TikTools-app \
  --branch remake --package tiktools-plugin-devkit --locked
```

## 1. Create a package

```bash
tiktools-plugin new acme.tts --template process-rust
```

| Template | Runtime | Build |
| --- | --- | --- |
| `process-rust` | crash-isolated executable using `tiktools-plugin-sdk` | `cargo build` |
| `native-rust` | in-process library (`cdylib`, trusted code) | `cargo build` |
| `napi-vm-ts` | sandboxed TypeScript using `@tiktools/plugin-sdk` | `bun run build` |
| `declarative` | HTTP integration described in `plugin.json`, no code | none |
| `webview-ui` | declarative plugin with an isolated page using `@tiktools/plugin-ui` | `bun run build` |

Ids follow `publisher.name` (`acme.tts`); the publisher defaults to the part
before the first dot. Inside this repository pass `--sdk-path crates/tiktools-plugin-sdk`
to depend on the local SDK instead of the git dependency.

## 2. Validate

```bash
tiktools-plugin validate
tiktools-plugin validate --publish
# Review several existing plugins at once; exits nonzero if any is not ready.
tiktools-plugin validate-all plugins-example-tiktools/plugins/sonicboom examples/*
```

`validate` parses the manifest with the host parser and reports host
compatibility, publishing problems, and warnings. `--publish` fails unless the
package has a semver `version`, a `publisher` that owns the id namespace, a
`license`, `engines.tiktools`, at least one known category, a description, and
any declared changelog file. Grandfathered ids that do not start with the
publisher name pass with `--legacy-id`.

## 3. Build and pack

```bash
tiktools-plugin build --release
tiktools-plugin build --release --target linux-x64-gnu
tiktools-plugin build --release --all-targets
tiktools-plugin pack --entry path/to/built-entry --out-dir dist-plugin
```

`build` runs `cargo build` (Rust projects) or `bun run build` (projects with a
`package.json`), then packs one archive per target as
`<id>-<version>[-<target>].plugin`. Process and native plugins are
platform-specific; napi-vm, declarative, and WASM packages are portable unless
they bundle native addons. The archive contains `plugin.json`, the entry,
`assets/`, `dist/`, `locales/`, the webview UI directory, the changelog,
`node_modules/` for napi-vm, and `checksums.json`.

## 4. Try it locally

```bash
tiktools-plugin dev stage --into ~/tiktools-dev-plugins
tiktools-plugin dev watch --into ~/tiktools-dev-plugins
TIKTOOLS_DEV_PLUGINS_DIR=~/tiktools-dev-plugins cargo run -p tiktools-desktop --locked
```

`dev stage` packs, verifies, and unpacks the package into the development
plugin root; `dev watch` rebuilds and restages when sources change.
Development packages are never touched by the installer. To exercise the real
install flow instead, install the archive:

```bash
tiktools plugin install dist-plugin/acme.tts-0.1.0-win32-x64-msvc.plugin --review
tiktools plugin install dist-plugin/acme.tts-0.1.0-win32-x64-msvc.plugin --review --yes --allow-unsigned
```

The first command prints the review; the second installs after it.

## 5. Sign

```bash
tiktools-plugin keygen --id acme-1 --publisher acme --out ~/.tiktools/keys
tiktools-plugin sign dist-plugin/*.plugin --key ~/.tiktools/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.tts-0.1.0-win32-x64-msvc.plugin --trust ~/.tiktools/keys/acme-1.pub.json
```

`keygen` writes a private `*.key.json` (never commit it) and a public
`*.pub.json` to send to registry operators. `build` and `pack` accept
`--sign <key>` to sign in the same step. The signature covers
`checksums.json`, which covers every file in the package.

The official publisher uses namespace `tiktools`. Existing IDs are preserved
only through explicit root-signed scopes, such as `--scope 'tiktools.*'
--scope sonicboom.server` when generating a publisher key. A scope does not
replace backend publisher approval or independent registry-root delegation.
Use `validate-all --legacy-id` for these grandfathered manifests. The two
SonicBoom manifests currently share an ID and version but describe different
implementations; select `plugins-example-tiktools/plugins/sonicboom` as the release source, not both.
Generated private keys are owner-readable only on Unix.

## 6. Publish

Static registries are directories you serve over HTTPS. The registry operator
delegates your publisher key once, then you publish signed archives:

```bash
tiktools-plugin index delegate site/v1 --root-key keys/root-1.key.json --delegate acme-1.pub.json
tiktools-plugin publish dist-plugin/*.plugin --registry site/v1 --index-key keys/index-1.key.json
```

`publish` refuses packages that fail `validate --publish`, are unsigned, or are
signed by a key the registry has not delegated for that id. See
[Plugin registry protocol](PLUGIN_REGISTRY_PROTOCOL.md) for the format, the
trust model, yanking, advisories, and revocation.

## TypeScript guests (`napi-vm`)

```ts
import { actionConfig, actionResult, definePlugin, emit } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    const message = String(actionConfig(request).message ?? 'Hello');
    return actionResult(message, { intents: [emit('acme.greeted', { message })] });
  },
});
```

The manifest `name` must match `/^[A-Za-z0-9][A-Za-z0-9._-]*$/` and the
manifest carries `"apiVersion": 1` for napi-vm. See
[napi-vm plugins](NAPI_VM_PLUGINS.md) for the guest environment and native
addons.

## Webview pages

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
broker.subscribe('plugin.settings-changed', () => location.reload());
```

The page runs in an isolated WebView and reaches the host only through the
broker, scoped to its own plugin. See
[Plugin UI architecture](PLUGIN_UI_ARCHITECTURE.md).

## Manifest schema

`packages/plugin-sdk/plugin.schema.json` (`@tiktools/plugin-sdk/plugin.schema.json`)
gives editors completion for `plugin.json`. The host parser stays the
authority; a test keeps the schema's keys in step with it.
