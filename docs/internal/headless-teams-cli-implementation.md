# Headless Teams and compiled-artifact acceptance

Implementation started from application `41b6d1901d4c76c232ccd823f087237c78d695da`
(`remake`, 4.2.2) and RemakeBackend
`6a69c99a3a134f3f76333316957d788f31d4f004`.
Work is isolated on `feat/headless-teams-cli` and
`feat/headless-teams-acceptance`, respectively, to preserve unrelated changes
in both original checkouts. No workflow, release, push or deployment is part
of this implementation.

## Available agent operations

The CLI uses the existing control contracts, Cloud authentication and native
peer SDK. It does not implement a second Teams service or bypass approval.

| Context | Commands |
| --- | --- |
| Owner runtime | `teams create`, `list --owned`, `host enroll/start/status/stop` |
| Discovery | `teams people search NICKNAME` |
| Owner admission | `teams invitations list/create/revoke`, `requests list/approve/deny` |
| Owner authorization | `teams grants list/update/revoke`, `sessions list/revoke` |
| Controller identity | `remote cloud login/status/logout/enroll` |
| Controller admission | `teams join --profile NAME --kind agent --invite-stdin --non-interactive` |
| Controller connection | `remote cloud connect --host ID --profile NAME --kind agent --wait --non-interactive` |
| Inspection | `teams list --joined`, `teams doctor`, `teams doctor --context controller`, `remote check NAME` |
| Persistent agent | `remote serve --profile NAME --stdio --events --timeout 30` |
| Artifact identity | `--build-info` |

Start one persistent owner with an absolute, isolated `TIKTOOLS_HOME`:

```sh
cargo build -p tiktools-cli --locked --features remote-control
TIKTOOLS_HOME=/absolute/owner target/debug/tiktools host --ipc
```

Use the same owner home for subsequent owner commands. A controller uses its
own home/profile directory. The initial login uses the existing native PKCE
browser handoff. Subsequent commands need no desktop window. Saved controller
credentials require a working OS credential store; noninteractive admission
returns `authentication_required` rather than starting a browser.

See [the agent guide](../content/en/developer/agents.md#headless-teams) for the
complete owner approval and controller sequence. Its policy snippet comes
from [agent-policy.json](../examples/agent-policy.json), a single executable
JSON source shared by all six translated editions.

## Persistent protocol and cleanup

The first notification is `connection.ready`, with the connection's host,
instance, session, grant revision and capabilities. Requests use JSON-RPC
envelopes, one per line, with string or integer IDs. Example:

```json
{"jsonrpc":"2.0","id":"inspect-1","method":"live.status","params":{}}
```

Requests are schema-validated and bounded to 128 KiB and the selected timeout.
Effectful or destructive methods require `confirm: true`; durable writes also
need the existing command/host metadata. The host remains responsible for
capabilities, resource scopes, limits, revisions and command deduplication.
The client preserves error data and does not automatically replay writes.
After an uncertain timeout, resolve the original command outcome.

Event output is optional. Internal closure notifications are consumed even
without `--events`, so idle clients stop when their authority is revoked.
EOF, termination signals and transport closure release the client. Native
profile commands never fall back to creating a local AppCore.

Use one persistent client for sequenced agent work. Rapid independent peer
connections can return transport errors; the acceptance flow does not hide
them by retrying mutations.

## Reproducible acceptance against an existing binary

In the matching backend checkout, install pgrust once:

```sh
cargo run -p devtools -- db install
# Or reuse an installed prebuilt distribution:
cargo run -p devtools -- db install --prebuilt /absolute/path/pgrust
```

From the app checkout, explicitly select the compiled CLI:

```sh
bun scripts/task.ts test headless-teams \
  --binary /absolute/path/tiktools \
  --backend /absolute/path/RemakeBackend \
  --report /tmp/headless-teams.json
```

The backend must contain `native-cli-acceptance`. The runner compiles only the
backend fixture and its development launcher; it never rebuilds the selected
app artifact. Cargo's JSON artifacts resolve executables correctly with a
custom `CARGO_TARGET_DIR`.

The lab creates private loopback TLS, a private PostgreSQL cluster and three
synthetic identities. It exercises real native PKCE, IPC and peer connections.
Reports contain the selected binary's SHA-256, a fresh run ID and timed cases.
An empty filter, stale report or changed binary cannot pass. Credentials are
logged out before the server stops, and CLI mode removes its database data.

Development artifacts accept the isolated Cloud origin at runtime. Release
artifacts retain their compiled origin. To test a release binary, compile a
staging build with a fixed loopback HTTPS origin and pass `--cloud-port PORT`.
Production-pinned binaries are rejected before authentication.

## Distribution changes

Release builds include the matching companion CLI with Cloud, controller and
headless host features. Packaging verifies its version, release build mode and
source SHA, writes `cli-build.json`, and includes it in portable archives and
installers. Installer verification checks its identity and content, allowing
the existing AppImage ELF relocation rule.

The desktop dispatches `--cli` to its companion before logging, runtime locks,
controller parsing or GUI initialization. New AppImages therefore support:

```sh
./TikTools.AppImage --cli --build-info
./TikTools.AppImage --cli --json teams doctor
```

`smoke-appimage.ts --require-cli` additionally executes the packaged CLI's
metadata and sandbox API verification. Recovery keeps compatibility with older
immutable tags. Previously published 4.2.2 binaries do not gain new commands
without rebuilding. For the full Teams acceptance suite, select the actual
packaged companion executable after extraction/installation.

## Bugs found by real acceptance

- Cloud login could create a profile parent with permissive permissions before
  the strict profile manager initialized it. New profile roots are prepared
  privately before account sidecars; existing unsafe roots still fail closed.
- Unified native dependencies could leave rustls without an unambiguous crypto
  provider. CLI startup now selects the same provider as the desktop.
- Controller account status did not restore its saved session before reporting
  sign-in state. It now reports verification and credential-state diagnostics.
- Agent grants were hidden by the desktop-kind membership filter. CLI membership
  discovery uses all authorized native kinds while desktop filtering remains.
- A side-effectful, nondestructive method such as `points.adjust` could bypass
  a destructive-only stdio confirmation check. Both flags now require consent.
- An idle stdio client without event output could miss transport closure. It now
  consumes the SDK shutdown signal independently of event presentation.
- A documentation metadata unit test rendered every article and exceeded its
  five-second budget. It now reads the one real article its assertion needs;
  the separate content gate still checks every published article.

## Validation scope

This suite validates real native processes and a disposable backend on one
Linux machine. It does not establish physical two-PC/cross-NAT behavior,
production account behavior, real TikTok/OBS effects, Windows execution or a
fresh installer build. Tests do not mutate existing application data or
production Teams. No GitHub Actions run was launched.
