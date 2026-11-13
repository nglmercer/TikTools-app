# Plugin Package Manager & SDK Plan

Status: M0–M5 implemented; M6 and the official registry bootstrap are open. Extends
[Plugins](PLUGINS.md) and [Plugin UI architecture](PLUGIN_UI_ARCHITECTURE.md).

## 1. Context

Plugins will be distributed from the cloud. Today TikTools can only install a
local `.plugin` archive (`--install-plugin`, `plugins.install {path}`), with no
notion of where a package came from, which versions exist, whether it was
updated, who published it, or how to go back. Authoring tools are spread
across two SDK binaries and in-repo Bun scripts, and plugin UIs copy the broker
client by hand.

Goal: one **plugin package manager** that serves every current and future
plugin (native, process, napi-vm, declarative, future WASM), plus an **SDK** —
a set of reusable libraries and one author CLI — designed so it can later be
published as standalone crates and npm packages and back a cloud plugin
library (catalog/store).

Design principles:

- **Library first.** All package-management logic lives in crates with no
  dependency on `AppCore`, Wry, Winit, or SQLite. Desktop, CLI, Termux, a
  future registry service, and third-party tools consume the same code.
- **Runtime-agnostic.** The installer never branches on runtime kind except
  through a `HostCapabilities` value. Adding a runtime never touches it.
- **Source-agnostic.** File, URL, registry, bundled, and development packages
  flow through one `PackageSource` trait and one transaction pipeline.
- **Static-first cloud.** The registry protocol works from plain files on a
  CDN (GitHub Pages / R2 / S3); a dynamic service may serve the same protocol
  later without client changes.
- **Verify before trust.** Digest → signature → manifest identity → policy →
  user consent, every time, for every source.
- **Typed everything.** Errors, progress, and results are typed enums shared
  with the UI through generated contracts. No substring classification.

## 2. Current state

| Area | Exists today | Gap |
| --- | --- | --- |
| Package format | ZIP `.plugin`, `plugin.json`, optional `checksums.json`, safe extraction, limits (`crates/tiktools-plugin-loader/src/installer.rs`) | `signature.json` is rejected; extraction and packing logic duplicated between loader and `tiktools-plugin-pack` |
| Install | `PluginInstaller::install` + atomic swap, `AppCore::install_plugin` (`crates/tiktools-core/src/lib.rs`) with a global `install_lock` | No source, no receipt, no rollback, synchronous, no progress, no cancel |
| Errors | `classify_plugin_install_error` substring matching (`crates/tiktools-core/src/ipc/messages/install.rs`) | Fragile; UI cannot explain failures |
| Versions | Free-form `version` string | No semver, no host-version compatibility (`engines`), no update concept |
| Metadata | name, description, icon, tags, longDescription | No publisher, license, homepage, repository, dependencies |
| Remote | `nativeLibs` npm/GitHub fetch at install time | No registry, no catalog, no URL installs |
| Control API | `plugins.install/uninstall/enable/disable/start/stop/install.set` | No inspect, update, rollback, jobs, registry, search |
| UI | Installed/Available catalog, pick-file install, replace banner | No browse, review/consent dialog, updates, versions, progress |
| Authoring | `tiktools-plugin-pack`, `tiktools-plugin-stage-native`, `scripts/build-plugin.ts`, Rust SDK | No scaffolding, validation CLI, signing, publishing; no TS guest SDK; broker client copied per plugin (`plugins-example-tiktools/plugins/sonicboom/ui/src/broker.ts`) |

What stays as is: manifest parsing and validation (`tiktools-plugin-api`),
runtime discovery and loading (`tiktools-plugin-loader`), the runtime-root
precedence (development > user > builtin), plugin data directories, and the
`behavior_plugins` installed/enabled activation state.

## 3. Target architecture

```text
host apps        tiktools-core ─► control-api ─► client · cli · desktop · web
                      │
manage on disk   tiktools-plugin-installer   sources · transactions · receipts · updates · jobs
                      │
cloud            tiktools-plugin-registry    protocol · client · resolver · downloader
                      │
format           tiktools-plugin-package     reader · writer · checksums · signatures · trust
                      │
contracts        tiktools-plugin-api         manifest · protocol · versions · targets

run plugins      tiktools-plugin-loader      (api)            discovery + runtimes only
write plugins    tiktools-plugin-sdk         (api)            guest-side Rust SDK
author tooling   tiktools-plugin-devkit      (api, package, registry)  `tiktools-plugin` CLI
npm              @tiktools/plugin-sdk        guest types, definePlugin(), plugin.schema.json
                 @tiktools/plugin-ui         webview broker client
```

