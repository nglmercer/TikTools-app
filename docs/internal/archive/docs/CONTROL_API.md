# TikTools Control API

Remote moderator/agent access is being implemented, specified in
[Collaborative LIVE management](LIVE-COLLABORATION.md) and
[the implementation specification](LIVE-COLLABORATION-SPEC.md). The remote
gateway provides experimental authenticated HTTPS/WSS reads, scoped durable
workflow/event disable/delete, additive points adjustments and author-owned
disabled automation drafts. Owner adoption is a separate local operation;
remote activation remains unavailable. Owner-only `automation.approval.activate`
and `automation.approval.revoke` now expose durable checked approval for saved
delegated log/delay/points definitions; see the
[approval contract](LIVE-COLLABORATION-APPROVAL.md) for exact parameters, final
bindings, native Owner review/recovery methods and pending effect policies.
`automation.approval.review|get|submit|resolve|review_unknown` now support the
Owner dialog without caller-generated command IDs. Local Owner responses have
a separate bounded 8 MiB allowance for complete reviews. The new LocalOwner
`automation.library.submit` alone accepts up to 8 MiB of params and 8 MiB plus
64 KiB of request framing; other request and all remote gateway limits are
unchanged. Atomic profile/template library contracts are implemented under
validation as described below. Remaining domain
writes and complete controller acceptance remain pending. Workflow, points and
automation subscriptions, coherent summary pages and replay are implemented.
Local-owner
administration (`collaboration.status|configure`, invitations, device approval,
sessions, principals, grants and audit) is implemented with typed SDK coverage.
In hosts built with `remote-control`, enabled configuration starts the gateway
against the existing core. See [transport setup and limitations](LIVE-COLLABORATION-TRANSPORT.md).

One authoritative operation path for every client:

```text
client -> ControlApi -> AppCore service
```

Clients: WebView, CLI (`tiktools`), JSON stdio (`tiktools host --stdio`),
local IPC (desktop host or `tiktools host --ipc`), integration tests, AI
agents, and approved remote sessions in `remote-control` hosts.

There is exactly one runtime: the desktop builds one `AppCore`, wraps it in
one `ControlApi`, and serves every client (WebView bridge, local IPC, CLI)
from that same instance. Standalone hosts (`host --stdio`, `host --ipc`,
`--standalone`) are explicit opt-ins for headless use.

## Protocol

Local transports use JSON-RPC-style NDJSON: one request per line, one response
per line. The remote gateway uses one JSON-RPC 2.0 message per WebSocket frame
and requires `collaboration.hello` before ordinary calls.

Request:

```json
{"jsonrpc": "2.0", "id": "1", "method": "plugins.list", "params": {}}
```

Success:

```json
{"jsonrpc": "2.0", "id": "1", "result": {}}
```

Error:

```json
{"jsonrpc": "2.0", "id": "1", "error": {"code": "plugin_not_found", "message": "Plugin `x` is not installed."}}
```

Event notification (interleaved when streaming is enabled):

```json
{"jsonrpc": "2.0", "method": "event", "params": {"topic": "plugin.progress", "data": {}}}
```

Ordinary limits: 1 MiB per request line, 256 KiB params, 150 s per request.
LocalOwner `automation.library.submit` alone allows 8 MiB params and 8 MiB plus
64 KiB request framing; remote allowances are unchanged.

## Version metadata

