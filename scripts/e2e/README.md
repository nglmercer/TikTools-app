# Two-workspace sync E2E (agent-driven)

Goal: an agent (or a person) creates its own TikTools workspace on this machine,
signs it in to Cloud, changes data, syncs, and checks that a second workspace
receives the change. Everything below is reachable from the CLI, so no UI is
needed.

## Phases

1. **Isolated workspaces (done).** `scripts/e2e/two-workspaces.sh` starts two
   headless hosts (`tiktools host --ipc`), each with its own `TIKTOOLS_HOME` and
   `TIKTOOLS_IPC_NAME`, and checks that each answers from its own home and pid.
   The desktop app is not needed: sync lives in the core.

   Rule: every client call needs the same `TIKTOOLS_HOME` and `TIKTOOLS_IPC_NAME`
   as its host, otherwise the client reports `host_unavailable`.

2. **Sign-in per workspace (blocked, decision needed).** `tiktools cloud login`
   is a browser OAuth flow (`cloud.login.begin` → browser → `cloud.login.complete`).
   There is no automated sign-in yet. Two options:
   - a dev-only mock OAuth provider in RemakeBackend, gated by an environment
     variable and never built for production, so an agent can complete sign-in;
   - one manual Google sign-in per test workspace; afterwards the agent can drive
     everything else.

3. **Change and sync (ready once phase 2 works).** Commands the agent can run:
   - Data: `tiktools points`, `globals`, `automation`, `plugin`.
   - Sync and any other method: `tiktools api call <method> [params-json]`
     (`sync.enable`, `sync.now`, `sync.status`), `tiktools api schema <method>`
     to see parameters, `tiktools api discover` to list methods.

4. **Remote control (product path).** A signed-in workspace can bring another
   workspace in with `tiktools cloud pair` / `cloud connect`. An agent that wants
   to help remotely should use this flow, not a shortcut.

## Run phase 1

```sh
cargo build -p tiktools-cli
scripts/e2e/two-workspaces.sh          # exits 0 when both workspaces are isolated
scripts/e2e/two-workspaces.sh --keep   # keep the scratch directory for inspection
```

The script stops both hosts on exit and removes the scratch directory unless
`--keep` is given.