| Crate / package | Kind | Responsibility |
| --- | --- | --- |
| `tiktools-plugin-api` | existing | Manifest + protocol contracts. Gains `PluginVersion`, `VersionReq`, `PluginTarget`, `PackageId` and the new manifest metadata. |
| `tiktools-plugin-package` | **new** | The `.plugin` format: `PackageReader` (safe streaming extraction, limits, manifest inspection), `PackageWriter` (pack), `Checksums`, detached ed25519 `Signature`, `KeyId`, `TrustStore`. Single implementation for host and author tools. |
| `tiktools-plugin-registry` | **new** | Registry protocol v1 models, `RegistryClient` (sparse index, ETag cache, mirrors), `Resolver` (semver, engines, targets, channels, yanks), `Downloader` (streaming, resume, content-addressed cache). Feature `index-writer` builds static registries. |
| `tiktools-plugin-installer` | **new** | Install transactions, `PackageSource` implementations, receipts, retained versions, rollback, update checking, policies, jobs and progress, typed `InstallError`. Takes over `PluginInstaller` and `backfill_native_libs` from the loader. |
| `tiktools-plugin-loader` | existing | Discovery + runtimes only. The `plugin-install` feature and `installer.rs` move out. Discovery skips dot-prefixed directories. |
| `tiktools-plugin-sdk` | existing | Guest-side Rust SDK. The `packager` feature and both binaries move to the devkit; `providers` stays and is consumed by the installer. |
| `tiktools-plugin-devkit` | **new** | One author CLI, `tiktools-plugin`: `new`, `validate`, `build`, `pack`, `stage-native`, `keygen`, `sign`, `verify`, `index`, `publish`, `dev`. |
| `packages/plugin-sdk` | **new npm** | `@tiktools/plugin-sdk`: generated protocol/manifest types, `definePlugin()` for napi-vm guests, `plugin.schema.json`. |
| `packages/plugin-ui` | **new npm** | `@tiktools/plugin-ui`: versioned broker client for webview plugin UIs, replacing copied `broker.ts` files. |

Dependency rules (enforced by `Cargo.toml`, checked in review):
`api ← package ← registry ← installer ← core`. The devkit may depend on
`api`, `package`, `registry`. Nothing below `core` may depend on `core`,
`control-api`, Wry, Winit, or `rusqlite`. Every new crate builds for Termux.

Publishability: new crates use `version.workspace` + path dependencies with
explicit versions, keep `publish = false` until the API is frozen, and run
`cargo semver-checks` in CI once published. npm packages are Bun workspaces
(`"workspaces": ["packages/*", "plugins/*/ui"]` in the root `package.json`).

## 4. Core vocabulary

```rust
pub struct PackageId(String);
pub struct PluginVersion(semver::Version);
pub struct VersionReq(semver::VersionReq);

pub enum PackageSpec {
    Registry { registry: Option<String>, id: PackageId, req: VersionReq },
    Url(url::Url),
    File(PathBuf),
    Link(PathBuf),
}

pub enum PackageOrigin {
    Registry { name: String, url: String },
    Url { url: String },
    File,
    Builtin,
    Development,
    Adopted,
}

pub struct HostCompat {
    pub host_version: semver::Version,
    pub protocol_version: u32,
    pub abi_version: u32,
    pub schema_versions: RangeInclusive<u32>,
    pub target: PluginTarget,
    pub runtimes: BTreeSet<PluginRuntimeKind>,
}
```

Spec grammar accepted everywhere (CLI, API, deep links):

```text
acme.tts                    latest stable from the registry that owns `acme.*`
acme.tts@1.2.0              exact
acme.tts@^1.2               range
official:acme.tts@~1.2      explicit registry
./dist/acme.tts.plugin      local archive (also file:<path>)
https://host/acme.plugin    URL, HTTPS only (http://localhost allowed for dev)
link:../my-plugin           development link (debug builds / dev root only)
```

`HostCompat` is built once by core from `CARGO_PKG_VERSION`,
`TIKTOOLS_PLUGIN_PROTOCOL_VERSION`, `TIKTOOLS_PLUGIN_ABI_VERSION`, the
supported schema range, the current target, and the runtimes actually
available on this platform (Android drops `native` and `.node` addons, builds
without `wasm-plugins` drop `wasm`).

## 5. Manifest additions

All additions are optional in schema v3, validated when present, ignored by
older hosts (the parser is tolerant of unknown keys). The **registry publish
policy** — not the parser — makes some of them mandatory.

