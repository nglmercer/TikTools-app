# Local browser adapter implementation record

Status: in progress. Full Android browser acceptance has not been completed.
Branch: `feat/local-browser-adapter`, based on merged Termux PR #29.

## Source inventory and boundaries

- The existing Vue `ControlClient` now accepts an injected wire transport;
  the native WebView globals and presentation handshake remain its default.
- `ui.projection` and frontend-ready are desktop window concerns. The browser
  does not call/emulate the desktop projection operation. It subscribes to the
  existing domain events and uses existing authoritative state operations.
- `ui.feed.backfill` was intercepted only by the desktop. An additive shared
  typed operation now exposes bounded history on the existing core EventBus,
  without another scheduler/bus or replaying automation side effects. The
  desktop interceptor remains compatible. Browser recovery detects cursor reset
  after host restart and marks history discontinuities.
- Plugin frames already use `sandbox="allow-scripts"` and a scoped postMessage
  broker. Desktop URLs use `tiktools-plugin://`; browser URLs use confined
  asset-only routes. `plugins.ui.assets` resolves the existing core manifest
  validation over IPC; its typed wrapper and parameter validation are included.
- Plugin windows, native file pickers and desktop-only integrations remain
  unavailable on Android. The inline frame reuses the existing broker for
  settings/actions/options/events. Its actual browser behavior is a pending
  acceptance gate, not implied by HTTP asset tests.
- The optional Rust server lives in the existing CLI under `local-web`.
  CLI-only compilation has no HTTP server feature by default. It connects to
  IPC and never creates AppCore; an absent host is an explicit error.

## Completed native Android checks

Android ARM64/Termux, Rust host `aarch64-linux-android`. Commands used the
native toolchain with the agent launcher's conflicting `LD_LIBRARY_PATH` unset.

- Locked optional adapter compilation and build passed. A cached offline check
  passed after minimizing Cargo.lock to eight added packages, with every
  pre-existing package version and dependency selection preserved.
- Locked CLI-only compilation passed with the adapter disabled by default.
- Follow-up CLI validation passed 24 tests after adding per-port session cookies
  and correct usage/help handling. These overlap with the combined run below.
- Combined native regression command passed **410 tests**, zero failures and
  zero ignored cases: CLI, Control API, typed client and full-capability core.
  This includes bounded feed history, no replay side effects, IPC reader
  lifetime, registry/parameter parity, SQLite and existing CLI integration tests.
- Full Vue/Vitest suite passed **463 tests**. The final focused transport/feed/
  plugin broker set passed **48 tests**. These counts overlap, not unique totals.
- Frontend typecheck, changed-source ESLint and Vite production build passed.
  Node ran the installed frontend tools natively; dependencies were installed
  without generating another lockfile. Bun-specific scripts were not run.
- Isolated HTTP/WebSocket probe passed one-use pairing, authentication,
  Host/Origin rejection, discovery/errors, workflow events, shared CLI Unicode
  state, eight-client admission bound, plugin HTML/module content types,
  opaque-origin asset access, wrong-token/symlink rejection and typed backfill.
  The expanded probe also passed host restart persistence, WebSocket disconnect
  propagation, reconnect using the still-valid session, and graceful adapter
  shutdown with an active socket while leaving the restarted host available.
- Direct sandbox verification passed: 85 discovered/typed methods match,
  22 smoke operations passed, zero failed, overall `ok: true`.
- Rust formatting and changed-file whitespace checks passed.

Successful commands:

```bash
env -u LD_LIBRARY_PATH CARGO_BUILD_JOBS=2 cargo build -p tiktools-cli --features local-web --locked
env -u LD_LIBRARY_PATH CARGO_BUILD_JOBS=2 cargo check -p tiktools-cli --locked
# Run under isolated application paths; see scripts/termux/test.sh for setup:
env -u LD_LIBRARY_PATH CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test \
  -p tiktools-cli -p tiktools-control-api -p tiktools-client -p tiktools-core \
  --features tiktools-cli/local-web,tiktools-core/persistence,tiktools-core/native-tiktok,tiktools-core/http --locked
env -u LD_LIBRARY_PATH node node_modules/vitest/vitest.mjs run --maxWorkers=2
env -u LD_LIBRARY_PATH node node_modules/vue-tsc/bin/vue-tsc.js --noEmit
env -u LD_LIBRARY_PATH node node_modules/vite/bin/vite.js build
env -u LD_LIBRARY_PATH node scripts/termux/web-smoke.mjs
env -u LD_LIBRARY_PATH ./target/debug/tiktools --json api verify --sandbox
cargo fmt --all -- --check
git diff --check
```

An overlapping Cargo build/test initially caused `ExecutableFileBusy`; serial
validation passed. Subsequent Cargo validation is serialized. Temporary preview
fixtures initially had an invalid VM manifest name; that fixture was corrected
without changing runtime validation or security checks.

## Remaining acceptance work

- Verify actual rendering/pairing/navigation on an Android browser, inline plugin
  broker behavior, phone viewport, suspension/resume and host/adapter restart.
- Record exact final commands/results and any unavailable desktop/frontend checks.

No remote access, public binding or Android WebView is implemented. No complete
browser support or authenticated LIVE connectivity is claimed by this record.
