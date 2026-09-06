# TikTools

TikTools is a Rust desktop host for TikTok LIVE with a Vue WebView. It
connects to live rooms, displays chat and engagement telemetry, awards viewer
points, and runs local automations. Tauri is not used.

## What it includes

- Direct Winit/Wry window and WebView lifecycle, plus a `tray-icon` tray.
- Native Rust TikTok discovery, signing, WebSocket transport, and event decode.
- Existing Vue UI and `PageMessage`/`HostMessage` JSON IPC contract.
- Rust-owned SQLite persistence for points, creators, gifts, workflows, and
  behavior records.
- Bounded JavaScript automation through the pure-Rust `napi-vm` runtime.
- Runtime-discovered native, process, and optional WASM plugins.
- English and Spanish UI translations, themes, points, analytics, and
  automation editors.

## Quick start

Requirements:

- Rust 1.88 or newer and Cargo.
- Bun 1.4.1 for the Vue/Vite development toolchain and frontend asset build.
- Platform WebView dependencies. Linux uses WebKitGTK; see
  [Getting Started](docs/GETTING_STARTED.md).

From a checkout:

```bash
bun ci
bun run lint
bun run typecheck
bun run test
bun run start
```

`start` builds `dist/web`, compiles the checked-in process-plugin examples into
the ignored `.dev-plugins` directory, and launches the Rust desktop binary with
that directory as a development runtime root. This makes plugin changes easy
to test without installing them into the user profile. Use `start:rust` to
launch without rebuilding the example plugins. For fast UI-only iteration, run
the frontend server and point the Rust host at it:

```bash
bun run serve:web
TIKTOOLS_DEV_URL=http://localhost:3000 cargo run -p tiktools-desktop --locked
```

Release assets are served through the `tiktools://app/...` custom Wry protocol.
Portable packages contain the executable beside a `web/` directory:

```bash
bun run build:web
cargo build -p tiktools-desktop --release --locked
RELEASE_TAG=v0.1.0 RELEASE_PLATFORM=windows-x86_64 bun run package:release
```

## Commands

```bash
bun run start             # Build the Vue assets and run the Rust host
bun run prepare:dev-plugins # Compile and stage example plugins only
bun run start:rust        # Run the Rust host against existing dist/web assets
bun run dev               # Run the frontend development server
bun run build:web         # Build dist/web
bun run typecheck         # Type-check the Vue/editor source
bun run test              # Run frontend and editor tests
bun run lint              # Run ESLint for Vue, TypeScript, and Bun scripts
bun run check:web         # Lint, type-check, test, and build the frontend
bun run check:rust        # Check every Cargo workspace crate with Cargo.lock
bun run fmt:rust          # Check Rust formatting
bun run lint:rust         # Run Clippy with warnings denied
bun run test:rust         # Run every Rust workspace test with Cargo.lock
bun run build:desktop     # Build the release desktop executable with Cargo.lock
bun run check:version     # Validate package and Cargo versions
```

## Development / Quality

Use Bun 1.4.1, as pinned in `package.json`. `bun ci` installs exactly from
`bun.lock`; use `bun install` only when intentionally changing dependencies.
Before opening a pull request, run:

```bash
bun ci
bun run lint
bun run typecheck
bun run test
bun run build:web

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
```

Pull requests targeting `remake` are expected to pass these checks and the
desktop cross-platform compilation matrix in CI. See
[Contributing](CONTRIBUTING.md) for the maintainer release and branch policy.

Install a validated plugin package after the executable has been compiled:

```bash
cargo run -p tiktools-desktop --locked -- --install-plugin ./example.plugin
cargo run -p tiktools-desktop --locked -- --install-plugin ./example.plugin --replace
```

## Project layout

```text
crates/tiktools-desktop/       Winit, Wry, tray, UI-thread bridge
crates/tiktools-core/          IPC router, services, SQLite, points, events
crates/tiktools-plugin-api/    Versioned manifest, protocol, capabilities, ABI
crates/tiktools-plugin-sdk/    Typed plugin trait, adapters, result compatibility
crates/tiktools-plugin-macros/ Small process/native entry-point macros
crates/tiktools-plugin-loader/ Runtime discovery and plugin runtimes
crates/tiktools-tiktok/        Native signer, discovery, WebSocket, event model
src/web/                       Vue application and styles
src/shared/messages.ts         Frontend compatibility contract
src/automation/                Generated contracts, registry, and event types
docs/                          Architecture and development documentation
```

`tiktools-core` deliberately has no Winit, Wry, or `tray-icon` dependency. The
desktop crate translates `HostMessage` values into UI-thread commands through
`EventLoopProxy`; the core can therefore be tested without a WebView.

## Runtime plugin model

Plugins are discovered from runtime directories, never from a compile-time
list. The usual locations are:

```text
Windows  %LOCALAPPDATA%/TikTools/plugins
Linux    ~/.local/share/TikTools/plugins
macOS    ~/Library/Application Support/TikTools/plugins
```

Each package contains a `plugin.json` with schema version 2, a runtime kind,
and an entry path. Native libraries use a small serialized-message C ABI and
remain loaded until application shutdown. Process plugins use length-delimited
JSON over stdin/stdout. WASM support is intentionally optional.

Example native manifest:

```json
{
  "schemaVersion": 2,
  "id": "miniaudio",
  "name": "MiniAudio",
  "version": "1.2.0",
  "runtime": "native",
  "entry": "miniaudio.dll",
  "protocolVersion": 1,
  "abiVersion": 1,
  "permissions": ["audio.output"]
}
```

Native plugins are trusted code. Manifest permissions limit the host API for
process/WASM plugins and document the expected access of native plugins.

## Data and privacy

Rust resolves writable paths from platform app-data directories, not the
process working directory. Override them for development with
`TIKTOOLS_HOME`, `TIKTOOLS_DATA_DIR`, `TIKTOOLS_PLUGINS_DIR`,
`TIKTOOLS_PLUGIN_DATA_DIR`, `TIKTOOLS_LOG_DIR`, or `TIKTOOLS_TEMP_DIR`.

Existing `data/tiktok-points.db` and `data/tiktok-automation.db` files are
copied to the platform data directory only when the destination is missing;
the Rust host preserves the existing table names and JSON records.

Session cookies stay in memory and must never be committed or logged.

## Documentation

- [Getting Started](docs/GETTING_STARTED.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Rust migration](docs/RUST_MIGRATION.md)
- [Development Guide](docs/DEVELOPMENT.md)
- [Contributing](CONTRIBUTING.md)
- [Automations](docs/AUTOMATIONS.md)
- [Plugins](docs/PLUGINS.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [User Guide](docs/USER_GUIDE.md)
- [UI Kit Usage](docs/UI_KIT_USAGE.md)