```json
{
  "$schema": "https://plugins.tiktools.app/schema/plugin.v3.json",
  "schemaVersion": 3,
  "id": "acme.tts",
  "version": "1.2.0",
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

| Field | Rule | Required to publish |
| --- | --- | --- |
| `version` | Any string locally; strict semver 2.0 to publish | yes (semver) |
| `publisher` | `[a-z][a-z0-9-]{1,38}`; must equal the id namespace for new registry ids | yes |
| `authors` | ≤16 entries, https URLs only | no |
| `license` | SPDX expression | yes |
| `homepage`, `repository` | https URLs | repository: recommended |
| `engines.tiktools` | semver range against the host version; absent = any | yes |
| `dependencies` | id → range; parsed and reported now, resolved in M6 | no |
| `categories` | fixed host list (`audio`, `chat`, `games`, `overlays`, `moderation`, `integrations`, `utilities`, `ai`) | ≥1 |
| `changelog` | safe relative path to a markdown file inside the package | no |

Identity rules: new registry packages use `publisher.name` ids; `tiktools.*`
is reserved for official packages. Existing ids (`sonicboom.server`,
`l4d2.interactive`, `textintel`, ...) are grandfathered by an explicit
allowlist owned by the official publisher, so persisted settings, automations
and activation state never break.

## 6. Package format v2 and trust model

The archive stays a ZIP `.plugin`. Changes:

- `checksums.json` (SHA-256 of every file) becomes mandatory for registry
  packages and for any signed package.
- `signature.json` is a detached ed25519 signature over the **exact bytes** of
  `checksums.json` (which covers `plugin.json`), so no JSON canonicalization
  is ever needed:

  ```json
  { "version": 1, "algorithm": "ed25519", "keyId": "acme-2026a", "publisher": "acme", "signature": "<base64>" }
  ```

- The archive digest (SHA-256 of the `.plugin` bytes) is published by the
  registry and verified before the archive is opened.

Trust model v1 (TUF-inspired, two levels, detached `.sig` files everywhere):

```text
root keys (offline; official ones compiled into the host,
           custom registries pinned by fingerprint when added)
   └─ sign keys.json ── delegates role keys with validity windows:
         index            (CI key) signs config.json, index docs, catalog pages
         publisher:<name> signs packages of that publisher
```

Verification order inside `PackageReader`: archive digest → `signature.json`
against a publisher key from the trust store → `checksums.json` → each file
digest while extracting → manifest identity (`id`, `version`, `publisher`
must match the signature and the registry release entry).

Source policies (`SignaturePolicy`):

| Origin | Default policy |
| --- | --- |
| Registry | signature **required**, publisher must own the id namespace |
| URL / File | verified when signed by a known publisher; otherwise **sideload**: allowed only with explicit consent (`--allow-unsigned`, UI checkbox), receipt marked unverified |
| Builtin | trusted (shipped with the host) |
| Development / Link | trusted, never updated, debug builds or explicit dev root only |

Revocation: `keys.json` carries revoked key ids; `config.json` carries revoked
package digests. Both are re-read on every registry refresh; an installed
package hit by revocation is disabled and flagged in the UI.

## 7. Registry protocol v1

Static files under a versioned base URL. Every JSON document has a sibling
`.sig`. Additive fields are ignored by clients; breaking changes bump `/v2/`.

```text
<base>/v1/config.json                 protocol, name, mirrors, optional api, revoked digests, expires
<base>/v1/keys.json                   delegated role keys (root-signed)
<base>/v1/index/<aa>/<bb>/<id>.json   sparse per-package document (sharded by id prefix)
<base>/v1/catalog/page-<n>.json       paged summaries for browsing (id, name, icon, tags, latest, categories)
<base>/v1/catalog/categories/<c>.json
<base>/v1/assets/<id>/...             icons, screenshots (served as images, never HTML)
<artifact url>                        .plugin files (any host: Releases, R2, S3)
```

Package document:

```json
{
  "id": "acme.tts",
  "name": "Acme TTS",
  "publisher": "acme",
  "description": "…",
  "icon": "voice",
  "categories": ["audio"],
  "tags": ["tts"],
  "homepage": "https://acme.dev/tts",
  "screenshots": ["assets/acme.tts/1.png"],
  "dist-tags": { "stable": "1.2.0", "beta": "1.3.0-beta.2" },
  "releases": [{
    "version": "1.2.0",
    "channel": "stable",
    "publishedAt": "2026-11-01T12:00:00Z",
    "yanked": false,
    "advisory": null,
    "runtime": "process",
    "engines": { "tiktools": ">=0.4.0" },
    "schemaVersion": 3, "protocolVersion": 1, "abiVersion": null,
    "permissions": ["audio.output"],
    "capabilities": ["audio.play"],
    "dependencies": {},
    "changelog": "…markdown-lite…",
    "artifacts": [
      { "target": "win32-x64-msvc", "url": "https://…/acme.tts-1.2.0-win32-x64-msvc.plugin", "size": 4194304, "sha256": "…", "keyId": "acme-2026a" },
      { "target": "linux-x64-gnu",  "url": "…", "size": 4012345, "sha256": "…", "keyId": "acme-2026a" }
    ]
  }]
}
```

Release metadata duplicates the manifest fields the resolver needs, so the
client never downloads an archive to decide compatibility; the installer
re-checks them against the real manifest after download.

Scalability:

- Sparse index: installing or checking one plugin costs one small cached GET,
  independent of catalog size. `ETag` / `If-None-Match`, `Cache-Control`.
- Catalog pages for browsing; when `config.json` advertises `api`, the client
  uses `GET {api}/search?q=&category=&page=` instead (dynamic service,
  download counts, ratings) and falls back to static pages otherwise.
- Multiple registries with priority and **namespace scoping** (a private
  registry claims `acme.*`; ids resolve only against their owning registry,
  preventing dependency confusion). Mirrors listed in `config.json`.
- Artifacts can live anywhere; only digests and signatures are trusted.

## 8. Resolver

`Resolver::resolve(spec, &HostCompat, &InstalledSet, &Policy) -> Result<ResolvedRelease, ResolveError>`

1. Pick the registry (explicit, namespace scope, or priority order).
2. Fetch the package document (cache + ETag).
3. Filter releases: not yanked (unless an exact version is requested with
   `allowYanked`), channel allowed (`stable` default, `beta` opt-in per plugin),
   `engines.tiktools` matches the host, protocol/ABI/schema supported, runtime
   available on this platform, an artifact exists for the current target or is
   portable (`target: "any"`).
4. Pick the highest matching version; refuse downgrades unless requested.
5. Report a typed rejection list when nothing matches, e.g.
   `requires TikTools >=0.5.0`, `no build for linux-arm64-gnu`, `only beta releases`.

Dependencies (M6): resolve the closure with the same filter, install in
topological order inside one multi-package transaction, refuse cycles and
conflicting ranges. Until then `dependencies` are checked for presence and
reported as warnings.

## 9. Install transaction pipeline

```text
resolve → download → verify → inspect → policy/consent → extract → stage-native
        → activate (stop · swap · keep previous) → record receipt → rescan/restart → publish events
