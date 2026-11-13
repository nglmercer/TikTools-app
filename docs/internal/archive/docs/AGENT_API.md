# TikTools Agent API

One control surface for humans, scripts, and AI agents, with a typed Rust
SDK underneath and a CLI on top:

```text
        ┌───────────┐
        │  WebView  │
        └─────┬─────┘
              │
        ┌─────▼─────┐
        │ Control   │
        │ API       │
        └─────┬─────┘
              │  (ControlApi owns AppCore)
   ┌──────────┴──────────┐
   │                     │
┌──▼──────────────┐ ┌────▼──────────┐
│ TikToolsClient  │ │ direct host   │
│ SDK             │ │ tests         │
└──┬──────────────┘ └───────────────┘
   │
   ├──── CLI
   ├──── agents
   └──── integration tests
```

The invariant is:

```text
Control API registry = Typed client coverage = Agent-callable control surface
```

It is enforced, not documented: `tests/parity.rs` in `tiktools-client`
fails CI when a registered method lacks a typed wrapper (or vice versa),
and `tiktools api verify` checks the same invariant live against a
running host, catching CLI/host version skew at deploy time.

Routing rules:

- `TikToolsClient` is the only caller of `ControlApi` from the
  outside; `ControlApi` remains the only caller of `AppCore` methods.
- The CLI never constructs `AppCore` or calls `ControlApi`/`AppCore`
  methods on command paths. The only exceptions are standalone execution
  and headless host startup, which build `ControlApi` via the existing
  constructor.
- The typed client crate (`crates/tiktools-client`) depends only on
  `tiktools-control-api` plus `serde`/`tokio`: no desktop, no direct
  core dependency. Param/result types are re-exported through
  `tiktools_control_api::modules`.

## Typed Rust SDK (`tiktools-client`)

Local-owner collaboration administration is exposed through typed
`collaboration_*` methods and the generic CLI discovery/call surface. These
operations manage the host's private invitation/session/grant store. They
do not make an agent a remote host administrator. The remote network backend
is now available behind the SDK `remote`/`remote-profiles` features and the CLI
`remote-client` feature. The controller UI remains under implementation; see
[collaboration progress](LIVE-COLLABORATION-VALIDATION.md).

