# Android / Termux validation record

Validation was performed natively on Android 16, `arm64-v8a`, Termux 0.118.3.
Rust host: `aarch64-linux-android`; Rust/Cargo 1.99.1; Clang 21.1.8.
Base: `remake`, commit `5f8f573099a77ab22cde694029b2e72d950aad92`.
The CLI ELF is AArch64 and uses `/system/bin/linker64`. These are device tests,
not a Linux cross-compilation result.

## Dependency audit and fixes

Both `cargo tree -p tiktools-cli --locked` and its `-e features` variant were
inspected for the actual Android host, with an additional explicit
`--target aarch64-linux-android` feature-tree check. The final CLI graph excludes
Wry, Winit, GTK, arboard, X11, Wayland, PipeWire and udev. The desktop workspace
members remain in the lockfile and are not part of the CLI build.

The first native `cargo check -p tiktools-cli --locked` failed in two dependencies:

* `rquickjs-sys` 0.12.2 lacked shipped `aarch64-linux-android` bindings. Android
  now enables its existing bindgen capability, using native Clang/Bionic headers.
* The pinned `ttl-live-proto` build selected `protoc-bin-vendored`, which rejects
  an Android build host. `vendor/ttl-live-proto` contains that exact pinned crate,
  with only workspace metadata and build-host protoc selection changed. Native
  Android uses `PROTOC` or installed `protoc`; other build hosts retain the
  vendored compiler. Schemas, runtime sources and upstream license are preserved.

No dependency was downgraded. Cargo.lock includes the new binding-generation
requirements and the source patch. The signer, WebSocket decoder, typed client,
JSON-RPC and plugin protocol remain the existing implementations.

Further portability changes:

* Android excludes the unsupported OS clipboard dependency and returns
  `capability_unavailable` for clipboard and raw-input requests.
* Shared HTTP tooling reads Termux's maintained CA bundle (`SSL_CERT_FILE` or
  `$PREFIX/etc/tls/cert.pem`) rather than requiring an initialized Android Java
  runtime. Missing/empty bundles fail closed. Existing upstream discovery and
  WebSocket WebPKI roots remain unchanged.
* Android native libraries and `.node` addons fail closed until explicit ABI
  validation exists. Discovery exposes unavailable states; other plugins keep
  running. Compatible native process plugins and pure Rust VM guests remain usable.
* Android process plugins receive only three additional environment keys:
  `PREFIX`, `TMPDIR`, `SSL_CERT_FILE`, for native CA and temporary-file discovery.
  Cookies and loader overrides remain excluded.
* Headless hosts call the existing core shutdown path on EOF, SIGINT and SIGTERM.
  The CLI acquires ownership before constructing AppCore, so duplicate hosts cannot
  initialize a second runtime. Concurrent shutdown callers wait for completion, and IPC retains ownership
  through resource cleanup. Final CLI runtime teardown is bounded because Tokio
  cannot cancel a blocking stdin read while the input pipe stays open.
* Sandbox verification now isolates all application paths, plugin sources, the
  working directory and session-cookie environment, and restores the caller's
  environment afterward. It cannot import a checkout's legacy databases.
* IPC test homes use shorter names to fit Unix socket paths under Termux's long
  private temporary-directory prefix.

## Checks

The agent launcher injected a bundled C++ library through `LD_LIBRARY_PATH`.
That made native `protoc` fail despite current Termux libraries. Removing that
launcher override made `protoc --version` work. Validation commands therefore
used `env -u LD_LIBRARY_PATH`; no application integrity or certificate checks
were disabled. Native `protobuf` and `cmake` were installed during setup.

All checks below completed on the native Android device. The final
`scripts/termux/test.sh` exited 0. Counts include repeated tests across feature
configurations; they are test executions, not a count of unique tests.