```

| Stage | Behavior |
| --- | --- |
| resolve | Section 8. File/URL specs skip registry lookup. |
| download | Streaming `reqwest`, byte progress, HTTP `Range` resume, size cap enforced while streaming, content-addressed cache hit skips the network. |
| verify | Archive digest, signature, checksums (Section 6). |
| inspect | Manifest identity must match the request and the release entry; host compatibility re-checked from the real manifest. |
| policy/consent | Signature policy, namespace ownership, downgrade guard, **consent token** check (below). |
| extract | `PackageReader` into private staging (existing traversal/symlink/zip-bomb defenses). |
| stage-native | Existing `nativeLibs` backfill, moved from the loader. |
| activate | Per-id lock; stop the running instance; atomic rename; move the previous version into `plugin-store/versions/<id>/<version>`; restart if it was running. |
| record | Write the receipt; prune retained versions beyond the limit (default 2). |
| rescan | `PluginManager::scan`, processor index rebuild, automation snapshot refresh, domain events. |

Failures attempt to restore the previous state. If restoration fails, staging
is retained because it may contain the only remaining package copy; startup
cleanup removes only empty staging directories. A cooperative cross-process
lease serializes package mutations and cleanup for updated binaries. Older
binaries do not honor the lease, and path operations do not yet defend against
malicious concurrent directory replacement. A durable package-operation
journal remains necessary for complete crash recovery. A `CancellationToken`
is checked between stages and while streaming.

Consent: `plugins.inspect` returns a `PackagePreview` and a
`consentToken = sha256(id, version, archiveDigest, runtime, sorted permissions, sorted capabilities)`.
`plugins.install` / `plugins.update` must carry a token matching what is
actually installed; otherwise they fail with `consent-required` and the fresh
preview. The user therefore approves exactly the bytes and permissions that
get installed (no TOCTOU). Updates that add permissions or capabilities are
never auto-applied.

Concurrency: one transaction per plugin id (keyed lock), a global download
semaphore (3), and a short global critical section only around
swap + rescan. The current global `install_lock` disappears.

## 10. Receipts, store layout, updates, rollback

```text
<root>/plugins/<id>/                         active package (discovery unchanged)
<root>/plugin-data/<id>/                     plugin data; kept on update, removed only on uninstall --purge
<root>/plugin-store/receipts/<id>.json       one receipt per installed package
<root>/plugin-store/versions/<id>/<ver>/     retained previous versions
<root>/plugin-store/cache/sha256/<aa>/<digest>.plugin   content-addressed downloads (LRU, 1 GB cap)
<root>/plugin-store/registries/<name>/       cached index docs, ETags, keys
<root>/plugin-store/registries.json          configured registries and global policy
```

`AppPaths` gains `plugin_store`; `TIKTOOLS_PLUGIN_STORE_DIR` overrides it.

```json
{
  "id": "acme.tts",
  "version": "1.2.0",
  "origin": { "kind": "registry", "name": "official", "url": "https://plugins.tiktools.app/v1" },
  "archiveDigest": "sha256:…",
  "signature": { "verified": true, "publisher": "acme", "keyId": "acme-2026a" },
  "grantedPermissions": ["audio.output"],
  "grantedCapabilities": ["audio.play"],
  "installedAt": "2026-11-01T12:00:00Z",
  "updatedAt": "2026-11-01T12:00:00Z",
  "policy": { "channel": "stable", "autoUpdate": "notify", "pinned": false },
  "retained": ["1.1.3"]
}
```

`ReceiptStore` is a trait; the crate ships `JsonReceiptStore` (works headless
and on Termux). Receipts are separate from `behavior_plugins`, which keeps
owning logical installed/enabled activation.

Adoption: on first start after upgrade, every user-root package without a
receipt gets one with `origin: adopted`, version from its manifest, no
signature. Adopted packages are offered registry updates only when the
official registry publishes the same id under the grandfather allowlist and
the user links it explicitly.

Updates:

- `UpdateChecker` runs on startup (after a delay) and every 12 h, with
  backoff on failures; Termux/CLI runs it on demand only.
- Produces `UpdateInfo { id, current, latest, channel, kind: patch|minor|major, permissionDiff, advisory }`
  and publishes `plugin.updates-available`.
- Per-plugin policy `autoUpdate: off | notify | patch | minor` (default
  `notify`), `pinned`, `channel`. Auto-apply never crosses majors and never
  applies permission changes.
- Advisories: `critical` advisories on an installed version surface a
  blocking banner and, if the global policy allows, disable the plugin.

Rollback: `plugins.rollback {pluginId, version?}` swaps a retained version
back through the same transaction (receipt `origin` preserved, consent token
from the retained manifest).

Builtin packages: installing a registry version of a bundled id lands in the
user root and shadows the builtin copy (existing root precedence);
uninstalling it reveals the bundled version again.

## 11. Jobs, progress, typed errors

Every mutating operation is a job:

```json
{
  "id": "job_01J…",
  "kind": "install",
  "pluginId": "acme.tts",
  "fromVersion": null,
  "toVersion": "1.2.0",
  "stage": "download",
  "progress": { "done": 1048576, "total": 4194304 },
  "status": "running",
  "error": null
}
```

- `kind`: `install | update | uninstall | rollback | refresh`.
- `status`: `queued | running | succeeded | failed | cancelled`.
- Progress pushed as `DomainEvent::PluginJob` (topic `plugin.job`), throttled
  to ≥100 ms per job, final state always delivered.
- Jobs are kept in memory (last 50) for `plugins.jobs.list`.

`InstallError { code, message, details }` with
`InstallErrorCode = already-installed | not-found | no-compatible-release |
download-failed | digest-mismatch | signature-required | signature-invalid |
untrusted-publisher | revoked | invalid-package | incompatible |
consent-required | downgrade-blocked | busy | cancelled | io-error`.
`details` carries structured data (resolver rejections, permission diff,
expected vs actual digest). Codes map to `ApiError` data and to i18n keys in
the UI. `classify_plugin_install_error` and its tests are deleted.

## 12. Control API, client, CLI

New or changed JSON-RPC methods (all registered with `register_typed`, so
`rpc.discover`, `rpc.schema`, parity tests and generated TS contracts follow):

| Method | Params → Result | Destructive |
| --- | --- | --- |
| `plugins.inspect` | `{spec}` → `PackagePreview {manifest summary, origin, signature, permissions, capabilities, securityModel, size, compat, change: install\|update\|reinstall\|downgrade, permissionDiff, consentToken}` | no |
| `plugins.install` | `{spec, consentToken, enable?, wait?}` → `{job}` or final result; legacy `{path, replaceExisting}` accepted | yes |
| `plugins.update` | `{pluginId, version?, consentToken?}` → `{job}` | yes |
| `plugins.updates.check` / `plugins.updates.list` | `{}` → `UpdateInfo[]` | no |
| `plugins.rollback` | `{pluginId, version?}` → `{job}` | yes |
| `plugins.versions` | `{pluginId}` → installed, retained, available | no |
| `plugins.uninstall` | `{pluginId, purgeData?}` → `{job}` | yes |
| `plugins.policy.set` | `{pluginId, autoUpdate?, pinned?, channel?}` | yes |
| `plugins.jobs.list` / `.get` / `.cancel` | job queries / cancel | cancel: yes |
| `registry.list` / `.add` / `.remove` / `.refresh` | registry sources; `add` returns the root key fingerprint for confirmation | add/remove: yes |
| `registry.search` | `{query, category?, page?}` → paged summaries | no |
| `registry.package` | `{id, registry?}` → package document (sanitized) | no |

New module `crates/tiktools-control-api/src/modules/registry.rs`; plugin
methods stay in `modules/plugins.rs`. `tiktools-client` gets typed wrappers.

End-user CLI (`crates/tiktools-cli/src/commands/plugins.rs`):

```bash
tiktools plugin search tts
tiktools plugin inspect acme.tts@^1.2
tiktools plugin install acme.tts --yes
tiktools plugin install ./acme.tts.plugin --allow-unsigned
tiktools plugin outdated
tiktools plugin update --all
tiktools plugin rollback acme.tts
tiktools plugin versions acme.tts
tiktools plugin pin acme.tts
tiktools plugin uninstall acme.tts --purge
tiktools registry add acme https://plugins.acme.dev/v1
tiktools plugin jobs
```

Non-interactive installs require `--yes` (consent computed after printing the
preview). The desktop `--install-plugin <spec>` flag accepts the same grammar.

## 13. Desktop integration

- Drag and drop a `.plugin` onto the window → install dialog.
- OS file association for `.plugin` and a `tiktools-plugin://install?id=…&version=…&registry=…`
  URL scheme (registered by the release packages) → install dialog. Never a
  silent install; the dialog always shows the review step.
