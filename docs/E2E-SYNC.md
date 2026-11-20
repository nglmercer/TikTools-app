# Two-workspace sync E2E (agent-driven, local, cross-platform)

Runs on this machine against the local backend. No browser, no Google, no
Docker, no ClamAV, no staging. The script is TypeScript run with Bun, so it runs
the same on Linux, macOS and Windows.

## Run

```sh
bun run e2e:sync                         # the full scenario
bun scripts/e2e/two-workspace-sync.ts --keep --plugin hotkeys
```

It creates a temporary email/password account, starts two headless hosts with
separate homes and IPC names, signs both in over HTTP (native handoff, no
browser), enables sync, disables a plugin in A, syncs both, and reports the
transport and the apply separately. The hosts are stopped on exit; `--keep`
keeps the scratch directories.

## Prerequisites

1. Backend: `cd RemakeBackend && cargo remake db up && cargo remake dev --no-watch`.
   It listens on `http://127.0.0.1:3000`, and its `APP_ORIGIN` must be that origin.
2. CLI: `cargo build -p tiktools-cli` in TikTools (`target/debug/tiktools` or `tiktools.exe`).
3. Development plugins: run `bun run start` once so `.dev-plugins/` exists, or set
   `TT_DEV_PLUGINS` to another plugins directory.

Environment overrides: `TT_BIN`, `TT_CLOUD_ORIGIN`, `TT_DEV_PLUGINS`.

## Status (what is verified)

| Step | Status |
|---|---|
| Two isolated headless workspaces | verified |
| Temporary email/password account | verified |
| Headless cloud sign-in per workspace (`signed-in`) | verified |
| Sync enabled on both | verified |
| Change leaves A and arrives at B (transport: `pushed: 1`, `pulled: 1`) | verified |
| Change applied on B | **open**: B reports `applicationDeferred: true` and keeps the plugin enabled |

The open item is the apply step. A second `sync.now` on B does not apply it.
Next investigation: what triggers the deferred apply (desktop running, an idle
tick, or a restart). Until then the E2E passes for transport only.

## Rules that bit us

- Every client call needs the same `TIKTOOLS_HOME` and `TIKTOOLS_IPC_NAME` as its host.
- The backend rejects a request whose `Origin` is not its `APP_ORIGIN` (`403 request origin is not trusted`).
- The host's cloud session is memory-only: a restarted host must sign in again.
- A plugin change needs at least one plugin installed: use the development plugins.

## For an AI agent

Use `tiktools api call <method> [params-json]`, `tiktools api schema <method>`
and `tiktools api discover`, or `bun run e2e:sync`. Read the open item before
reporting sync as working.