| Focused suite | Passed | Ignored | Filtered |
| --- | ---: | ---: | ---: |
| tiktools-cli | 20 | 0 | 0 |
| tiktools-control-api | 32 | 0 | 0 |
| tiktools-client | 3 | 0 | 0 |
| tiktools-core | 282 | 1 | 0 |
| tiktools-tiktok | 16 | 0 | 0 |
| core with CLI capabilities | 349 | 0 | 0 |
| plugin SDK and native TLS initialization | 30 | 0 | 0 |
| Android portable plugin probes | 3 | 0 | 0 |
| process framing, environment, timeouts and cleanup | 13 | 0 | 37 |

The single ignored case is `persistence_at_scale_and_restart` when the standalone
core crate is built without its optional SQLite feature. The same test passed
in the full-capability core run. The 37 filtered loader unit tests were not run
by the targeted `process::tests::` command and are not counted as passed.
Desktop native-addon fixtures, frontend tests, external integrations and
credentialed LIVE tests were skipped separately.

Successful commands (with the launcher library override removed):

```bash
env -u LD_LIBRARY_PATH bash scripts/termux/check.sh
env -u LD_LIBRARY_PATH CARGO_BUILD_JOBS=2 cargo build -p tiktools-cli --locked
env -u LD_LIBRARY_PATH bash scripts/termux/test.sh
env -u LD_LIBRARY_PATH TIKTOOLS_BIN="$PWD/target/debug/tiktools" bash scripts/termux/smoke.sh
env -u LD_LIBRARY_PATH bash scripts/termux/doctor.sh
env -u LD_LIBRARY_PATH CARGO_BUILD_JOBS=2 cargo check -p tiktools-cli --locked --offline
env -u LD_LIBRARY_PATH ./target/debug/tiktools --json api verify --sandbox
cargo fmt --all -- --check
git diff --check
bash -n scripts/termux/check.sh scripts/termux/smoke.sh scripts/termux/test.sh scripts/termux/doctor.sh
```

The test script executes the five focused crate commands in the user-facing
workflow, plus the core with `persistence,native-tiktok,http`, SDK TLS tests,
Android portable plugin probes and targeted process-runtime tests. It contains
no desktop build or frontend requirement.

The final smoke test passed standalone startup, IPC ownership and duplicate-host
refusal, multiple clients observing the same host PID, mode-0600 sockets, JSON
discovery, automation/workflow/processor listing, SQLite persistence over restart,
SIGINT/SIGTERM cleanup, stale-socket recovery after SIGKILL, CLI event inspection,
stdio events, idle-stdio signal termination, VM plugin enable/health/disable,
structured native-plugin refusal, and sandbox verification. Only its own isolated
children and directories were used and cleaned up.

An additional locked offline compilation check passed using the populated Cargo
cache. A separate direct sandbox verifier invocation exited 0 and reported
`ok: true` for the combined coverage/smoke report.

Doctor reported `AndroidTermux`, writable temporary storage and a working SQLite
behavior store. Disconnected LIVE was correctly reported as skipped. The final
full debug matrix occupied approximately 15 GB of target artifacts; this includes
multiple feature configurations. Low-storage devices should select reduced debug
information before their first build, as documented in TERMUX.md.

Earlier failures led to the native compiler fixes above, shorter IPC test paths,
platform-aware test assertions, and a corrected process-timeout fixture. Termux's
shell can exec a final redirected command, so `cat > /dev/null` closed the response
pipe before timeout; the test now uses `exec sleep` to hold the pipe open. The
production timeout implementation was preserved and its 13 focused tests passed.


## Capability and verification limits

On-device `system info` reports `AndroidTermux`, local IPC, persistence, networking,
process plugins and JavaScript VM capability. Native plugins, `.node` addons,
desktop audio, global hotkeys, native file pickers, WebView and desktop
notifications are unavailable. Capabilities describe build/platform support;
they do not claim an external service is connected.