- Registry and URL installs use HTTPS only; the desktop never fetches on behalf
  of plugin UIs (the plugin UI broker gains no install methods).

## 14. UI (`src/web`, FSD)

| Slice | Content |
| --- | --- |
| `entities/plugin-registry` | `api/` RPC wrappers, `model/registry-store.ts` (sources, search results, package docs, paging), `lib/` spec parsing and formatting |
| `entities/plugin-job` | `model/plugin-jobs-store.ts` fed by `plugin.job` pushes; `ui/PluginJobProgress.vue` |
| `features/install-plugin` | `InstallPluginDialog.vue`: preview, publisher + verified badge, security model (trusted / isolated / sandboxed explained), permissions and capabilities with plain-language descriptions, permission diff on updates, size, compatibility, "Enable after install", sideload confirmation; replaces the replace banner and `entities/plugin/lib/plugin-install.ts` state |
| `features/update-plugins` | update one / all, per-plugin policy controls |
| `widgets/plugin-catalog` | tabs **Installed · Browse · Updates (n)**. Browse lists registry packages plus bundled-but-inactive packages (badge "Bundled"); categories filter; paged loading |
| `widgets/plugin-details` | header shows origin, publisher, verified state; new **Versions** tab (installed, retained → rollback, available, changelog via the existing markdown-lite renderer); screenshots in General |
| `widgets/plugin-jobs` | status-bar indicator + popover with active jobs and cancel |
| `pages/settings` | "Plugin sources" section: registries (add with fingerprint confirmation, remove, refresh), default auto-update policy, update interval, allow sideloading |

