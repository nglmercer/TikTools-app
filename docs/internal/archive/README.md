# TikTools

TikTools is a Rust desktop and headless host for TikTok LIVE. The desktop
client uses a Vue WebView; Android/Termux uses the native `tiktools` CLI. It
connects to live rooms, displays chat and engagement telemetry, awards viewer
points, and runs local automations. Tauri is not used.

For native phone development, see [Termux setup and execution](docs/TERMUX.md).
That workflow builds the Rust CLI directly. Frontend tools are optional.

## What it includes

- Desktop Winit/Wry window and WebView lifecycle, plus a `tray-icon` tray.
- Native Rust TikTok discovery, signing, WebSocket transport, and event decode.
- One `AppCore` behind a typed JSON-RPC control API shared by the Vue UI,
  the `tiktools` CLI, local IPC, and agents, with `DomainEvent` streaming
  to every client.
- Experimental authenticated HTTPS/WSS host, native remote CLI profiles and
  separate desktop controller with scoped workflows, closed automation status,
  confirmed whole-event disable/delete and a bounded live points
  board and viewer lookup. Desktop and SDK/CLI can disable/delete permitted
  workflows and adjust points through durable commands with outcome recovery.
  The controller also provides explicit private draft saves and host/draft
  comparison. See the
  [native client guide](docs/LIVE-COLLABORATION-CLIENT.md) for setup and limits.
- Owner-reviewed delegated automations with current permission checks, scoped
  HTTP/audio, exact-topic emitted rules and bounded scripts. Runtime receipts
  prevent retained script/emission intents from being repeated; see the
  [approval policy and remaining gates](docs/LIVE-COLLABORATION-APPROVAL.md).
- Rust-owned SQLite persistence for points, creators, gifts, workflows, and
  behavior records.
- Bounded JavaScript automation and plugins through the embedded QuickJS (`rquickjs`) runtime.
- Runtime-discovered native, process, and optional WASM plugins.
- English and Spanish UI translations, themes, points, analytics, and
  automation editors.

## Quick start

Requirements:

- Rust 1.95 or newer and Cargo.
- Bun 1.4.4 for the Vue/Vite development toolchain and frontend asset build.
- Platform WebView dependencies. Linux uses WebKitGTK; see
  [Getting Started](docs/GETTING_STARTED.md).

From a checkout:

```bash
bun ci
bun run lint
bun x vue-tsc --noEmit
bun scripts/task.ts frontend-test
bun run start
```

