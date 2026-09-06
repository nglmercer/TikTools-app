# Getting Started

## Requirements

- Rust 1.88 or newer with Cargo.
- Bun 1.4.1 for the frontend build and development server. The version is
  pinned by `package.json`.
- A system WebView supported by Wry:
  - Windows: WebView2.
  - Linux: GTK and WebKitGTK development/runtime packages.
  - macOS: the system WebKit framework.
- Git.

On Debian/Ubuntu, install the GTK/WebKitGTK development packages provided by
your distribution before building the desktop crate. Package names vary with
the distribution release; if Cargo reports a missing GTK or WebKit library,
install its `-dev` package and retry.

## Install and run

```bash
bun ci
bun run lint
bun run typecheck
bun run test
bun run start
```

The `start` script launches the Vite dev server, compiles and stages
the checked-in process-plugin examples into the ignored `.dev-plugins`
directory, then runs `tiktools-desktop` against `TIKTOOLS_DEV_URL` with that
directory as a development plugin root. Use `bun run start:rust` when you do
not need the example plugin build. To exercise the packaged-asset path
instead (Vue assets built into `dist/web` and served through the
`tiktools://app/index.html` custom protocol, as releases use), run
`bun run start:packaged`.

For UI-only iteration:

```bash
bun run serve:web
TIKTOOLS_DEV_URL=http://localhost:3000 cargo run -p tiktools-desktop --locked
```

To only rebuild and stage example plugins:

```bash
bun run prepare:dev-plugins
```

The development URL bypasses the custom protocol while retaining the same Wry
IPC bridge.

## First connection

1. Enter a TikTok creator handle; the leading `@` is optional.
2. Leave Cookie empty for anonymous discovery, or enter an authenticated
   Cookie request header when the room requires it.
3. Connect directly or choose the first room returned by live discovery.
4. Use Feed, Points, Analytics, Automations, Plugins, and Settings.

Cookies stay in memory. Do not paste them into source files, issue reports, or
logs.

## Rust commands

```bash
cargo check -p tiktools-core --locked
cargo test -p tiktools-core --locked
cargo check -p tiktools-desktop --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build -p tiktools-desktop --release --locked
```

The core-only commands avoid desktop dependencies. `cargo test --workspace --locked`
also exercises SQLite, plugin manifest/ABI, native event normalization, the
bounded `napi-vm` adapter, and the Wry asset handler.

## Frontend commands

```bash
bun run build:web
bun run serve:web
bun run lint
bun run typecheck
bun run test
```

The frontend uses Vue single-file components. Interactive render functions
continue to use Vue's JSX transform where that keeps behavior localized. Its
JSON message types live in
`src/shared/messages.ts`; no Rust UI rewrite is required.

## Runtime data

The host uses platform app-data locations:

```text
Windows  %LOCALAPPDATA%/TikTools/
Linux    ~/.local/share/TikTools/
macOS    ~/Library/Application Support/TikTools/
```

Subdirectories are `data/`, `plugins/`, `plugin-data/`, `logs/`, and `temp/`.
For development or tests, set `TIKTOOLS_HOME` or the more specific path
overrides documented in the [Development Guide](DEVELOPMENT.md).

If a checkout contains `data/tiktok-points.db` or
`data/tiktok-automation.db`, the Rust host copies it to the platform data
directory only when the destination does not exist. The original files are
not deleted or modified by that copy.

## Installing a plugin

Build or obtain a validated `.plugin` package, then install it after the app
binary exists:

```bash
cargo run -p tiktools-desktop --locked -- --install-plugin ./my-plugin.plugin
```

The package must contain a schema-version-2 `plugin.json`. The installer
rejects path traversal, unsafe symlinks, invalid checksums, incompatible
protocol/ABI versions, and entries outside the package. Restart the app after
replacing a native library; native hot-unloading is intentionally unsupported.

## Release package layout

Maintainer-controlled tags such as `v0.1.0` run the full CI quality gate before
portable archives are published. Each archive contains the desktop executable,
the packaged frontend, and the license:

```text
TikTools/
├── LICENSE
├── README.md
├── tiktools-desktop[.exe]
└── web/
    ├── index.html
    └── assets/
```

The release workflow produces Windows x86_64 ZIP and Linux x86_64 tar.gz
artifacts plus `SHA256SUMS.txt`. Release builds use
the packaged `web/` directory and never depend on `TIKTOOLS_DEV_URL`.
