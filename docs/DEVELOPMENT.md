# Development Guide

TikTools has two intentionally separate edit loops: Bun runs Vite/Vue
builds/tests, and Cargo builds/tests the Rust host. The desktop integration
is only needed when changing Winit, Wry, the tray, or the final IPC bridge.

Bun 1.4.1 is the supported frontend version and is pinned in `package.json`.
Use `bun ci` for a clean install from `bun.lock`; run `bun install` only when
changing dependencies intentionally.

## Fast checks

```bash
bun run lint
bun run typecheck
bun run test
cargo check -p tiktools-core --locked
cargo test -p tiktools-core --locked
cargo check -p tiktools-plugin-api --locked
cargo test -p tiktools-plugin-sdk --locked
cargo check -p tiktools-tiktok --locked
```

The core checks do not compile Winit, Wry, GTK, or tray integration. The
workspace feature graph keeps the optional native plugin, persistence, HTTP,
TikTok, and WASM boundaries explicit.

## Desktop loop

```bash
bun run build:web
cargo check -p tiktools-desktop --locked
cargo run -p tiktools-desktop --locked
```

The normal `bun run start` command launches the Vite dev server, waits until
it responds, prepares development plugins, and runs the desktop host against
`TIKTOOLS_DEV_URL`. It discovers example directories containing a
`Cargo.toml` and `plugin.json`, builds process examples in debug mode, and
stages them under the ignored `.dev-plugins` runtime root. This is a
development convenience only; it does not install plugins into the user
profile or add a compile-time plugin registry. Set
`TIKTOOLS_SKIP_DEV_PLUGINS=1` to skip it, or use `bun run start:rust` when the
existing frontend and plugin artifacts are sufficient.

To validate the packaged frontend instead (the `tiktools://app` custom
protocol path used by releases), run:

```bash
bun run start:packaged
```

For manual live frontend changes:

```bash
bun run serve:web
TIKTOOLS_DEV_URL=http://localhost:3000 cargo run -p tiktools-desktop --locked
```

Release builds use the custom `tiktools://app` protocol:

```bash
bun run build:web
cargo build -p tiktools-desktop --release --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

The portable release package is assembled by `scripts/package-release.ts`.
Set `RELEASE_TAG` and one of `windows-x86_64`, `linux-x86_64`,
`macos-arm64`, or `macos-x86_64` as `RELEASE_PLATFORM` after building:

```bash
RELEASE_TAG=v0.1.0 RELEASE_PLATFORM=linux-x86_64 bun run package:release
```

The resulting archive has this layout and is validated before it is returned:

```text
TikTools/
├── LICENSE
├── README.md
├── tiktools-desktop[.exe]
└── web/
    ├── index.html
    └── assets/