i18n: new keys in `shared/i18n/locales/en` and `es`, one key per
`InstallErrorCode` and per permission/capability description. Tests with
Vitest + `createStoreHarness` asserting RPC calls.

## 15. Author SDK and devkit

`tiktools-plugin` (crate `tiktools-plugin-devkit`, installable with
`cargo install`) replaces `tiktools-plugin-pack` and
`tiktools-plugin-stage-native` (kept as deprecated aliases for one release):

```bash
tiktools-plugin new acme.tts --template process-rust     # native-rust | napi-vm-ts | declarative | webview-ui
tiktools-plugin validate                                   # same validation code as the installer + publish lint
tiktools-plugin build --target win32-x64-msvc              # cargo / bun per template; --all-targets
tiktools-plugin pack --out dist/                           # checksums.json, target-suffixed names
tiktools-plugin stage-native                               # existing nativeLibs staging
tiktools-plugin keygen --publisher acme
tiktools-plugin sign dist/*.plugin --key ~/.tiktools/keys/acme-2026a
tiktools-plugin verify dist/acme.tts-1.2.0-win32-x64-msvc.plugin
tiktools-plugin index add ./registry dist/*.plugin         # static registry: sparse docs, catalog, .sig
tiktools-plugin index yank ./registry acme.tts@1.2.0
tiktools-plugin publish --registry official                # dynamic API (token) or PR/upload for static registries
tiktools-plugin dev link                                   # register this package in the dev root
tiktools-plugin dev watch                                  # rebuild + restart through the control API
```

