---
title: "Development setup and loops"
docId: "developer.setup"
translationKey: "developer.setup"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8842472fa359a7c4b2e33bbd6e84a900b9a6dd71a8207afcf61aa507be7494d3"
translationRevision: "8842472fa359a7c4b2e33bbd6e84a900b9a6dd71a8207afcf61aa507be7494d3"
order: 10
---

# Development setup and loops {#development-setup-and-loops}

## Goal and prerequisites {#goal-and-prerequisites}

Build from the current `remake` source without changing the user's installed profile. Use Bun 1.4.4 from `package.json`, Rust 1.95 or newer, and Node compatible with the locked Vite: `^20.19.0 || >=22.12.0`. CI currently uses Rust 1.99.1. Source compilation is separate from [binary installation](../user/installation.md).

## Platform setup {#platform-setup}

On Windows install MSVC Build Tools, WebView2 and native Strawberry Perl for vendored OpenSSL. The Cargo wrapper discovers the MSVC environment and native Perl, preserving explicit `OPENSSL_SRC_PERL`/`PERL` overrides. MSYS/Cygwin Perl is unsuitable for this MSVC path. LLVM's `lld-link` is optional.

For Ubuntu 24.04 desktop development, the CI dependency set includes `libasound2-dev`, `libappindicator3-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libxdo-dev` and `pkg-config`. AppImage packaging also needs GStreamer's development metadata, base/good/libav plugins and its scanner. Optional `mold` accelerates Linux linking. Android/Termux uses a native Android toolchain rather than these desktop libraries.

## Install dependencies {#install-dependencies}

```sh
git switch remake
bun ci
```

Use the committed Bun and Cargo lockfiles. `bun install` is for intentional dependency changes. Documentation is an explicit workspace: its stable VitePress has an isolated Vite dependency while the application retains Vite 8. Do not force both onto one Vite major.

## Choose a loop {#choose-a-loop}

```sh
bun run start                 # owned Vite + Wry desktop development host
bun run dev                   # native local-browser development, no dev-plugin preparation
bun scripts/task.ts serve     # Vite only; requires a real host/adapter for native operations
bun scripts/task.ts packaged  # packaged frontend path, rather than a development URL
```

`start` launches Vite, waits for its allocated URL and runs the desktop against `TIKTOOLS_DEV_URL`. It can prepare plugins from the separate `plugins-example-tiktools` checkout found through `TIKTOOLS_PLUGINS_REPO` or the sibling directory. It does not compile checked-in plugin implementations. Use `TIKTOOLS_SKIP_DEV_PLUGINS=1` to omit that preparation. No host is replaced automatically; an existing desktop owner causes a startup error.

For isolated native experiments, set `TIKTOOLS_HOME` to a disposable directory before launching the host. Use the same home from every CLI terminal controlling that host. Never delete a live ownership lock.

## Validate and recover {#validate-and-recover}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

Run focused tests while iterating and retain the complete required CI gates before merging. Do not run multiple Cargo builds against the same target directory while timing signing tests that invoke Cargo. If a startup refuses an old host, stop the owning process cleanly; if a linker or WebView dependency is missing, repair the platform setup before changing application logic.