```

`bun run check:version` compares `package.json` with the canonical
`[workspace.package].version` in `Cargo.toml`; pass a tag to validate a
release, for example `bun run check:version v0.1.0`.

## Cutting a release

The release workflow refuses tags that do not match the code, so bump
first, then tag. To ship `vX.Y.Z`:

1. set the same version in `package.json` and `Cargo.toml`
   `[workspace.package]`, plus the refreshed `Cargo.lock`
2. commit and push to `remake`
3. create the GitHub release (or push tag `vX.Y.Z`) — the workflow
   validates the tag, builds, packages, and publishes idempotently

## Source ownership

- `crates/tiktools-desktop`: UI-thread lifecycle, Wry IPC callback, custom
  asset protocol, tray, and platform event-loop setup.
- `crates/tiktools-core`: typed IPC, service graph, event bus, points,
  persistence orchestration, automation, and capability policy.
- `crates/tiktools-tiktok`: native discovery, signing, WebSocket reconnects,
  decode, and stable event values.
- `crates/tiktools-plugin-api`: manifest, protocol, capability names, and C ABI.
- `crates/tiktools-plugin-sdk`: typed plugin trait, process adapter, result
  compatibility boundary, and runtime-neutral developer ergonomics.
- `crates/tiktools-plugin-macros`: small generated process/native entry-point
  bridges; unsafe FFI details stay in the SDK implementation.
- `crates/tiktools-plugin-loader`: runtime scanning, validation, installation,
  dynamic native libraries, process plugins, and the optional WASM boundary.
- `src/web`: Vue presentation only.
- `src/automation`: editor contracts and the native event registry consumed by
  the Vue UI.

Keep Wry/Winit types out of core services. Use `HostEmitter` for outbound UI
messages and `EventLoopProxy` for UI-thread work. Never put database handles,
VM values, native TikTok objects, or plugin instances into an automation JSON
event.

## Adding an IPC message

1. Add the discriminated union member to `src/shared/messages.ts`.
2. Mirror it in `crates/tiktools-core/src/ipc/messages.rs`.
3. Validate bounded input in both the frontend boundary and Rust parser.
4. Route it in `AppCore::handle_page_message`.
5. Add the matching `HostMessage` and update the Vue state handler.
6. Test serialization and invalid-input rejection.

The Rust parser is authoritative at runtime. The TypeScript contract stays in
the repository so the existing Vue build remains compatible.

## Adding an automation action

Host action descriptors belong in the Rust catalog and are sent as JSON in the
behavior snapshot. The Vue editor renders their field metadata; it must not
execute host behavior. An action implementation should:

- validate its JSON configuration;
- request a named capability through the core broker;
- keep network/filesystem access outside the WebView;
- emit JSON-safe results and logs;
- include a Rust unit test.

Plugin actions are declared in a runtime manifest under `actionTypes`; no
plugin id is added to Rust source.

## Adding workflow nodes

Workflow node definitions are JSON-safe data returned by
`get-automation-nodes`. Saved graphs must remain schema version 1 until a
deliberate migration is introduced. New nodes need stable type/version values,
validated ports, bounded configuration, and a migration-compatible execution
implementation in core.

## Plugin development

Use `tiktools-plugin-sdk` for Rust plugin ergonomics and
`tiktools-plugin-api` for low-level bindings. Native plugins export
`tiktools_plugin_init` through `tiktools_export_native_plugin!`; the bridge
passes serialized bytes through the C ABI. Do not pass Rust `String`, `Vec`,
trait objects, or futures across that boundary. Process plugins are standalone
executables speaking length-prefixed JSON on stdin and stdout; they are
isolated for crashes, not sandboxed. JavaScript source files are not silently
executed by the desktop host.

Install a package after compilation:

```bash
cargo run -p tiktools-desktop --locked -- --install-plugin ./my-plugin.plugin
```

The installer validates manifest schema, checksums, package-relative paths,
symlinks, and atomic replacement before the next runtime scan.

## Paths and databases

Use `AppPaths::from_environment` rather than `current_dir` for runtime data.
The supported overrides are `TIKTOOLS_HOME`, `TIKTOOLS_DATA_DIR`,
`TIKTOOLS_PLUGINS_DIR`, `TIKTOOLS_PLUGIN_DATA_DIR`, `TIKTOOLS_LOG_DIR`,
`TIKTOOLS_TEMP_DIR`, and `TIKTOOLS_WEB_ROOT`.

Do not rename or recreate the existing SQLite tables as part of ordinary
feature work. Add fixtures when a persisted record format changes, and never
overwrite a user's destination database during path migration.

## Verification before commit

```bash
bun ci
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build -p tiktools-desktop --release --locked
cargo check -p tiktools-desktop --locked
bun run lint
bun run typecheck
bun run test
bun run build:web
git diff --check
```

## Branch protection recommendation

Repository settings are intentionally managed in GitHub rather than by this
repository. Protect `remake` by requiring pull requests, up-to-date branches,
and these checks before merge:

- `frontend`
- `rust`
- each desktop cross-platform check
- CodeQL where available
- dependency review where available

Also prevent force pushes and branch deletion. Release tags are controlled by
maintainers.