`call_with_meta` and `call_value_with_meta` preserve response revisions on the
direct, local IPC and remote backends. Pass `RequestMetadata.expected_revision`
for owner workflow compare-and-set; existing typed wrappers preserve their
domain-only return shape. See [version metadata](CONTROL_API.md#version-metadata).

Owner behavior automation calls now also return family-scoped revision metadata
through these generic SDK helpers. For single-record automation creates,
updates, deletes or enablement changes, pass the base `expected_revision` captured when reading the
specific action/event; retain that base while editing. On `revision_conflict`,
preserve the candidate and read/compare the current record. Never retry a missing
update as a new create automatically. Create, update, delete, enable, disable and
owner adoption support durable IDs with host-instance fencing and retained
outcomes. Durable create needs an explicit record ID. The
[automation specification](LIVE-COLLABORATION-AUTOMATION.md) describes runtime
authority and the remaining delegated-execution work.

The local behavior owner editor now uses these base-version rules, with retained
drafts, current-host comparison and separate reviewed recreation. Its UI support
does not yet attach durable intent IDs. Remote agents can separately read closed
`automation.list/get` summaries and issue durable `automation.disable/delete` for
complete events. Require `automation.read` plus toggle/edit and qualified
`event:<id>` resource permission. Remote `automation.get/list` always return
closed `{kind,id,name,enabled,definitionRedacted:true}` summaries, including for
the draft's own author. They never return editable definitions or source.

Remote automation authoring is available through generic metadata-aware SDK
calls or CLI `api call`, independently of the native controller UI:

- `automation.create {kind,record}` and `automation.update {kind,id,record}`
  require explicit `record.enabled:false`, `automation.read` and
  `automation.edit`, target family/ID permission, and `commandId`,
  `hostInstanceId`, `expectedRevision` together. A `live` resource restriction
  denies these global operations. `resources.automation` restrictions use
  `action:<id>` / `event:<id>`; omitting that restriction permits the whole
  automation domain.
- Create may use a new ID or the same author's retained tombstone at its current
  revision. Update replaces only a delegated record belonging to the same
  principal. Owner-owned or other-principal records/tombstones are protected.
  Action authoring is also denied when an owner or other principal's event
  references that ID, including disabled or unresolved references.
- A successful remote result is exactly
  `{kind,id,enabled:false,pendingReview:true}` with the committed family revision
  and receipt in `meta`. Keep the submitted definition in your own editor;
  summaries and receipts cannot reconstruct it. No remote full-definition read
  or owner snapshot is available.
- Resolve an unknown acknowledgement through the original
  `collaboration.commands.get` ID. This requires `commands.read` plus current
  automation read/resource access. An identical retained create/update can
  return its original pending-review result even after owner adoption; replay
  checks current authoring capabilities/scope but never reinstalls the old
  definition. The receipt is historical. A fresh post-adoption update is denied.

For example, save this params object to `/absolute/path/action-proposal.json`:

```json
{"kind":"action","record":{"id":"draft","name":"Proposed log","enabled":false,"typeId":"core.log","config":{"message":"Proposal"}}}
```

Submit it with a fresh command ID, then query that ID if the acknowledgement
is uncertain:

```bash
tiktools --remote-profile moderator api call automation.create @/absolute/path/action-proposal.json --expected-revision 0 --command-id UNIQUE_DRAFT_ID --host-instance-id HOST_INSTANCE --include-meta --format json
tiktools --remote-profile moderator api call collaboration.commands.get '{"commandId":"UNIQUE_DRAFT_ID"}' --format json
```

Use revision `"0"` only for a never-created ID. An update or deliberate tombstone
recreation uses the exact base revision captured for that family and ID; an
action version cannot stand in for an event version with the same raw ID.

The owner can read `automation.authority.get {kind,id}` through the typed
`automation_authority_get` wrapper, which preserves matching version metadata.
Its result is `{kind,id,provenance}`, with trusted authority
`{kind:"owner"}` or `{kind:"delegated",principalId,sessionId}` and a
decimal-string generation; never-created IDs have null provenance. The owner
must inspect the current full definition and its dependencies before calling
`automation_adopt(params, metadata)` or the local `automation.adopt` RPC with
`acknowledgeAuthority:true`, a durable command ID, current host instance and the
reviewed expected revision. Adoption returns `{id,adopted:true}`. Transferring a
delegated record disables it atomically and advances its version/generation; a
separate owner enable is required for automatic operation, and dependencies are
adopted individually. Explicit owner fire/test retains its usual rules after
authority transfer.
Both methods deny remote sessions. An already owner-owned target is a no-op.

Local owners can use typed `automation_review` to capture the full saved
definition, trusted provenance and all referenced actions with their complete
ordered version envelope. Pass that envelope as optional `reviewedVersions` to
generic adoption to fence dependency changes atomically. The new owner GUI does
this automatically and preserves the unsaved editor candidate.

Local owners can call `automation_execution_manifest_get` with strict
`AutomationManifestParams {kind,id}`. It returns a metadata-aware, validated
preview of saved definition/dependency/provenance bindings, every delegated
subject's current persisted grant revision and closed effect/render-field
classifications. It contains no source/config/scripts and always reports
`executionAuthorized:false`; it is not approval or an execution token. Caller
authority/body fields and command metadata are rejected. Remote profiles are
denied even with all capabilities. For an existing local host, inspect it with:

```bash
tiktools api call automation.manifest.get '{"kind":"event","id":"welcome"}' --include-meta --format json
```

Local Owners can inspect external-effect metadata with SDK
`automation_effects_get(EffectGetParams)` and
`automation_effects_history(EffectHistoryParams)`; `limit` is 1–100 and `before`
is an optional opaque receipt ID. CLI generic calls use
`automation.effects.get/history`. Remote profiles are denied. Inspection never
retries an unknown, expired or otherwise retained effect and exposes no request,
file/settings/response payload. Host restart recovery runs before these reads.

Saved subjects are resolved from trusted host provenance, not request tokens.
The lookup checks current identity, grants, revocation and session deadlines
without extending activity. Runtime context propagation, plugin bindings and
final-effect enforcement remain required before delegated activation.

Typed `automation_adoption_get/submit/resolve/review_unknown` wrap the owner's
host-native journal. Submit requires captured host/instance identity, complete
reviewed versions and acknowledgement; the host chooses/persists the command ID.
Resolve only queries the original result. One pending intent blocks new
coordinator submissions until its result is known or explicitly reviewed.
These four owner-only APIs reject caller command metadata; they do not belong
in the isolated remote controller's forwarding surface. See the
[owner coordinator contract](CONTROL_API.md#version-metadata).

Delegated records remain quarantined despite ordinary owner edits, enable/bulk
enable, deletion/recreation or manual fire/test. Editable JSON cannot claim
owner authority, and a disabled or unselected delegated action still blocks an
owner event that references it. Remote enable, bulk enablement, fire/test and
full snapshots remain denied. Manifest previews are implemented; approval and
execution under delegated grants remain incomplete. Owner adoption explicitly transfers execution
authority rather than widening the remote grant.

The native controller's automation board and raw `controller.forward` bridge do
not expose draft authoring or adoption. The board supports closed summaries and
its journaled event disable/delete operations. Existing local workflow draft
files are separate from the host's quarantined automation proposals.

For an explicitly selected remote profile:

```bash
tiktools --remote-profile moderator api call automation.list '{"kind":"all"}' --include-meta --format json
tiktools --remote-profile moderator api call automation.disable '{"kind":"event","id":"event-id"}' --expected-revision BASE_REVISION --command-id UNIQUE_ID --host-instance-id HOST_INSTANCE --include-meta --format json
tiktools --remote-profile moderator api call collaboration.commands.get '{"commandId":"UNIQUE_ID"}' --format json
```

Capture the base revision and instance from the current host, record the intent
before dispatch, and resolve the original ID after an unknown response. Do not
turn a summary into an editable record or automatically create a new stop/delete
command. Automation subscriptions use immutable summary pages and retained
metadata invalidations, matched by kind, family scope and raw ID.

Owner workflow commands now persist their result and deduplicate IDs in the
business transaction. CLI `api call` supports `--expected-revision`,
`--command-id`, `--host-instance-id` and `--include-meta`; query
`collaboration.commands.get` to resolve an acknowledgement. Scoped remote
points reads and additive `points.adjust` are now available. Supply command ID
and host instance without an expected revision; enforce the current points
read/adjust/viewer grant and delta limit. The server shares IDs/capacity across
workflow and points ledgers. See [points agent use](LIVE-COLLABORATION-POINTS.md#available-api-and-agent-use).
`points.info` supplies the unit label for confirmation. An optional
`params.expectedCurrency` fences a durable adjustment against a renamed unit;
`currency_changed` rejects before acceptance. Native typed points intents use a
version-2 journal migration that preserves pending workflow IDs and histories.
`points.view {limit?,uniqueId?}` returns bounded minimal committed display rows,
the selected viewer or null, unit label and matching versions. Subscription
`pointsLimit` explicitly requests a bounded display baseline (zero means only
invalidations); omitted means the complete permitted snapshot. The native points
view/form is connected. Remaining domain ledgers and external effects remain pending. Native remote profiles use OS credential
storage, explicit TLS trust and one-use refresh intent fencing; see the
[native client guide](LIVE-COLLABORATION-CLIENT.md). Typed
`collaboration_subscribe`, `collaboration_snapshot` and `collaboration_resume`
methods expose workflow/points WSS recovery, including mixed snapshots. Match
versions by resource kind, scope and ID; points-only clients need no workflow
capability. These calls require an
authenticated connection context; local-owner IPC is not a remote session.
Approved remote agents can now use durable `workflows.disable`/`delete` under
the read plus toggle/edit capabilities and exact resource scope. Save/enable
stay denied until delegated-effect policy is implemented. Use the metadata-aware
call or CLI `api call`; simple domain wrappers do not invent command metadata.
See [remote command usage](LIVE-COLLABORATION-CLIENT.md#remote-workflow-commands).
Reliable event envelopes preserve subscription/instance IDs, decimal sequences
and resource versions as `ControlEvent::Reliable`. A malformed reliable
notification forces a gap instead of becoming an unversioned domain event.
The SDK's optional `remote::controller::Controller` now owns the desktop's
scoped workflow projection and native lifecycle. Other domain/feed recovery
and the complete controller UI remain pending.

```rust
use tiktools_client::TikToolsClient;
use tiktools_control_api::modules::plugins::PluginIdParams;

// Against the running host over local IPC.
let client = TikToolsClient::connect().await?;

// Or in-process (hosts, tests): TikToolsClient::direct(api).

let plugins = client.plugins_list().await?;
let one = client
    .plugins_get(PluginIdParams { plugin_id: "demo".into() })
    .await?;

// One stream type on both backends.
let mut events = client.subscribe();
while let Ok(event) = events.recv().await {
    println!("{event:?}");
}
```

- One typed wrapper per registered method, with one module per domain (`automation_list`,
  `plugins_action_execute`, `rpc_discover`, ...). Methods whose params
  are `Empty` take no argument.
- `call` / `call_value` are the generic escape hatches for runtime-chosen
  methods; `validate_params` dry-validates params against the registry's
  own typed shapes with host-exact errors.
- `ControlEvent::Domain` carries authoritative events;
  `ControlEvent::Gap { lost }` means the stream skipped reliable events
  and the consumer must resync instead of reconstructing them.

## CLI: `tiktools api`

Domain commands (`plugin`, `automation`, `points`, `live`, `media`,
`system`, `workflow`, `processor`) resolve to one typed client call each;
`api` is the generic surface reaching 100% of the registry.

```bash
tiktools api discover                      # every method, live from the host
tiktools api schema plugins.settings.set   # description, flags, JSON schemas
tiktools api call plugins.list
tiktools api call points.adjust '{"uniqueId":"amy","delta":5}'
tiktools api events --topics 'live.*,points.changed' --max-events 10
tiktools api verify --sandbox              # coverage + smoke, isolated
```

Global flags: `--json` (stdout carries JSON only), `--standalone`
(isolated in-process runtime instead of IPC). Exit codes are stable:
0 ok, 1 operation error, 2 usage, 3 transport failure. Usage errors are
validated before any connection attempt, so they never depend on host
state.

Agent-safe `call` flags:

- `--dry-run` resolves the method and validates params without executing.
- Destructive methods (delete/reset/uninstall/shutdown) refuse to run
  without `--confirm` (`confirmation_required`, exit 1).
- `--timeout SEC` bounds the call (default is the 150 s host budget).
- `--explain` prints the call plan (method, description, flags, redacted
  params) to stderr; stdout keeps the pure result.
- `--format json|human` overrides `--json` for one command.

`api events` prints one `{topic, data}` NDJSON object per line until
`--max-events`, `--timeout`, or Ctrl-C (exit 0). Gaps print as
`{"topic":"event.gap","data":{"lost":N,"resync":true}}`.
Topic filters accept exact names, `*`, and `prefix.*` (which covers the
bare `prefix` too); `--topics` is repeatable and comma-separated.

## Verification

```bash
tiktools api verify --sandbox   # safe anywhere: throwaway runtime
tiktools api verify              # against the running host (or --standalone)
tiktools api verify --coverage   # registry-vs-SDK check only
tiktools api verify --smoke      # 21 read-only calls only
```

`--sandbox` forces an isolated `TIKTOOLS_HOME` that is removed
afterwards, so CI and agents verify without a host and without touching
real data. Reports print human-readable by default and machine-readable
with `--json`; any failure exits 1.

## Secrets

- `api call` output is secret-redacted by default (session cookies,
  passwords, tokens, API keys, credentials, ...); only
  `--secrets-visible` opts out, and `--redact` documents the default
  explicitly. `--explain` always redacts.
- `live connect` / `live pick` read the session cookie from
  `--session-cookie` or `TIKTOOLS_SESSION_COOKIE` (the flag wins). The
  value never appears in usage text, errors, or explanations.
- Plugin settings stay on the host-safe endpoints: `plugins.settings.get`
  returns schema plus redacted values, and saving the placeholder back
  preserves stored secrets. Never raw SQL or debug stores.

## Host-safe endpoints

Verification and agents use the control surface exclusively. The smoke
list is read-only by construction; mutating-method coverage belongs to
the host test suite, never to a CLI pointed at live data.