- `validate` and `pack` reuse `tiktools-plugin-api` and
  `tiktools-plugin-package`, so author-side and host-side validation can never
  drift.
- Templates live in `crates/tiktools-plugin-devkit/templates/` and are tested
  end to end (generate → build → validate → pack).
- `scripts/build-plugin.ts` becomes a thin wrapper over the devkit for in-repo
  examples; `scripts/lib/plugin-targets.ts` stays the TS mirror of
  `PluginTarget`, pinned by a test against the Rust list.

npm packages:

- `@tiktools/plugin-sdk`: protocol types (`PluginCall`, `PluginCallResult`,
  enrichment, events) generated from the Rust schemas by extending
  `scripts/generate-contracts.ts`; `definePlugin()` and typed context for
  napi-vm guests; `plugin.schema.json` for editor autocompletion via
  `$schema`. The schema is hand-written and pinned by a test that parses every
  example manifest with both the Rust parser and the schema.
- `@tiktools/plugin-ui`: the broker client extracted from
  `plugins-example-tiktools/plugins/sonicboom/ui/src/broker.ts`, versioned by `BROKER_API_VERSION`;
  SonicBoom and future UIs depend on it through the workspace. Host and
  desktop interop tests stay where they are.

Documentation: `docs/PLUGIN_AUTHORING.md` (quick start per template),
`docs/PLUGIN_REGISTRY_PROTOCOL.md` (normative spec of Sections 6–7), and an
updated "Installation" section in `docs/PLUGINS.md`.

## 16. Migration and compatibility

- `plugins.install {path, replaceExisting}` keeps working (mapped to a file
  spec; `replaceExisting` becomes implicit consent for local files only).
- `--install-plugin <path> [--replace]` keeps working.
- Existing user packages are adopted (Section 10). Development (`.dev-plugins`)
  and builtin roots are never modified by the installer.
- In-repo plugins (`plugins-example-tiktools/plugins/sonicboom`, `examples/*`) gain
  `publisher: "tiktools"`, `license`, `engines`, `categories`; ids unchanged.
- Release CI (`.github/workflows/release.yml`) builds, signs (index and
  publisher keys from CI secrets), and publishes official plugins to the
  official static registry.
- Schema version stays 3; no manifest becomes invalid.

## 17. Security checklist

- HTTPS only (localhost exempt); redirects limited and kept on HTTPS.
- Size limits enforced while streaming and while extracting (existing 512 MB /
  2 GB / 50k files limits move to `PackageReader`).
- Digest before parse; signature before trust; identity re-checked against the
  real manifest; checksums per extracted file.
- Namespace ownership bound to publisher keys; registry namespace scoping.
- Consent token binds approval to digest + permissions.
- Downgrade protection; yanked and revoked handling; advisories.
- Root keys pinned; custom registry roots pinned by fingerprint on add.
- Registry tokens for private registries stored like plugin secret settings
  (redacted outside execution paths); OS keychain integration is a later
  hardening.
- Deep links and drag-drop only open the review dialog.
- Discovery ignores dot-prefixed directories (swap backups) and the store.
- Native and process plugins are labelled trusted code in the review dialog;
  sandboxing claims only for sandboxed runtimes.

## 18. Roadmap

Each milestone ships green CI, docs, and leaves the app releasable.

### M0 — Foundations (no behavior change)

- Create `tiktools-plugin-package`: move safe extraction, limits, checksums,
  manifest inspection from `crates/tiktools-plugin-loader/src/installer.rs`
  and the writer from `crates/tiktools-plugin-sdk/src/bin/tiktools-plugin-pack.rs`.
- Create `tiktools-plugin-installer`: move `PluginInstaller` and
  `backfill_native_libs`; loader loses `plugin-install`.
- Typed `InstallError` end to end; delete `classify_plugin_install_error`.
- `PluginVersion`, `VersionReq`, `PluginTarget`, `HostCompat` in the API crate.
- `AppPaths::plugin_store`; discovery skips dot-directories.
- Create `tiktools-plugin-devkit` with `pack`, `stage-native`, `validate`.
- Exit: existing installer tests pass from their new homes; UI and CLI
  installs behave exactly as before.

### M1 — Local package management

- Receipts + adoption, retained versions, rollback, per-id locks.
- Job system + `plugin.job` events + cancellation.
- `plugins.inspect` + consent token; `plugins.install` with `spec`;
  `plugins.update` for files; `plugins.rollback`, `plugins.versions`,
  `uninstall --purge`.
