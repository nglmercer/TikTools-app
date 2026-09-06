# Troubleshooting

## The Rust host does not start

Build the frontend first:

```bash
bun run build:web
cargo run -p tiktools-desktop
```

If a debug host says that `index.html` is missing, set `TIKTOOLS_WEB_ROOT` to
the directory containing `dist/web/index.html`, or set `TIKTOOLS_DEV_URL` to a
running frontend server. A packaged release must keep `web/index.html` beside
the executable inside the extracted TikTools folder; release builds ignore
development URL overrides.

## Linux WebView errors

Wry uses the system GTK/WebKitGTK stack on Linux. Install the development and
runtime packages supplied by your distribution, then run:

```bash
cargo check -p tiktools-desktop
cargo run -p tiktools-desktop
```

Platform-specific setup belongs in `crates/tiktools-desktop/src/platform.rs`;
the core does not require GTK.

Current Linux desktop WebViews require X11/XWayland; native Wayland without
`DISPLAY` is rejected with a warning. When both `DISPLAY` and
`WAYLAND_DISPLAY` are present, the host selects the X11 backend for GTK and
Winit so the raw handles stay compatible.

## The packaged web application did not become ready

The hidden window waits up to 10 seconds for the `frontend-ready` IPC after
the WebView is created. On Linux the event loop keeps pumping GTK every 50 ms
inside that deadline so WebKit can progress; Windows/macOS wait for the
deadline directly.

Run with debug logging and inspect the startup transitions:

```bash
RUST_LOG=tiktools=debug bun run start:packaged
tail -n 200 ~/.local/share/TikTools/logs/tiktools.log
```

Expected sequence:

```text
Linux desktop display environment
creating TikTools frontend WebView
Wry WebView created successfully
frontend page load started
frontend page load finished
frontend-ready IPC received
```

If only `frontend page load finished` appears, the page loaded but the ready
signal never arrived; a missing `window.ipc` bridge now throws during
frontend startup instead of timing out silently. To isolate a custom-protocol
problem from an event-loop problem, compare with the Vite dev server path:

```bash
bun run serve:web
RUST_LOG=tiktools=debug \
TIKTOOLS_DEV_URL=http://127.0.0.1:3000 \
cargo run -p tiktools-desktop
```

If the dev-server path works but the packaged path fails, suspect asset
loading; if both fail, suspect the GTK/Wry/IPC bridge.

## The page loads but does not respond

The Vue app expects the Wry bridge `window.ipc.postMessage`. A normal
browser tab or an arbitrary HTTP server does not provide that bridge. Use the
Rust desktop host and inspect DevTools in a debug build. The raw message flow
is:

```text
window.ipc.postMessage → Wry → IpcRouter → AppCore → WebView
```

Invalid or oversized messages are rejected by the Rust parser and logged at
debug/warn level.

## TikTok connection errors

The native client needs a valid creator handle and a network path to TikTok.
Anonymous discovery may be rate-limited; retry later or supply an authenticated
Cookie request header. Cookies are held in memory and are not written to
SQLite or logs.

For signing failures, provide a compatible `webmssdk.js` bundle with
`TIKTOOLS_TIKTOK_SIGNING_BUNDLE`, or allow the native client to use its configured
cache/download path. The Rust client uses the pinned `tiktok-signer` crates
directly and does not require a JavaScript/Bun host process.

## Database or missing data

Rust uses platform app-data paths. Check the resolved overrides:

```text
TIKTOOLS_HOME
TIKTOOLS_DATA_DIR
TIKTOOLS_PLUGINS_DIR
TIKTOOLS_PLUGIN_DATA_DIR
TIKTOOLS_LOG_DIR
TIKTOOLS_TEMP_DIR
```

A checkout-local `data/tiktok-points.db` or `data/tiktok-automation.db` is
copied only when the corresponding platform destination does not exist. The
source is not deleted, and an existing destination is never overwritten.

If SQLite reports a schema error, back up both database files before changing
anything and run:

```bash
cargo test -p tiktools-core --features persistence
```

The Rust schema intentionally preserves the existing table names and JSON
payload columns.

## Plugin is not listed

Check the runtime plugin directories and make sure each package contains
`plugin.json` with:

```text
schemaVersion: 2
runtime: native | process | wasm
entry: package-relative file
protocolVersion: 1
```

The host rejects invalid IDs, path traversal, incompatible versions, platform
mismatches, missing entries, and JavaScript source entries declared as process
plugins. Scan order is built-in, user, then development override.

## Native plugin failed to load

Native plugins are trusted in-process libraries. Confirm that the library
matches the current OS/architecture and exports `tiktools_plugin_init`. Its
manifest ABI version must match the host. A native library remains loaded for
the process lifetime; restart TikTools after replacing it.

Use a process plugin when a crash or incompatible native dependency should be
contained outside the application.

## Automation script errors

Scripts run in `napi-vm` with bounded source/result sizes and a loop budget.
They receive JSON `event`, `inputs`, and `data` values only. Node APIs,
filesystem, network, process spawning, and WebView handles are unavailable.
Move privileged work into a declared host action or plugin capability.

## Useful checks

```bash
bun run typecheck
bun run test
cargo test --workspace
cargo check -p tiktools-desktop
git diff --check
```
