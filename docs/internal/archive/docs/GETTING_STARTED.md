# Getting Started

## Requirements

- Rust 1.95 or newer with Cargo.
- MSVC Build Tools are required on Windows.
- Native Windows Perl is required on Windows to compile the vendored OpenSSL
  dependency used by cloud authentication. Install Strawberry Perl once:
  `winget install --id StrawberryPerl.StrawberryPerl --exact`.
  Released TikTools binaries do not require Perl.
- LLVM `lld-link` is optional. When available on `PATH`, Rust commands
  automatically use it for faster linking. Otherwise the default MSVC
  linker is used.
- Bun 1.4.4 for the frontend build and development server. The version is
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

### Windows build environment

`bun run dev`, `bun run start`, `bun run build`, and
`bun scripts/task.ts rust <cargo arguments>` use `scripts/cargo-with-linker.mjs`.
It loads the installed MSVC developer environment when necessary and detects
native Windows Perl on `PATH`. It also finds Strawberry Perl under
`C:\Strawberry`, `<SystemDrive>\Strawberry`, `<ProgramFiles>\Strawberry`, and
`<LOCALAPPDATA>\Programs\Strawberry`, even when the terminal or IDE has an old
`PATH`. It sets `OPENSSL_SRC_PERL` for the Cargo child process without adding
Strawberry's GCC tools to `PATH`.

For a portable or custom Perl installation, set the executable explicitly:

```powershell
$env:OPENSSL_SRC_PERL = 'D:\Tools\Strawberry\perl\bin\perl.exe'
bun run dev
```

Existing `OPENSSL_SRC_PERL` and `PERL` settings are preserved. Use native
Windows Perl for MSVC builds; the MSYS/Cygwin Perl shipped with Unix-like
environments is unsuitable for this configuration. See
[OpenSSL's Perl notes](https://github.com/openssl/openssl/blob/master/NOTES-PERL.md).
If Perl is absent, the wrapper prints the installation command before Cargo
runs. Install it once and retry. Direct `cargo` commands bypass this setup;
use a Developer PowerShell with native Perl available or set
`OPENSSL_SRC_PERL` yourself.

## Install and run

```bash
bun ci
bun run lint
bun x vue-tsc --noEmit
bun scripts/task.ts frontend-test
bun run start
```

The `start` script launches the Vite dev server, compiles and stages
the checked-in process-plugin examples into the ignored `.dev-plugins`
directory, then runs `tiktools-desktop` against `TIKTOOLS_DEV_URL` with that
directory as a development plugin root. Use `bun scripts/task.ts rust run -p tiktools-desktop` when you do
not need the example plugin build. To exercise the packaged-asset path
instead (Vue assets built into `dist/web` and served through the
`tiktools://app/index.html` custom protocol, as releases use), run
`bun scripts/task.ts packaged`.

For UI-only iteration:

```bash
bun scripts/task.ts serve
TIKTOOLS_DEV_URL=http://localhost:3000 cargo run -p tiktools-desktop --locked
```

To only rebuild and stage example plugins:

```bash
bun scripts/task.ts prepare-plugins
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
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

The core-only commands avoid desktop dependencies. `cargo test --workspace --locked`
also exercises SQLite, plugin manifest/ABI, native event normalization, the
bounded QuickJS adapter, and the Wry asset handler.

## Frontend commands

```bash
bun scripts/task.ts frontend-build
bun scripts/task.ts serve
bun run lint
bun x vue-tsc --noEmit
bun scripts/task.ts frontend-test
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
