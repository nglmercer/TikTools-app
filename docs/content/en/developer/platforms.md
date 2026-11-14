---
title: "Platforms and native Termux builds"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c7befe2be6f91bcf50ce4572aaa95d02723a315ba11ed1d94e6d499e802c2473"
translationRevision: "c7befe2be6f91bcf50ce4572aaa95d02723a315ba11ed1d94e6d499e802c2473"
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

## Operational details {#operational-details}

For a local browser adapter, build frontend assets containing index.html and a CLI with `local-web`. Clear inherited path overrides and set a disposable `TIKTOOLS_HOME`; start the IPC host first. In a second terminal with the same home run the adapter command below, open its loopback one-click link or enter the six-digit one-use pairing code (valid five minutes). Stop the adapter and host independently when finished. Loopback binding and pairing do not provide remote/LAN access. A Termux device needs its own native Android toolchain and built web assets; a Linux desktop binary is not an Android binary.

```sh
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-cli --features local-web --locked
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools host --ipc
```

```sh
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools web --assets /absolute/path/to/TikTools/dist/web --port 4174
```
