# Contributing to TikTools

TikTools keeps the existing Vue/Bun frontend and Rust Winit/Wry desktop host.
Please keep changes focused and avoid unrelated architecture or UI rewrites.

## Supported toolchain

- Rust 1.88.0 or newer (the workspace MSRV is 1.88; CI uses Rust 1.98.1).
- Bun 1.4.1, pinned in `package.json`.
- MSVC Build Tools are required on Windows.
- LLVM `lld-link` is optional. When available on `PATH`, Rust commands
  automatically use it for faster linking. Otherwise the default MSVC
  linker is used.
- Linux desktop builds require GTK/WebKitGTK and audio development packages;
  see [Getting Started](docs/GETTING_STARTED.md).

## Setup and checks

Start with a lockfile-clean install:

```bash
bun ci
```

Before opening a pull request, run the frontend checks:

```bash
bun run lint
bun run typecheck
bun run test
bun run build:web
```

Run the Rust checks with the committed `Cargo.lock`:

```bash
npm run fmt:rust
npm run rust:clippy
npm run rust:test
npm run build:desktop
```

Pull requests targeting `remake` should pass the frontend, Rust, desktop
cross-platform, CodeQL, and dependency-review checks that apply to the change.
Keep the branch up to date before merging.

## Security and privacy

Never commit TikTok cookies, session tokens, API keys, private keys, `.env`
files, credentials, or generated local databases. Cookies stay in memory and
must not be copied into logs, issues, tests, or pull requests. Native plugins
are trusted native code; process boundaries are not OS sandboxes.

## Releases and branch policy

Release tags are maintainer-controlled. A tag must match the canonical Cargo
workspace version, for example:

```bash
bun run check:version v0.1.0
```

The release workflow builds portable packages with the executable beside
`web/index.html` and publishes `SHA256SUMS.txt`.

Repository administrators should protect `remake` by requiring pull requests,
required status checks, and an up-to-date branch. Recommended required checks
are `frontend`, `rust`, every desktop platform check, CodeQL where available,
and dependency review where available. Force pushes and branch deletion should
remain disabled.
