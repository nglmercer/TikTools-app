---
title: "Platforms and native Termux builds"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "84f0988ac1aa15def527c00d0a134fb6c5c3bd00ef60efc7a376faec426a162f"
translationRevision: "84f0988ac1aa15def527c00d0a134fb6c5c3bd00ef60efc7a376faec426a162f"
order: 20
---

# Platforms and native Termux builds {#platforms-and-native-termux-builds}

## Goal and prerequisites {#goal-and-prerequisites}

Choose a build that matches the target operating system. Desktop development needs Bun 1.4.4, Rust 1.95 or newer and the platform WebView dependencies described in [setup](setup.md). Termux builds the Rust CLI directly inside Android's private home; Android shared storage is unsuitable for executables and Unix sockets.

## Build the appropriate host {#build-the-appropriate-host}

Windows uses MSVC Build Tools and WebView2. Native Strawberry Perl supports vendored OpenSSL; do not substitute MSYS Perl. Linux desktop needs GTK3, WebKitGTK 4.1, audio development libraries and `pkg-config`. Winit/Wry integration belongs in `crates/tiktools-desktop/src/platform.rs`; core business services remain independent of the window toolkit.

On an ARM64 phone, first verify that `rustc -vV` reports `aarch64-linux-android`. In Termux:

```sh
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
```

Cargo is intentional here: the native phone workflow does not require Bun or Node. With a Bun-capable desktop, route Cargo through `bun scripts/task.ts rust ...` to retain linker/cache handling. For two phone terminals, start `./target/debug/tiktools host --ipc` and use the same `TIKTOOLS_HOME` for subsequent CLI calls.

## Expected result and status {#expected-result-and-status}

Local IPC, SQLite and portable process/QuickJS plugins have implemented Android paths. Process artifacts must target Android/Bionic. Native libraries and `.node` addons are unavailable on Android. Desktop audio, hotkeys, notifications, file pickers and WebViews have no phone backend. Persisted remote profiles fail explicitly because Android has no selected credential-store backend; SDK credentials can remain in memory.

## Verify and recover {#verify-and-recover}

Run `bash scripts/termux/doctor.sh`, `bash scripts/termux/test.sh` and `bash scripts/termux/smoke.sh` from the checkout root. Report skipped frontend checks honestly. For broken `protoc` C++ symbols, compare `env -u LD_LIBRARY_PATH protoc --version` before changing libraries. A missing CA bundle must be repaired, never bypassed. Cross-compilation alone does not establish phone acceptance. The [Termux smoke script](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/termux/smoke.sh) records the targeted probes and remaining restrictions.