Workflow reads and writes return their persisted versions in top-level
`meta.resourceVersions`; the existing domain `result` shape is preserved.
`workflows.save`, `delete`, `enable` and `disable` accept an optional top-level
`meta.expectedRevision` for local-owner compare-and-set:

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.save","params":{"graph":{"id":"welcome","name":"Welcome","enabled":false,"nodes":[],"edges":[]}},"meta":{"expectedRevision":"0"}}
```

Revision `"0"` means never created. A successful first save returns
`meta.resourceVersions: [{"kind":"workflow","scope":"host","id":"welcome","revision":"1"}]`.
Local legacy writers may omit the precondition but still increment revisions.
A stale precondition returns `revision_conflict` with permitted current
versions in `error.data.resourceVersions`. Deletion keeps a version tombstone;
recreating an ID cannot accept an older revision. Missing `workflows.get`
keeps `workflow_not_found` and includes the tombstone or never-created version
in its error data. Discovery marks these writes `concurrency: "compare-and-set"`.

Graph, revision and an invalidation outbox row commit in one SQLite transaction.
Owner workflow writes also support durable commands: supply `commandId`,
`hostInstanceId` and `expectedRevision` together. Discovery identifies the four
implemented writes with `durableCommands: true`. The result keeps its domain
shape and `meta.command` contains the original receipt; `meta.hostInstanceId`
identifies the currently answering host. An identical retained command returns
its original result/revision after another edit or host restart. It does not
apply the operation again. A retained receipt is historical; use a fresh
versioned read for current resource state. A different canonical method, params or precondition
using that ID returns `command_id_reused`. A new ID sent to an old instance
returns `host_restarted` before writing anything.

The command result, version and minimal audit reservation share the business
transaction in `tiktok-automation.db`. Full results last seven days; compact
ID/hash records remain and return `command_expired` with unknown historical
outcome. Retention runs at startup, once a minute and during command lookup or
submission. `limits.commandIdsPerPrincipal` defaults to 100,000 retained IDs
and can be lowered to 1; the host caps IDs at 1,000,000 across the owning ledgers. Capacity
rejects new commands with `overloaded`; it never evicts IDs to allow replay.
Workflow delivery imports metadata idempotently into the private audit index,
then publishes to a bounded internal projection channel. Retries retain event
IDs without executing business actions. Owner `collaboration.audit.list`
entries include `details` with the command/resource version transition and no
graph body. Audit retention uses original timestamps and defaults to 30 days;
fully delivered source metadata is removed independently of command-ID records.
Transactional workflow and points commands are implemented. Remaining domain ledgers,
external-effect `running`/`unknown` recovery and invalidation recovery for other
domains remain pending. Workflow/points recovery uses `collaboration.subscribe`,
`collaboration.snapshot` and `collaboration.resume`; see the
[wire protocol and bounds](LIVE-COLLABORATION-TRANSPORT.md#subscription-and-recovery).

`points.adjust` is additive. Its durable metadata contains `commandId` and
`hostInstanceId`, without `expectedRevision`. The balance, viewer revision,
history, exact outcome, ID/hash and audit/invalidation reservations commit
together in `tiktok-points.db`. IDs and admission capacity are shared across
both business ledgers under the host coordinator. A workflow ID cannot be
reused for a points command by the same principal, or vice versa.

Owner `automation.list/get/snapshot/create/update/delete/enable/disable` also
return `meta.resourceVersions`, using `kind:"automation"` and distinct scopes
`"action"`/`"event"` for identical raw IDs. Single-record writes accept optional
`meta.expectedRevision`; legacy unconditional writers still advance revisions.
Create rejects existing IDs, update requires an existing record and delete
retains a tombstone. Action deletion atomically updates dependent event
references and their versions without changing canonical enablement.
`automation.set_enablement` applies both families atomically, caps a batch at
512 updates and rejects a singular revision precondition.

Automation discovery advertises compare-and-set and durable commands for
create, update, delete, enable, disable and owner adoption; bulk remains an
exclusive transition. A durable write requires `commandId`, `hostInstanceId`,
`expectedRevision` and an explicit record
ID (including create). Commands share the workflow/points namespace and recovery
contract. Remote `automation.list/get` return only
`{kind,id,name,enabled,definitionRedacted:true}` per record. Neither author access
nor successful submission exposes a full definition, config or script. Remote
`automation.disable/delete` apply only to complete events with read plus toggle/
edit capability, current `event:<id>` permission and durable metadata. Recovery
adds an `automation` summary array and `automation.changed` metadata under the
existing subscription protocol.

Remote `automation.create/update` accept only an explicit JSON
`record.enabled:false`, with `automation.read` and `automation.edit`, current
permission for the target family/ID and all three durable metadata fields.
Use `action:<id>` or `event:<id>` in a `resources.automation` restriction;
the families are independent even when their raw IDs match. Omitting that
resource restriction leaves the automation domain unrestricted. Any `live`
resource restriction denies these global automation operations. A new author
may create an unused ID, then replace only delegated records belonging to the
same principal. Owner-owned IDs and another principal's records or retained
tombstones cannot be claimed. An author's own deleted ID can be recreated only
against its retained tombstone revision. Action writes also reject references
from owner-owned or other-principal events, including disabled events and
references to an action that has not yet been created.

The remote draft result has exactly four fields:

```json
{"kind":"action","id":"draft","enabled":false,"pendingReview":true}
```

`meta.resourceVersions` and `meta.command` identify the committed family/ID and
original revision. `collaboration.commands.get` returns that same minimal
historical result under `commands.read` and current automation read/resource
permission. On an unknown response, query the original command ID; never invent
a replacement ID to repeat the operation. Identical retained commands remain
replayable after owner adoption, subject to current write/read/resource checks,
and cannot overwrite or reinstall the later owner definition. Their historical
`pendingReview:true` does not describe the current authority. Fresh updates
after adoption are forbidden to the previous author.

The host stores authority separately from editable definition JSON. Claims such
as `record.authority` or `record.provenance` never confer authority. Delegated
records stay quarantined from runtime execution through owner save, enable,
bulk enablement, deletion/recreation and manual fire/test. An owner event cannot
bypass a delegated action dependency, including a disabled or unselected random
step. Remote `automation.enable`, `automation.set_enablement`, fire/test and
`automation.snapshot` remain denied. Effect manifests and execution under a
delegated grant are not implemented.

The trusted local owner can inspect `automation.authority.get {kind,id}`. It
returns `{kind,id,provenance}` plus the matching resource version. Provenance
contains `authority:{kind:"owner"}` or
`authority:{kind:"delegated",principalId,sessionId}`, and a decimal-string
`generation`; a never-created ID has `provenance:null` and revision `"0"`.
Retained tombstones keep their provenance. This read rejects mutation metadata
and is never available to remote sessions.

After reviewing the current full definition and dependencies through the owner API, the owner
can call `automation.adopt {kind,id,acknowledgeAuthority:true}` with a fresh
command ID, current host instance and the reviewed `expectedRevision`. Missing
acknowledgement is rejected; durable metadata is mandatory. A stale revision
returns `revision_conflict` without transferring authority. Successful adoption
of a delegated record atomically transfers authority to the owner, advances its
revision/generation and disables that record, returning `{id,adopted:true}`.
It adopts only the selected record. Each delegated dependency requires its own
review and adoption. Automatic operation requires a separate owner enable.
Explicit owner fire/test retains its usual enablement and test rules after
authority transfer. Adopting an
already owner-owned record is a no-op at its current version. This explicit
transfer authorizes future owner execution; it does not enlarge a remote grant
or approve delegated effects. Both adoption and authority inspection have typed
SDK wrappers and remain local-owner only.

`automation.review {kind,id}` is an owner-only atomic source/provenance read.
It returns `{kind,id,record,provenance,dependencies:[{id,record,provenance}]}`;
envelope versions contain the primary followed by all distinct referenced
actions in first-occurrence order, including disabled/random steps and missing
or deleted IDs. Null bodies distinguish absent definitions without discarding
retained versions/authority. Review is limited to 512 references, 256 KiB per
definition and 4 MiB total; malformed or inconsistent selected state rejects
without a partial body. The typed SDK validates complete bindings and closed
authority shapes. Remote sessions cannot read this source.

`automation.manifest.get {kind,id}` is a separate owner-only, strict-parameter
read. It returns a bounded execution preview with
`{policyVersion,digestAlgorithm,hostId,subjects,primary,dependencies,effects,reasons,executionAuthorized,manifestDigest}`
and the same complete ordered resource versions in metadata. Source/config/scripts
are absent. Definition bindings include exact family/ID/revision, presence,
canonical content digest and trusted provenance/generation. Subjects bind all
delegated principal/session pairs and current persisted grant revisions; stale,
revoked or expired saved sessions cannot become Owner authority. Preview neither
touches session activity nor accepts bearer tokens, caller subjects, command
metadata or approval payloads.

Effects and reasons are closed classifications, not execution permits.
`executionAuthorized` is always false. Unsupported plugin bindings and effect
policies, dynamic targets, opaque scripts, unrestricted global reads and missing
references remain explicit. No approval/activation endpoint is introduced by this
read. All remote sessions are denied at ingress and egress. The metadata-aware
SDK `automation_execution_manifest_get` validates the complete closed envelope
and family/version/dependency/subject relationships. See the
[manifest contract](LIVE-COLLABORATION-AUTOMATION.md#execution-subject-resolution-and-manifest-preview).

Owner-only native definition recovery uses the following strict methods:

| Method | Parameters | Result |
| --- | --- | --- |
| `automation.definition.get` | `{}` | Native metadata state |
| `automation.definition.submit` | Closed `operation:create|update|delete|enable|disable` union | `{state,acknowledgement}` |
| `automation.definition.resolve` | `{}` | Original-ledger lookup and metadata state |
| `automation.definition.review_unknown` | `{commandId,acknowledgeUnknown:true}` | Explicit uncertainty review and metadata state |

Submit carries `kind`, canonical `expectedRevision`, reviewed `hostId` and
`hostInstanceId`. Create carries an explicit-ID `record`; update adds matching
`id`; other variants carry `id`. The host creates/persists the business command
identity before dispatch. Caller command metadata is forbidden, and remote
principals are denied at both ingress and egress even with every capability.
Revisions are nonnegative decimal strings: absent delete at revision `0` may
return `deleted:false`, while an absent update never becomes a create.

State contains host identity, one optional pending intent and at most 16 recent
metadata records. Intents expose opaque command/operation/target/base/host and
optional source digest; receipts expose final revision/digest/enablement/delete
and affected-resource count. Source, grants and credentials are absent. Only a
fresh successful submit may include an acknowledgement with the original
`{value,resourceVersions,command,hostInstanceId}`; recovery never returns source
or writes state again. Missing/expired outcomes stay pending until explicit
review; an existing known result wins over an unknown acknowledgement.
The generic CLI requires `--confirm` for submit because its union includes
deletion; ordinary in-app Save retains its existing explicit button action.

The native journal shares pending admission with adoption, approval and library.
Create/update source is bounded to 256 KiB/depth 64/65,536 nodes; toggle output
and action-delete cascade versions are checked under the writer before effect
to keep native recovery inside the finite Owner response budget. Action deletion
may include all affected event versions, rather than one primary-only receipt.
Typed SDK methods are `automation_definition_get`, `_submit`, `_resolve` and
`_review_unknown`. See the [Owner editor and recovery contract](LIVE-COLLABORATION-AUTOMATION.md#native-owner-definition-intents).

## Native Owner library API

These five strict LocalOwner-only methods are implemented contracts under
validation. Remote sessions are denied at ingress and egress, even with every
capability; the isolated controller bridge cannot forward them. Caller command
metadata is forbidden.

| Method | Parameters | Result |
| --- | --- | --- |
| `automation.library.snapshot` | `{}` | `{hostId,hostInstanceId,revision,definitionDigest,catalog,definitions,resourceVersions}` |
| `automation.library.get` | `{}` | Metadata-only `{hostId,hostInstanceId,pending,recent}` |
| `automation.library.submit` | `{mutation,expectedRevision,definitionDigest,hostId,hostInstanceId}` | `{state,acknowledgement}` |
| `automation.library.resolve` | `{}` | Original-result lookup and metadata state |
| `automation.library.review_unknown` | `{commandId,acknowledgeUnknown:true}` | Final original-result lookup and explicit uncertainty archival |

The closed `mutation.operation` union is `create-profile`, `switch-profile`,
`delete-profile`, `apply-template`, `import-profile`, `assign-record`,
`import-templates` or `delete-template`. An apply carries `{entry,profileId}`
and optional `additionalEntries`/`templates`; a profile import carries
`{profile,entries}` and optional `templates`. Entries contain exact staged
`{actions,event}` bodies/IDs. Optional gallery and graph additions commit with
profile registration in the same business transaction. See the
[complete operation fields and semantics](LIVE-COLLABORATION-AUTOMATION.md#atomic-owner-library-transitions).

Catalog is `{packs,activeId,templates}`. Snapshot captures catalog plus all
saved action/event source and qualified versions coherently. Catalog CAS and
definitionDigest both fence submit, including newly appearing/deleted graph IDs.
The host generates the native command ID before dispatch. Fresh success may
return an acknowledgement `{value,resourceVersions,command,hostInstanceId}`;
value is `{revision,catalogDigest,definitionDigest,versionsDigest,affectedResources}`.
The catalog version is first, then unique final action/event versions sorted
by `(scope,id)`; count includes catalog. Recovery returns at most 16 recent
metadata records and never source, another dispatch or historical state install.
All four Owner journals share pending admission. Unknown outcomes require
explicit archival and current review before fresh work, with no ID regeneration
or automatic retry. The generic CLI submit requires `--confirm`.

The three reserved legacy profile/gallery app-state keys migrate once from
PointsDB into a revision-zero AutomationDB catalog, retaining old values.
Legacy reads project canonical state; typed single-key compatibility writes
commit catalog/version/outbox before acknowledgement. Corruption fails closed.
The native library transaction atomically commits graph/catalog, provenance,
approval invalidations, versions, original command result and outboxes; it does
not grant delegated execution or transfer authority.

Library-only SHA-256 canonical JSON sorts object keys by Unicode scalar value,
keeps array order and exact native integer decimals, and uses ECMAScript
shortest finite floats (`1.0` → `1`, `-0` → `0`). Command hash input is
`{method:"automation.library.apply",params:{mutation,definitionDigest},expectedRevision}`.
Definition digest includes full ID-sorted actions/events and sorted qualified
versions; versionsDigest hashes the exact primary-first vector. Other protocol
hashes are unchanged. Browser review rejects unsafe source integers instead of
approving rounded data.

Snapshot/source is bounded to 8 MiB, catalog to 4 MiB, each new record/template
to 256 KiB/depth 64/65,536 nodes and explicit new graph batches to 512 targets.
There is no 512-record cap on existing workspaces or bulk affected results.
Final version bytes reserve 256 KiB for native responses/history; precommit
snapshot admission also reserves measured runtime-catalog overhead and 8 KiB
of envelope headroom. Any oversized business result rolls back. Typed SDK
methods are `automation_library_snapshot`, `_get`, `_submit`, `_resolve` and
`_review_unknown`. CLI template/profile imports use this atomic flow, including
optional gallery saves. Evidence is recorded in
[validation](LIVE-COLLABORATION-VALIDATION.md).

## External effect receipts

Owner-only `automation.effects.get {effectId}` and
`automation.effects.history {before:null|effectId,limit:1..100}` inspect bounded
metadata-only external-effect receipts. Results expose opaque identity, effect
kind (`http`, `audio`, `plugin`, `emit` or `script`),
accepted/dispatched/succeeded/failed/unknown state, timestamps and expiry.
They contain no request, destination, file path, settings or response data.
All remote sessions are denied at ingress and egress, including all-capability
sessions. Caller command metadata is rejected. Retained/expired/unknown effects
cannot be retried through these operations; no replay operation exists. The
SDK validates closed receipt shapes, identity/cursor/order and result bounds.
See [external-effect policy](LIVE-COLLABORATION-APPROVAL.md#implemented-external-effect-grants-and-handoff).

Local `automation.adopt` optionally accepts `reviewedVersions` from that read.
The complete primary/dependency list is checked in the authority-transfer
transaction and included in the semantic hash. A changed action can therefore
reject adoption without changing the event first. Omission preserves the legacy
primary-only owner contract and original hashes.

The owner's Automations UI uses these additional owner-only native coordinator
methods, all of which reject caller-selected top-level command metadata:

| Method | Params | Result / behavior |
| --- | --- | --- |
| `automation.adoption.get` | `{}` | Host ID/instance, pending intent and at most 16 recent records |
| `automation.adoption.submit` | `{kind,id,expectedRevision,hostId,hostInstanceId,reviewedVersions,acknowledgeAuthority:true}` | Host persists its own intent ID before adopting the exact review |
| `automation.adoption.resolve` | `{}` | Reads the original result; never dispatches adoption |
| `automation.adoption.review_unknown` | `{commandId,acknowledgeUnknown:true}` | Final lookup, then metadata-only explicit uncertainty archival |

Their state is `{hostId,hostInstanceId,pending,recent}`. Intent fields are
`{commandId,kind,id,expectedRevision,hostInstanceId,createdAt}`. Each recent record
is `{intent,outcome,revision,errorCode,recordedAt}`, where outcome is `succeeded`,
`rejected` or `reviewed-unknown`. Missing revision/error values are null. One
pending intent blocks fresh coordinator submissions across owner windows.
Private metadata is stored on the host, without definition source or browser
storage. A known success wins over a stale request to archive uncertainty.
Durable top-level command fields on these non-forwarding methods return
`unsupported_remote`; unsupported top-level revision preconditions return
`invalid_params`. Submit/get/resolve parameter objects are closed schemas.
These methods have typed SDK wrappers and remain denied to remote sessions and
the native controller bridge. Adoption never enables the definition.

The behavior owner editor now captures base revisions on selection,
preserves conflicting/unconfirmed drafts and reviews current definitions or
tombstones before a separate submission. It never recreates a missing update
automatically. See the
[automation consistency and delegated-execution specification](LIVE-COLLABORATION-AUTOMATION.md)
for implemented runtime admission and remaining authorization gates.

Approved remote sessions need `points.read` plus `points.adjust`, their exact
viewer grant and any configured absolute-delta limit. Both requested and
subscriber-adjusted amounts obey that limit. Remote `points.viewer.get` and
`points.leaderboard` return committed data with matching versions; local-owner
legacy wrappers preserve their optimistic behavior and envelope. All global
points calls deny creator-restricted LIVE grants. Reset/configuration methods
remain unavailable remotely. Points outcomes use `collaboration.commands.get`
with `commands.read` and current points/viewer read access. Points/mixed-domain
snapshots and replay are available. Native typed points intents and a preserving
version-2 journal migration, bounded live view and confirmed adjustment form are implemented. See
[the complete points contract](LIVE-COLLABORATION-POINTS.md).

`points.info {}` returns `{currencyName}` under the same points-read/global-scope
policy. Durable adjustments optionally supply `params.expectedCurrency`; native
intents always do. A changed unit label rejects with `currency_changed` before
acceptance. The field participates in the hash when present; existing unchecked
hashes are preserved, and historical replay retains the original unit. A local
call without durable metadata cannot silently ignore this precondition.

`points.view {limit?,uniqueId?}` is a committed minimal display read for both
owner and remote callers. Limit defaults to 20 and clamps to 0–1,000; zero
returns only the optional lookup. It returns
`{currencyName,viewers,selectedId,selected}` and metadata for the unique union
of board/selected IDs, including a permitted missing ID's zero version. Rows
contain only ID, nickname, points, level and subscriber flag; profile images
and counters are never loaded. Ranking, permission filtering, selected row and
versions share one SQLite transaction and a negotiated serialized byte bound.

`collaboration.commands.get {commandId, principalId?}` retrieves a retained
outcome. The trusted local owner can select another principal. Remote callers
can select only themselves and need `commands.read`, the target domain
read capability and permission for that exact resource; successful and expired queued responses
are reauthorized. Remote workflow save/enable remain denied while delegated effect
policy is incomplete. Methods without a durable
implementation reject command metadata with `unsupported_remote`. Incomplete
metadata or unsupported preconditions return `invalid_params` before execution.

The Rust SDK and IPC client expose `call_with_meta` / `call_value_with_meta`;
most domain wrappers return just the domain result. `automation_adopt` accepts
durable metadata and returns the envelope; `automation_authority_get` also
retains its response metadata. Use the generic metadata-aware call for remote
automation create/update. The Vue bridge exports `callWithMeta` and preserves
metadata through correlation.
The native WebView and optional local browser adapter retain the envelope.
Native profiles, local workflow drafts and event stop/delete controls are
implemented. The native controller has no automation authoring or adoption UI;
its raw forwarding bridge still rejects those operations.

The local IPC CLI accepts `--expected-revision`, `--command-id`,
`--host-instance-id` and `--include-meta` on `api call`. Obtain the current
instance from `collaboration.status` and revisions from a metadata-aware read.
Use `"0"` only for an ID that has never existed. For example, with a running host
and params in a local JSON file:

```bash
tiktools api call workflows.save @/absolute/path/workflow-params.json --expected-revision 0 --command-id UNIQUE_ID --host-instance-id HOST_INSTANCE --include-meta --format json
tiktools api call collaboration.commands.get '{"commandId":"UNIQUE_ID"}' --format json
```

Keep the same command ID and payload when resolving a lost acknowledgement.
The CLI does not retry automatically or create replacement IDs. `--dry-run`
checks the call plan and never reserves a command. Default output still returns
just the domain result; `--include-meta` returns `{result, meta}`. JSON errors
preserve redacted `error.data`, including conflict versions and unknown outcomes;
`--format json` also applies to operation/transport errors. Command IDs
use 1–128 ASCII letters/digits, `-`, `_` or `:`; a generated UUID fits this
contract. Without a selected remote profile, these options connect to the
running host over local IPC. An explicit `--remote-profile NAME` selects the
configured remote backend in CLI builds with `remote-client`.

## Discovery

```bash
tiktools api discover
tiktools api schema plugins.settings.set
tiktools rpc rpc.discover '{}'   # raw JSON-RPC equivalent
```

`rpc.discover` returns every method with its description, `sideEffect`
flag, `destructive` and `requiresDesktop` risk flags, and JSON Schemas for
params/result, so agents can work without hardcoded knowledge.

## CLI

```bash
tiktools plugin list
tiktools plugin settings set demo.plugin serverUrl=http://localhost:8080
tiktools automation list
tiktools live status
tiktools points leaderboard
tiktools rpc plugins.list '{}'
echo '{"id":1,"method":"plugins.list"}' | tiktools rpc --stdio
```

All commands accept `--json` (stdout = JSON only, stderr = diagnostics,
exit 0 ok / 1 operation error / 2 usage / 3 transport failure).

Commands run against the running host over local IPC by default and fail
with `host_unavailable` when no host is running; they never silently start
a second runtime. `--standalone` opts one command into an isolated
in-process runtime instead. `tiktools workflow list` manages graph
workflows; every other method is reachable through `tiktools rpc`.

Every command resolves to one typed `TikToolsClient` SDK call (see
`docs/AGENT_API.md` for the SDK, the agent-oriented `api`
discover/schema/call/events/verify surface, agent-safe flags, and secret
handling). The registry, the typed client, and the agent surface are
pinned to each other by parity tests and `tiktools api verify`.

## Headless host

```bash
tiktools host --stdio [--events]   # AppCore without Winit/Wry/tray
tiktools host --ipc                # Unix socket / Windows named pipe
```

The IPC endpoint is `$TIKTOOLS_HOME/tiktools-control.sock` on Unix and
the per-user `tiktools-control-<user-sid>` named pipe on Windows, speaking
the same NDJSON protocol as stdio. The desktop host serves this endpoint
itself from its one `AppCore`; `host --ipc` refuses to start when a host is
already listening. The Unix socket is created owner-only (`0600`); the
Windows pipe carries a current-user-only DACL. Integration tests isolate
Windows pipes with `TIKTOOLS_IPC_NAME`.

## Methods

`rpc.discover`, `rpc.schema`, `system.info|health|snapshot|doctor|shutdown|ping`,
`plugins.list|get|install|uninstall|enable|disable|start|stop`,
`plugins.install.set`, `plugins.settings.get|set|reset`,
`plugins.packages.inspect|install|update|rollback|versions|receipts|policy`,
`plugins.updates.check|list`, `plugins.jobs.list|get|cancel`,
`registry.list|root|add|remove|enable|refresh|search|package`,
`registry.defaults.get|set`,
`plugins.health|options|action.execute`, `plugins.token.provision|import`,
`processors.list|status|test`, `live.connect|pick|disconnect|status|channels`,
`live.monitor.start|stop|status`,
`points.config.get|set`, `points.viewer.get`, `points.adjust`,
`points.leaderboard|reset`, `app.state.get|set`, `globals.list|get|set|delete`, `creators.get|recent`,
`creators.history.clear`, `analytics.summary`, `gifts.list|debug`,
`workflows.list|get|save|delete|enable|disable`,
`automation.list|get|create|update|adopt|delete|enable|disable|set_enablement|test|fire|context|runs|snapshot`,
`automation.authority.get`,
`automation.review`, `automation.manifest.get`, `automation.adoption.get|submit|resolve|review_unknown`,
`automation.definition.get|submit|resolve|review_unknown`,
`automation.nodes.list`, `automation.script.analyze`,
`media.validate|play|pick`.

The Vue frontend speaks JSON-RPC through `src/web/platform/control-client.ts`
(one bridge owns `window.ipc`, request ids, timeouts, `rpc-response`
routing, and `event` dispatch) with domain state in `src/web/features/`.
WebView JSON-RPC (`{"method": ...}`) is answered as
`{"type": "rpc-response", ...}`, and domain events stream to the WebView as
`event` notifications on the same channel. The legacy `{"type": ...}`
`PageMessage` path remains only as an explicitly marked compatibility
layer; every legacy message is an adapter over the same control
operations, and no new legacy variants are accepted. The
`tests/parity.rs` suite fails if any legacy operation lacks a Control API
equivalent.

## Safety rules

- Operations only: no database handles, plugin instances, WebView
  objects, filesystem handles, or native TikTok objects cross RPC.
- Secrets stay redacted; settings placeholders preserve stored values.
- Behavior-record tables are a closed allowlist (`behavior_events`,
  `behavior_actions`); table names never come from caller input.
- `plugins.action.execute` defaults to a dry run (`live: true` executes).