A separate anonymous `--standalone --json live channels --limit 1` probe succeeded
with no session cookie and returned `rooms`, `nextOffset`, `fetchedAt`, and `source`.
Authenticated LIVE, signing a live bundle, real gift delivery, sustained WebSocket
ping/pong and mobile-network transitions were not tested. Lifecycle and principal
events are tested with mocks/synthetic fixtures. Subscription and room-control
messages retain the raw payload at the upstream SDK decoder boundary, but unknown
methods are not currently projected into CLI domain events. No normalized
subscription schema or CLI subscription-event coverage is claimed. No session cookies were supplied during automated or public-network probes.

Windows, Linux and macOS desktop builds were not executed on this phone. Their
existing target branches/dependencies are preserved, but that is not a claim of
passing desktop regression tests. Frontend/Bun checks, desktop native-addon
fixtures and third-party desktop integrations were skipped.

For exact setup and execution commands, see [TERMUX.md](TERMUX.md). For safe
agent validation and pre-commit checks, see
[TERMUX_AGENT_WORKFLOW.md](TERMUX_AGENT_WORKFLOW.md).

## Files changed

- `Cargo.lock`
- `Cargo.toml`
- `README.md`
- `crates/tiktools-cli/Cargo.toml`
- `crates/tiktools-cli/src/client.rs`
- `crates/tiktools-cli/src/host.rs`
- `crates/tiktools-cli/src/main.rs`
- `crates/tiktools-cli/tests/sandbox.rs`
- `crates/tiktools-control-api/src/modules/system.rs`
- `crates/tiktools-control-api/src/transport/mod.rs`
- `crates/tiktools-control-api/tests/ipc.rs`
- `crates/tiktools-core/Cargo.toml`
- `crates/tiktools-core/src/control/mod.rs`
- `crates/tiktools-core/src/control/system.rs`
- `crates/tiktools-core/src/control/widgets.rs`
- `crates/tiktools-core/src/input_access.rs`
- `crates/tiktools-core/src/lib.rs`
- `crates/tiktools-core/src/platform.rs`
- `crates/tiktools-core/src/plugin_processors/tests.rs`
- `crates/tiktools-core/src/tests.rs`
- `crates/tiktools-core/tests/live_throughput.rs`
- `crates/tiktools-plugin-loader/src/discovery.rs`
- `crates/tiktools-plugin-loader/src/napi_vm.rs`
- `crates/tiktools-plugin-loader/src/native.rs`
- `crates/tiktools-plugin-loader/src/process.rs`
- `crates/tiktools-plugin-loader/tests/android_portable.rs`
- `crates/tiktools-plugin-sdk/Cargo.toml`
- `crates/tiktools-plugin-sdk/src/http.rs`
- `crates/tiktools-plugin-sdk/src/lib.rs`
- `crates/tiktools-plugin-sdk/src/native_providers.rs`
- `crates/tiktools-tiktok/Cargo.toml`
- `crates/tiktools-tiktok/src/lib.rs`
- `crates/tiktools-tiktok/tests/fixtures/README.md`
- `crates/tiktools-tiktok/tests/fixtures/chat-unicode-emote.pb`
- `crates/tiktools-tiktok/tests/fixtures/gift.pb`
- `crates/tiktools-tiktok/tests/fixtures/like.pb`
- `crates/tiktools-tiktok/tests/fixtures/member.pb`
- `crates/tiktools-tiktok/tests/fixtures/room-user.pb`
- `crates/tiktools-tiktok/tests/fixtures/share.pb`
- `crates/tiktools-tiktok/tests/fixtures/social.pb`
- `crates/tiktools-tiktok/tests/portable_events.rs`
- `docs/TERMUX.md`
- `docs/TERMUX_AGENT_WORKFLOW.md`
- `docs/TERMUX_VALIDATION.md`
- `scripts/termux/check.sh`
- `scripts/termux/doctor.sh`
- `scripts/termux/smoke.sh`
- `scripts/termux/test.sh`
- `vendor/ttl-live-proto/`: 65 files copied from the pinned upstream crate; only `Cargo.toml`, `build.rs` and the added `TERMUX-PATCH.md` differ from that source.
