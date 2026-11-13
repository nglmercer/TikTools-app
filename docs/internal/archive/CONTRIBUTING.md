# Contributing to TikTools

TikTools keeps the existing Vue/Bun frontend and Rust Winit/Wry desktop host.
Please keep changes focused and avoid unrelated architecture or UI rewrites.

## Supported toolchain

- Rust 1.95.0 or newer (the workspace MSRV is 1.95; CI uses Rust 1.99.1).
- Bun 1.4.4, pinned in `package.json`.
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
bun x vue-tsc --noEmit
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
```

Run the Rust checks with the committed `Cargo.lock`:

```bash
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

Targeted tests (fast units by default; full coverage stays in `bun run ci`):

```bash
bun run test                                  # fast Bun + Vitest units
bun run test rust tiktools-core batched_likes # one crate, one filter
bun run test rust tiktools-core --features native-tiktok
bun run test web live-batch                   # one web module
bun run test web --watch
bun run test bun automation                   # one Bun area
bun run test plugin sonicboom                 # one plugin
bun run test e2e                              # browser tests
bun run test --changed                        # only changed files
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

Release tags are maintainer-controlled. Before tagging, bump the app
version in one step so `package.json`, `Cargo.toml`, and `Cargo.lock`
stay in sync:

```bash
bun scripts/task.ts version bump v0.1.0
```

The release workflow builds portable packages with the executable beside
`web/index.html` and publishes `SHA256SUMS.txt`.

Repository administrators should protect `remake` by requiring pull requests,
required status checks, and an up-to-date branch. Recommended required checks
are `frontend`, `rust`, every desktop platform check, CodeQL where available,
and dependency review where available. Force pushes and branch deletion should
remain disabled.
