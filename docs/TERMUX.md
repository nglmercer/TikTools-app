# Native Termux development

Use the `remake` branch in Termux's private home directory, not Android shared
storage (which may prohibit execution and Unix sockets). No APK, compositor,
Node.js or Bun is needed for the Rust CLI. The CLI uses the same AppCore,
typed client, JSON-RPC, SQLite and plugin protocol as the desktop host.

## Setup

```bash
pkg update && pkg upgrade
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
git clone https://github.com/nglmercer/TikTools-app.git
cd TikTools-app
git checkout remake
rustc -vV
cargo --version
clang --version
```

Rust must be at least 1.95; ARM64 native Rust reports host
`aarch64-linux-android`. A Linux/glibc toolchain or a cross-compilation result
does not validate native Termux. `protobuf` provides Bionic-native `protoc`;
Android QuickJS bindings are generated using Clang. SQLite is bundled.
OpenSSL development files are not required by the CLI's Rust TLS stack.

If `protoc --version` reports missing C++ symbols despite current packages,
inspect `LD_LIBRARY_PATH`: agent launchers may inject an older bundled C++
library. `env -u LD_LIBRARY_PATH protoc --version` diagnoses that conflict.
When confirmed, prefix development commands with `env -u LD_LIBRARY_PATH`.
Do not copy desktop libraries into Termux or disable certificate verification.

## Build and run

```bash
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
./target/debug/tiktools --json api verify --sandbox
./target/debug/tiktools host --ipc
```

The host runs in the foreground. In a second Termux session, from the checkout:

```bash
./target/debug/tiktools --json system info
./target/debug/tiktools --json system health
./target/debug/tiktools live status
./target/debug/tiktools plugin list
./target/debug/tiktools api discover
./target/debug/tiktools api schema live.connect
./target/debug/tiktools automation list
./target/debug/tiktools workflow list
./target/debug/tiktools processor status
```

These commands connect to one existing host; they never start a second core.
`--standalone` explicitly creates a core for that invocation. Keep `TIKTOOLS_HOME`
identical between sessions; its default is `$XDG_DATA_HOME/TikTools` or
`$HOME/.local/share/TikTools`. The socket lives inside that directory, with
mode 0600. Keep the directory private and short enough for Unix socket paths.
Stop with Ctrl-C, SIGTERM, or:

```bash
./target/debug/tiktools api call system.shutdown '{}' --confirm
```

The ownership guard protects startup and stale-socket recovery. Do not manually
remove locks while a host is running. SIGKILL cannot flush pending writes;
restart the host with the same home to recover the endpoint and SQLite stores.

## LIVE and event debugging

Read the session cookie without placing its contents in shell history:

```bash
read -rs -p 'Session cookie: ' TIKTOOLS_SESSION_COOKIE; echo
export TIKTOOLS_SESSION_COOKIE
./target/debug/tiktools live connect --unique-id example
unset TIKTOOLS_SESSION_COOKIE
./target/debug/tiktools api events --topics 'live.*' --max-events 20 --timeout 60
./target/debug/tiktools live disconnect
```

Do not log, commit, or paste cookies into agent prompts. Authenticated LIVE
checks are manual and separate from automated validation. Network availability,
TikTok access rules and session validity cannot be inferred from successful
compilation. Discovery, signing and decoding remain in the existing TikTok
crate and its pinned upstream dependencies. HTTPS uses Termux's maintained CA
bundle at `$PREFIX/etc/tls/cert.pem`, or `SSL_CERT_FILE` when explicitly set.
No JVM is needed. A missing or empty CA bundle fails closed. Upstream discovery
and WebSocket transport use their existing WebPKI root stores. Subscription and
room-control messages retain raw payloads in the upstream SDK decoder. Unknown
methods are not currently projected into CLI domain events; normalized subscription
support is not added by this portability milestone.

## Plugin capabilities

`system info` reports platform/build capabilities, not successful runtime
probes. `system doctor` tests storage and database access; disconnected LIVE
transport is reported as skipped, not a successful network test.

* Portable napi-vm guests use the existing pure Rust interpreter. Compatibility
  depends on their requested host facilities; JavaScript alone is no guarantee.
* Process plugins must be Android/Bionic executables for the device architecture,
  with executable permissions and a usable interpreter if they have a shebang.
  Use installed interpreter paths, not `/bin/bash` or glibc Linux binaries.
  Existing bounded calls, framing and integrity checks apply. Android adds only
  `PREFIX`, `TMPDIR` and `SSL_CERT_FILE` to the existing environment allowlist
  so native plugins can resolve the CA bundle and writable temporary storage.
  Cookies and dynamic-loader overrides are not forwarded.
* Native dynamic libraries and `.node` addons are unavailable on Android until
  explicit ABI validation is implemented. Desktop artifacts must not be loaded.
* Desktop audio, hotkeys, file pickers, WebView and notifications have no Android
  backend in this milestone. OBS, PipeWire and game integrations also require
  their actual supported platform and executable dependencies.

```bash
./target/debug/tiktools plugin health plugin-id
./target/debug/tiktools plugin enable plugin-id
./target/debug/tiktools plugin disable plugin-id
```

## Validate changes on the phone

```bash
bash scripts/termux/doctor.sh
bash scripts/termux/test.sh
bash scripts/termux/smoke.sh
cargo fmt --all -- --check
git diff --check
```

The smoke script clears application path overrides, excludes development plugins
and session cookies, and uses a disposable home. It checks IPC ownership, JSON,
SQLite restart, graceful termination, stale-socket recovery and local events.
It never contacts or terminates a production host. Focused tests can be run with:

