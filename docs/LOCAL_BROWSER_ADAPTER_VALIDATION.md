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

## Browser connection lifecycle follow-up

A source audit found that detaching before a WebSocket opened left its connect
promise unsettled, and old socket callbacks could arrive after replacement.
The browser transport now settles cancelled connection attempts, rejects pending
work when an open connection is replaced, clears old buffered state, and ignores
stale callbacks. It does not replay accepted operations.

The focused browser transport/control-client regression run passed 25 tests,
including cancellation, late callbacks and connection replacement. These tests
are additional evidence for the transport lifecycle, not actual Android browser
suspension/resume verification.

Follow-up validation commands:

```bash
env -u LD_LIBRARY_PATH node node_modules/vitest/vitest.mjs run --maxWorkers=2 \
  src/web/shared/api/browser-transport.test.ts src/web/shared/api/control-client.test.ts
env -u LD_LIBRARY_PATH node node_modules/vue-tsc/bin/vue-tsc.js --noEmit
env -u LD_LIBRARY_PATH node node_modules/eslint/bin/eslint.js \
  src/web/shared/api/browser-transport.ts src/web/shared/api/browser-transport.test.ts --max-warnings=0
env -u LD_LIBRARY_PATH node node_modules/vite/bin/vite.js build
git diff --check
```

## Checks unavailable in this environment

- Windows, Linux and macOS desktop binary builds/runtime checks were not run:
  this phone's native Rust host is `aarch64-linux-android`. Passing native
  Control API and default WebView client tests is not a desktop binary result.
- Bun-specific scripts were not run; Bun is not installed. The listed Node-based
  typecheck, lint, Vue tests and production asset builds were executed instead.
- No Android browser-control tool, ADB executable or local Chromium executable
  is available to this agent. `termux-open-url` can launch a browser but cannot
  inspect rendering, iframe execution, touch layout or suspension/resume.
- Authenticated TikTok LIVE remains a separate credentialed integration check;
  no session cookie was supplied to these local-adapter tests.

Skipped/unavailable checks above are not counted as passed.

## Six-digit pairing follow-up

Human pairing now uses a uniformly generated six-digit decimal code, including
leading zeros. Only the human code was shortened: session cookies and plugin
asset tokens still use independent 256-bit random values. Codes remain one-use,
expire after five minutes, and share an eight-attempt limit across all clients.
Malformed/non-string/non-ASCII codes consume attempts and are rejected.

The pairing form uses a text field with numeric input mode, six-digit validation
and one-time-code autofill. Codes are sent by POST, never in the URL. The native
CLI regression run passed 26 tests, including leading-zero handling, malformed
codes, expiry/reuse, attempt limits and unchanged session-token length. The
expanded isolated HTTP/WebSocket smoke passed with a generated six-digit code,
verified numeric form attributes and a 64-hex-character session cookie.
Changed-script ESLint, Rust formatting and whitespace checks also passed.
Actual Android keyboard rendering remains part of the browser acceptance below.

## Pairing-code renewal follow-up

The pairing page can request a replacement code using an origin-checked POST.
The replacement appears only in the adapter terminal and invalidates the old
code. A shared 30-second cooldown bounds requests; renewals preserve the shared
eight-attempt budget and cannot invalidate an active browser session.

Native Termux validation passed:

- `cargo test -p tiktools-cli --features local-web --locked`: 29 test executions.
- Focused `pair-page.test.ts` Vitest run: four tests, covering renewal, cooldown,
  lockout, network failure, active sessions and leading-zero form submission.
- `node scripts/termux/web-smoke.mjs`: isolated HTTP/WebSocket smoke, including
  origin rejection, replacement-code login, old-code rejection and cooldown.
- Vue typecheck, changed-file ESLint, Rust formatting and whitespace checks.

These automated results do not establish actual Android browser rendering.

## Remaining acceptance work

- Verify actual rendering/pairing/navigation on an Android browser, inline plugin
  broker behavior, phone viewport, suspension/resume and host/adapter restart.

No remote access, public binding or Android WebView is implemented. No complete
browser support or authenticated LIVE connectivity is claimed by this record.