`start` builds `dist/web` and launches the Rust desktop binary. Example and
first-party plugins live in the separate
[`plugins-example-tiktools`](https://github.com/nglmercer/plugins-example-tiktools)
repository and are never bundled with the app; when a checkout sits next to
this one (or `TIKTOOLS_PLUGINS_REPO` points at it), `start` builds its packages
into the ignored `.dev-plugins` directory and loads them as development
plugins. Use `TIKTOOLS_SKIP_DEV_PLUGINS=1 bun run start` to skip that step. For fast UI-only iteration, run
the frontend server and point the Rust host at it:

```bash
bun scripts/task.ts serve
TIKTOOLS_DEV_URL=http://localhost:3000 cargo run -p tiktools-desktop --locked
```

Release assets are served through the `tiktools://app/...` custom Wry protocol.
Portable packages contain the executable beside a `web/` directory:

```bash
bun scripts/task.ts frontend-build
cargo build -p tiktools-desktop --release --locked
RELEASE_TAG=v0.1.0 RELEASE_PLATFORM=windows-x86_64 bun scripts/task.ts package
```

## Commands

```bash
bun run start  # Launch the desktop development application
bun run dev    # Launch the headless development environment
bun run build  # Build frontend, widgets, plugins, and desktop
bun run lint   # Check frontend, plugin and Rust code
bun run test   # Run fast unit tests (Bun + Vitest)
bun run ci     # Run checks, tests and production builds
```

Targeted tests (no full-suite run needed):

```bash
bun run test rust tiktools-core batched_likes --features native-tiktok
bun run test web live-batch
bun run test bun automation
bun run test plugin sonicboom
bun run test e2e
```

Specialized tasks are available via `bun scripts/task.ts`, for example
`bun scripts/task.ts rust test --workspace --locked`,
`bun scripts/task.ts e2e test`, and `bun scripts/task.ts version bump 0.3.4`.
Run `bun ci` after changing dependencies before starting development.

## Development / Quality

Use Bun 1.4.4, as pinned in `package.json`. `bun ci` installs exactly from
`bun.lock`; use `bun install` only when intentionally changing dependencies.
Before opening a pull request, run:

```bash
bun ci
bun run lint
bun x vue-tsc --noEmit
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build

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
cargo run -p tiktools-desktop --locked -- --install-plugin acme.tts
```

## Project layout

```text
crates/tiktools-desktop/       Winit, Wry, tray, UI-thread bridge, control IPC
crates/tiktools-control-api/   JSON-RPC router, transports, client, discovery
crates/tiktools-collaboration/ Private host invitations, sessions, grants, audit
crates/tiktools-remote/        Optional authenticated HTTPS/WSS host gateway
crates/tiktools-client/        Typed direct/IPC SDK and optional native remote client
crates/tiktools-cli/           IPC-first CLI with explicit remote profiles
crates/tiktools-core/          Control operations, services, SQLite, points, events
crates/tiktools-plugin-api/    Versioned manifest, protocol, capabilities, ABI
crates/tiktools-plugin-sdk/    Typed plugin trait, adapters, result compatibility
crates/tiktools-plugin-macros/ Small process/native entry-point macros
crates/tiktools-plugin-loader/ Runtime discovery and plugin runtimes
crates/tiktools-tiktok/        Native signer, discovery, WebSocket, event model
src/web/                       Vue application and styles
src/web/platform/              JSON-RPC control-client bridge (owns window.ipc)
src/web/features/              One domain composable per feature
src/shared/messages.ts         Host-push shape contract
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

Plugins contribute actions, event sources, and pre-filter event processors
(`processorTypes` + the `events.enrich` capability); processors enrich host
events under the optional `event.intel` namespace before automation filters
run, fail open on any error, and are previewed over IPC with
`processors.test`/`processors.status`. See `docs/PLUGINS.md` and the
`plugins-example-tiktools/plugins/textintel/` reference implementation.

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
- [Control API](docs/CONTROL_API.md)
- [Collaborative LIVE design and roadmap](docs/LIVE-COLLABORATION.md)
- [Collaborative LIVE implementation specification](docs/LIVE-COLLABORATION-SPEC.md)
- [Experimental HTTPS/WSS gateway setup](docs/LIVE-COLLABORATION-TRANSPORT.md)
- [Owner collaboration settings and access management](docs/LIVE-COLLABORATION-OWNER.md)
- [Desktop command journal and recovery](docs/LIVE-COLLABORATION-CLIENT-COMMANDS.md)
- [Automation consistency and delegated execution](docs/LIVE-COLLABORATION-AUTOMATION.md)
- [Development Guide](docs/DEVELOPMENT.md)
- [Native Termux / Android development](docs/TERMUX.md)
- [Termux agent workflow](docs/TERMUX_AGENT_WORKFLOW.md)
- [On-device Termux validation](docs/TERMUX_VALIDATION.md)
- [Contributing](CONTRIBUTING.md)
- [Automations](docs/AUTOMATIONS.md)
- [Plugins](docs/PLUGINS.md)
- [Plugin authoring](docs/PLUGIN_AUTHORING.md)
- [Plugin registry protocol](docs/PLUGIN_REGISTRY_PROTOCOL.md)
- [Plugin UI architecture](docs/PLUGIN_UI_ARCHITECTURE.md)
- [Playwright E2E](docs/PLAYWRIGHT.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [User Guide](docs/USER_GUIDE.md)
- [UI Kit Usage](docs/UI_KIT_USAGE.md)
