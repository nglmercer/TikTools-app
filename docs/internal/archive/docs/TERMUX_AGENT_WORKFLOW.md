# Agent workflow on Android

Work inside the native Termux checkout on `remake`. Read the source and any
repository instructions before editing. Preserve the typed Control API,
AppCore ownership, plugin protocol and event schemas. Never launch desktop
components as a validation shortcut.

```bash
git status --short --branch
rustc -vV
bash scripts/termux/check.sh
```

Make focused edits, then run:

```bash
git diff --check
cargo fmt --all -- --check
cargo check -p tiktools-cli --locked
cargo test -p tiktools-cli --locked
cargo test -p tiktools-control-api --locked
cargo test -p tiktools-client --locked
cargo test -p tiktools-core --locked
cargo test -p tiktools-tiktok --locked
cargo build -p tiktools-cli --locked
./target/debug/tiktools --json api verify --sandbox
bash scripts/termux/smoke.sh
```

Use `api discover` and `api schema <method>` rather than inventing commands.
The sandbox verifier selects a throwaway core. The smoke script additionally
isolates every application directory and plugin source, validates the real IPC
host and cleans up only its own child processes. A standalone command without
sandbox isolation uses the user's real data; do not use it for mutating tests.

Never print or commit `TIKTOOLS_SESSION_COOKIE`. Automated tests must not require
an authenticated TikTok session. Use fixtures and mocked lifecycle transports.
Keep tests with real credentials separate and explicitly report them untested
unless they actually ran. Report passed, failed and skipped checks separately;
a skipped frontend test is never a pass. Capture the target, Rust version,
command exit status and relevant failure output in the final report.

Native `.so` and `.node` plugins fail closed on Android. Process plugins need
Bionic executables and valid shebangs. Do not disable trust, integrity, timeout
or environment restrictions to make a desktop integration run on a phone.
The pure Rust VM supports only the host capabilities actually available.

For offline work, first populate the Cargo cache while online, then add
`--offline` to focused Cargo commands. TLS failures require checking the CA
bundle and network, not disabling verification. Missing QuickJS bindings require
native Clang binding generation. Native protobuf generation requires Termux's
`protobuf` package. If an agent-injected library path breaks `protoc`, diagnose
with `env -u LD_LIBRARY_PATH protoc --version` and use that prefix only when
confirmed. See [Termux setup](TERMUX.md) for resource and background limits.

Before committing, inspect `git diff`, check for secrets and unrelated generated
files, and record every remaining failure. Commit and push only the intended
changes using the project's normal Git workflow.