```bash
cargo test -p tiktools-cli --locked
cargo test -p tiktools-control-api --locked
cargo test -p tiktools-client --locked
cargo test -p tiktools-core --locked
cargo test -p tiktools-tiktok --locked
cargo test -p tiktools-plugin-sdk --locked --features http-client
cargo test -p tiktools-plugin-loader --locked --no-default-features --test android_portable
cargo test -p tiktools-plugin-loader --locked --no-default-features --lib process::tests::
```

The Android plugin probes use committed Rust/JavaScript test code and a native
Python process fixture. The full desktop native-addon fixture suite is excluded;
it requires unsupported desktop addons and separately compiled TypeScript
fixtures. That exclusion is a skip, not a successful test.

Bun/Vue lint, type checks and frontend tests are optional and require a compatible
runtime. Report them as skipped when unavailable. Never include tiktools-desktop
in the default phone build/test pipeline. See [agent workflow](TERMUX_AGENT_WORKFLOW.md).

## Edit, inspect and commit

Pull the selected branch before starting a new editing session, then edit Rust
or TypeScript with Codex or an installed text editor. Run the focused crate test
for the code you changed, rebuild the CLI, and exercise the feature against a
private test home or the sandbox verifier. Before committing, run the checks
above and inspect the diff.

```bash
git switch remake
git pull --ff-only
# Edit source, run focused checks, rebuild, and test the feature.
git status --short --branch
git diff
git add -p
git commit -m "Describe the change"
git push origin remake
```

Stage new files explicitly by name after reviewing them. Do not stage credentials
or session captures. With a running host, save a private event capture for local
inspection using the existing event command:

```bash
umask 077
./target/debug/tiktools api events --topics 'live.*' --timeout 60 > "$HOME/tiktools-live-events.ndjson"
```

Host startup/transport diagnostics go to stderr and can also be redirected to a
private file. Review captures before sharing them. `api discover`, `api schema`
and `system doctor` expose runtime diagnostics without a desktop console.

## Resource limits and interrupted work

Start with two build jobs, or one on low-memory phones. The test script also
defaults to two test threads; override `RUST_TEST_THREADS` when appropriate. Keep several GB free;
debug information and test executables can grow substantially. Optional
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` reduce disk/memory
use but trigger a separate rebuild when changed. Let the phone cool during long
builds and avoid repeated clean builds. Offline checks work with `--offline`
once all locked dependencies have been downloaded.

Android can stop Termux when backgrounded. Keep the host foregrounded during
validation. Optional `termux-wake-lock` / `termux-wake-unlock` can help during
short active sessions, at a battery cost; they cannot guarantee survival.
Review Android's per-app battery settings if needed. Root is unnecessary.
Network loss and mobile-network changes may disconnect LIVE; inspect status
before reconnecting and avoid tight shell retry loops. Existing bounded event
queues and gap reporting remain authoritative: resync after an event gap.
Capture diagnostics in a private file if needed, and review for secrets before
sharing. The CLI does not require a desktop log viewer.

## Local browser adapter (development preview)

The optional `local-web` feature serves the existing Vue UI over loopback HTTP
and uses an authenticated WebSocket to the existing IPC host. It creates no
second AppCore. It accepts no LAN/public bind address and provides no remote
access. Browser/device acceptance is tracked in
[LOCAL_BROWSER_ADAPTER_VALIDATION.md](LOCAL_BROWSER_ADAPTER_VALIDATION.md).

Build the adapter natively:

```bash
cargo build -p tiktools-cli --features local-web --locked
```

Prepare the web assets using the existing frontend build in a compatible
JavaScript environment (`bun run build:web`). The generated `dist/web` can be
copied to the phone from that environment. Bun/Node is not needed to run those
assets or the Rust adapter. On this device, Node with the installed frontend
dependencies also runs `node node_modules/vite/bin/vite.js build` successfully;
this does not replace the repository's Bun lockfile or imply every Bun script
runs under Node. Do not commit an npm-generated lockfile.

Start the host in one Termux session, then the adapter in another:

```bash
./target/debug/tiktools host --ipc
# Second session, from the repository root:
./target/debug/tiktools web --assets "$PWD/dist/web" --port 8787
```

Open `http://127.0.0.1:8787` in the phone browser. Enter the one-use code printed
in the adapter terminal; it expires after five minutes and eight unsuccessful
attempts. The browser receives a port-scoped HttpOnly, SameSite=Strict session cookie valid
for one hour. Restart the adapter to obtain a new pairing code. Never paste a
TikTok session cookie into this pairing form. Stop the adapter with Ctrl+C; the
IPC host remains running and available to the CLI.

The frontend uses the shared Control API for system state, LIVE, automations,
workflows and plugins. Unsupported Android capabilities remain unavailable.
Browser disconnect does not undo an accepted operation; the client never
replays mutations automatically. Reconnection refreshes authoritative state and
requests bounded feed backfill. An evicted history or restarted host is reported
as a discontinuity rather than complete event delivery.

Plugin UIs stay in sandboxed iframes and communicate through the existing scoped
plugin broker. A random asset-only URL prefix allows opaque-origin module loads
without exposing the control session to the iframe; it expires with the pairing
session and cannot authorize RPC. It serves only validated plugin UI roots,
rejects path traversal/symlink escapes, and never serves plugin data directories.
The WebSocket rejects opaque/plugin origins. External plugin network requests
from the iframe are restricted; plugin backend actions keep their normal policy.

Development integration probe (requires the installed frontend test toolchain,
including its `ws` package; these are not Rust runtime dependencies):

```bash
node scripts/termux/web-smoke.mjs
```

The probe launches only its own host/adapter under isolated application paths,
never uses a real cookie, and cleans up its own processes and temporary files.
Use `env -u LD_LIBRARY_PATH` for Node/Cargo if the agent launcher supplies the
incompatible library override described earlier.
