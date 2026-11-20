# Two-workspace sync E2E (agent-driven, local)

Everything runs on this machine against the local backend. No browser, no
Google, no Docker, no ClamAV, no staging.

## Status (what is verified)

| Step | Script | Status |
|---|---|---|
| Two isolated headless workspaces | `scripts/e2e/two-workspaces.sh` | verified |
| Temporary email/password account | `scripts/e2e/create-test-user.sh` | verified |
| Headless cloud sign-in per workspace | `scripts/e2e/cloud-login.sh` | verified (`signed-in`) |
| Sync enabled on both | `api call sync.enable` | verified |
| Change leaves A and arrives at B | `scripts/e2e/sync-two-workspaces.sh` (transport) | verified (`pushed: 1` on A, `pulled: 1` on B) |
| Change applied on B | same script (apply) | **open**: B reports `applicationDeferred: true` and keeps the plugin enabled |

The open item is the apply step. The transport works. Applying a plugin
selection to a headless host is deferred, and a second `sync.now` does not
apply it. Next investigation: what triggers the deferred apply (desktop running,
an idle tick, or a restart). Until then the E2E passes for transport only.

## Prerequisites

1. Backend: `cd RemakeBackend && cargo remake db up && cargo remake dev --no-watch`
   (listens on `http://127.0.0.1:3000`; its `APP_ORIGIN` must be that origin).
2. CLI: `cargo build -p tiktools-cli` in TikTools.
3. Development plugins (for the sync check): `bun run start` once, which stages
   `.dev-plugins/`, or set `TT_DEV_PLUGINS` to another plugins directory.

## Run

```sh
scripts/e2e/sync-two-workspaces.sh          # full scenario, prints transport and apply
scripts/e2e/stop-workspaces.sh              # stop every host started by these scripts
```

Single steps, for debugging:

```sh
scripts/e2e/create-test-user.sh e2e@example.test 'Pass-123!'
scripts/e2e/cloud-login.sh /tmp/ws-a tt-a e2e@example.test 'Pass-123!'
TIKTOOLS_HOME=/tmp/ws-a TIKTOOLS_IPC_NAME=tt-a tiktools api call cloud.status
```

## Rules that bit us

- Every client call needs the same `TIKTOOLS_HOME` and `TIKTOOLS_IPC_NAME` as its host.
- The backend rejects a request whose `Origin` is not its `APP_ORIGIN` (`403 request origin is not trusted`).
- The host's cloud session is memory-only: a restarted host must sign in again.
- `pkill -f` matches its own shell. Use `scripts/e2e/stop-workspaces.sh` instead.
- A plugin change needs at least one plugin installed: use the development plugins.

## For an AI agent

Use only these entry points: `tiktools api call <method>`, `tiktools api schema <method>`,
`tiktools api discover`, and the scripts above. Read the open item before reporting
sync as working.