- Manifest metadata fields (Section 5) parsed, validated, exposed in summaries.
- UI: `InstallPluginDialog`, jobs indicator, Versions tab; drag and drop.
- CLI: `inspect`, `install <file>`, `rollback`, `versions`, `jobs`.
- Exit: install → update → rollback → uninstall of local files from UI and CLI,
  interrupted at every stage without corrupting the previous install.

### M2 — Signatures and trust

- Detached ed25519 signatures, `keys.json` roles, `TrustStore`, signature
  policies and sideload consent.
- Devkit `keygen`, `sign`, `verify`.
- Exit: signed packages verify; any tampered byte, wrong key, revoked key, or
  namespace mismatch is rejected with the right code.

### M3 — Registry

- `tiktools-plugin-registry`: models, client, cache, resolver, downloader with
  resume, content-addressed cache with LRU GC.
- Registry sources configuration, namespace scoping, mirrors.
- Devkit `index add|yank|build` (static registry generator, `index-writer`
  feature) and `publish` for static registries.
- URL and registry specs in API, CLI (`search`, `install id@req`,
  `registry add/list/remove/refresh`).
- Official static registry bootstrapped; release CI publishes official plugins.
- Exit: loop test — devkit builds a registry into a temp dir, a stub HTTP
  origin serves it, the host resolves, downloads, verifies, and installs by id.

### M4 — Cloud UX and updates

- Browse and Updates tabs, package details (screenshots, changelog),
  categories, paging, search API support.
- `UpdateChecker`, policies, advisories, `plugins.updates.*`, auto-update rules.
- Settings → Plugin sources.
- File association and `tiktools-plugin://` URL scheme in release packages.
- Exit: discover → review → install → get notified → update → rollback,
  entirely from the UI.

### M5 — Author experience

- Devkit `new` templates (process-rust, native-rust, napi-vm-ts, declarative,
  webview-ui), `build`, `dev link`, `dev watch`.
- `@tiktools/plugin-sdk` and `@tiktools/plugin-ui` as Bun workspaces;
  SonicBoom UI migrated to `@tiktools/plugin-ui`.
- `plugin.schema.json`, `docs/PLUGIN_AUTHORING.md`,
  `docs/PLUGIN_REGISTRY_PROTOCOL.md`.
- Exit: from an empty folder, `new → build → pack → sign → index add` and the
  app installs it from a local registry, every template.

### M6 — Scale and ecosystem (later)

- Plugin dependencies with multi-package transactions.
- Dynamic registry service implementing protocol v1 plus search, stats,
  accounts and publisher onboarding; `publish` with API tokens.
- TUF-grade metadata (snapshot/timestamp roles) or Sigstore.
- Publish crates to crates.io and packages to npm with semver checks.

## 19. Verification

- `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- `tiktools-plugin-package`: traversal, symlink, duplicate path, zip bomb,
  checksum mismatch, signature tamper, wrong key, revoked key.
- `tiktools-plugin-registry`: resolver table tests (semver ranges, engines,
  targets, channels, yanked, prerelease), client tests against a stub origin
  (pattern from the existing `nativeLibs` installer tests), ETag cache, resume.
- `tiktools-plugin-installer`: transactions with fault injection after every
  stage (previous version intact, staging cleaned, receipt unchanged), consent
  token mismatch, concurrent jobs on one id (`busy`), cancellation mid-download,
  adoption, rollback, retention pruning.
- `crates/tiktools-control-api/tests/parity.rs` covers every new method;
  `bun run check` regenerates contracts with no diff.
- Web: Vitest for registry/job stores, install dialog states, catalog tabs.
- End-to-end loop test from M3 runs in CI on Linux and Windows.
- Manual: `bun run start`, add a local static registry served over
  `http://localhost`, install a signed package from it, update, rollback,
  uninstall with purge; Termux: `tiktools plugin install` from the same
  registry.

## 20. Decisions to confirm (recommended defaults)

| Decision | Recommended default |
| --- | --- |
| Cloud hosting for v1 | Static registry (GitHub Pages or R2) + artifacts on GitHub Releases; dynamic service later behind the same protocol |
| Signing | ed25519 detached signatures, root → index/publisher roles |
| Id namespace | `publisher.name` for new registry ids; current ids grandfathered |
| Receipts storage | JSON files in `plugin-store` (crate-agnostic) |
| Auto-update default | `notify`; never auto-apply permission changes or majors |
| Retained versions | 2 per plugin |
| Sideloading unsigned files | Allowed with explicit consent, marked unverified |
| Registry base URL | Final domain to be chosen; examples use `plugins.tiktools.app` as a placeholder |
