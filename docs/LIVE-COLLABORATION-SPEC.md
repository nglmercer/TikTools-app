# Collaborative LIVE implementation specification

Status: version-1 target contract; partial experimental implementation.
Date: 2026-11-07. Last updated: 2026-11-08.
Companion: [product design and implementation roadmap](LIVE-COLLABORATION.md).

## 1. Normative language and boundary

MUST, MUST NOT and SHOULD describe implementation requirements, not release
claims. Implemented behavior is identified in
[the transport guide](LIVE-COLLABORATION-TRANSPORT.md) and
[the validation record](LIVE-COLLABORATION-VALIDATION.md). The full version-1
contract and end-to-end acceptance remain incomplete.

The [automation domain implementation](LIVE-COLLABORATION-AUTOMATION.md) tracks
versioned local behavior writers and exact-definition admission. It also lists
the implemented trusted provenance, disabled author-only drafts and explicit
local owner adoption, plus the pending delegated execution/approval contracts. Its local
consistency tests do not close COL-05 or COL-22 for remote execution.

The local behavior owner editor now pins revisions, retains conflicting drafts
and explicitly reviews base changes/recreation. Its two-owner browser/IPC test
does not prove the full host/moderator/agent or physical two-PC acceptance.

Version 1 covers a persistence-enabled authoritative host and concurrent
desktop, browser and agent controllers. LAN and Internet connections use
the same protocol when the advertised endpoint is reachable. Automatic host
discovery across the Internet, hosted relay infrastructure, peer-to-peer NAT
traversal, offline multi-master database merging, controller failover to host,
shared audio streaming and simultaneous character/canvas editing are outside
version 1. They MUST NOT be silently emulated by starting a second runtime.

## 2. Invariants

1. One execution host owns the LIVE, databases, plugins and side effects.
2. Controllers operate through `TikToolsClient -> ControlApi -> AppCore`.
3. Remote identity comes from validated transport credentials, never RPC params.
4. Every remote method and event is denied unless explicitly permitted.
5. All local and remote writers participate in the same revision/command rules.
6. Notifications and recovery snapshots never execute business side effects.
7. Reconnection never implies permission to repeat a mutation.
8. Host UI visibility and controller UI visibility are independent.
9. Cloud login and Cloud plugin-selection sync are independent of collaboration.
10. Permission revocation is enforced by the host, including existing sockets,
    asset routes, snapshots and queued work.

## 3. Identities, session lifecycle and storage

### 3.1 Identifiers

| Identifier | Lifetime / meaning |
| --- | --- |
| `hostId` | Stable random UUID for the host installation; not a credential |
| `hostInstanceId` | New random UUID on every host process start |
| `principalId` | Stable host-local identity for one approved controller or agent |
| `sessionId` | One independently revocable authenticated session |
| `connectionId` | One WebSocket connection; changes on reconnect |
| `inviteId` | One invitation; separate from its secret |
| `grantRevision` | Monotonic authorization revision for a principal |
| `commandId` | Client-generated UUID for one mutation intent |
| `subscriptionId` | One authorized event stream |

Human names and device labels MUST NOT be used as authorization keys. All
times use UTC Unix milliseconds; identifiers and versions are opaque strings.
Versions MUST NOT depend on wall-clock ordering.

### 3.2 Pairing state machine

`created -> redeemed_pending_approval -> approved | rejected | expired | revoked`.

- Only the trusted local owner creates, approves or revokes invitations.
- An invitation carries a random secret with at least 128 bits of entropy,
  expires after five minutes, is single-use, and contains an immutable maximum
  grant and optional resource restrictions. Store only a secret verifier.
- Redemption MUST atomically consume the invitation and create a pending
  request. Two redeemers cannot obtain two sessions. Approval cannot widen
  the invitation grant without a separate explicit owner action.
- The redeemer generates an ephemeral exchange secret. Approval status and
  credential collection require it; possession of a public request ID is
  insufficient. Expire pending exchanges after five minutes.
- Show the owner the requested profile/resources, client label and transport
  address. The owner approves the request, not a claimed Cloud identity.
- Rate-limit redemption and approval polling; cap pending requests at 32.
  Use generic rejection responses to avoid disclosing invitation existence.
- Join links use a URL fragment for the invitation secret. Remove the fragment
  with `history.replaceState` before loading other resources; send redemption
  via POST. Never place secrets in query strings or server access logs.
- The local adapter's six-digit pairing contract is unchanged. It MUST NOT
  become the authentication contract for an Internet-facing endpoint.

### 3.3 Credentials and revocation

Native clients use independent opaque access and refresh credentials, each
with at least 256 bits of entropy. Access credentials expire after 15 minutes;
refresh credentials after 24 hours of inactivity and seven days absolute.
Refresh rotation is atomic and single-use; reuse revokes that session.
Native clients store refresh credentials in the OS credential store. Serialize
refresh operations per session so two windows do not invalidate each other.

Browser sessions use a separate random credential in a `Secure; HttpOnly;
SameSite=Strict; Path=/` cookie with a host-only name and a maximum seven-day
absolute lifetime, 24-hour idle expiry and explicit logout. Browser sessions
do not expose native refresh credentials to JavaScript. Idle activity requires
an authenticated call or heartbeat, not merely an open TCP connection.

Store credential verifiers and expiry/revocation metadata on the host, never
plaintext credentials. A compromised device can be revoked without revoking
other controllers. Recheck session expiry and grant revision on every request
and event batch; a timer closes idle revoked/expired sockets within two seconds.
An action not yet started MUST be reauthorized before execution. Revocation
does not undo an already committed command or a physical effect already begun.

Restart closes connections and invalidates pending exchanges. Approved sessions
may authenticate again if still valid, but MUST obtain the new instance ID and
a fresh state baseline. Refresh credentials MUST NOT appear in app-state,
Cloud Sync, feed snapshots, audit payloads or error reports.

## 4. Host transport and trust

### 4.1 Listener

Implement an optional `remote-control` host feature. The host starts the
gateway against its existing `Arc<ControlApi>`. No CLI proxy may convert a
remote call into an unrestricted trusted-local IPC call. Keep `local-web`
and the existing local transport behavior separate.

Remote access is disabled by default. Enabling requires configured bind address,
port, advertised HTTPS origin and TLS certificate/key or an explicitly configured
TLS reverse proxy. A proxy backend binds loopback or a private authenticated
transport; validate its configured upstream boundary. Do not trust arbitrary
`Forwarded`, `X-Forwarded-*`, `Host` or `Origin` values. Advertised and allowed
origins are explicit normalized allowlists; no wildcard origins.

Native clients validate a trusted certificate or an owner-verified certificate
fingerprint supplied out of band. First-use pin approval displays the fingerprint;
pin changes require fresh approval. Browser clients require a certificate their
browser trusts, with operator instructions for local trust setup. No automatic
certificate-validation bypass is allowed.

### 4.2 Proposed HTTP endpoints

All responses use `Cache-Control: no-store` except immutable, authorized assets.
Bodies and responses are bounded. Pairing secrets and credentials are redacted
from logs at the HTTP layer.

| Route | Auth | Contract |
| --- | --- | --- |
| `GET /remote/v1/info` | Public | Protocol range, host ID/name, fingerprint hint; no LIVE/user data |
| `GET /remote/v1/join` | Public | Minimal pairing page; consumes fragment locally, no LIVE data or third-party resources |
| `POST /remote/v1/pair/redeem` | Invite secret | `{inviteId, secret, clientLabel, clientKind, exchangeSecretHash}` → `{requestId, expiresAt}` |
| `POST /remote/v1/pair/status` | Exchange secret | `{requestId, exchangeSecret}` → pending/rejected or one-time approved credentials/cookie |
| `GET /remote/v1/session` | Session | Safe bootstrap: caller identity/grant and browser CSRF token; never native refresh credentials |
| `POST /remote/v1/session/refresh` | Native refresh credential | Rotate refresh/access pair; browser uses authenticated activity instead |
| `POST /remote/v1/session/logout` | Session | Revoke only the caller's session and close its sockets |
| `POST /remote/v1/ws-ticket` | Session | Single-use, 30-second WebSocket ticket returned only as HttpOnly scoped cookie for browser |
| `GET /remote/v1/ws` | Access header or browser session + ticket cookies | Upgrade after auth, origin and limits checks |
| `GET /remote/v1/app/*` | Browser session | Existing Vue bundle with remote transport metadata |
| `GET /remote/v1/plugins/:pluginId/*` | Scoped asset capability | Confined installed plugin UI assets only |

`clientKind` is `desktop`, `browser` or `agent`, informational only. A redeemed
invitation and its exchange secret permit at most one credential collection;
lost credential collection requires another invitation, not an unlimited replay.

Native authentication uses an Authorization header on upgrade, never a URL.
Browser upgrade requires both the session cookie and the one-time ticket,
issued by an origin-checked POST. State-changing browser HTTP requests require
an exact approved Origin and a session-bound CSRF header token. Obtain that
token from an authenticated same-origin bootstrap response. Pairing redemption
and status use exact Origin checks for browser traffic and the exchange secret;
native requests use explicit client authentication flows, not an Origin bypass
that could turn ambient browser cookies into native access.

Native redemption, approval polling and refresh require
`X-TikTools-Client: native` and no Cookie or Origin headers. The header selects
the transport boundary; it does not authenticate the device or grant access.

Requests carrying session cookies MUST follow browser Origin/CSRF rules even
if they claim `clientKind: agent` or also carry an Authorization header. Native
credential exchange MUST NOT consume browser cookies. The join page uses a
restrictive Content-Security-Policy and `Referrer-Policy: no-referrer`; invite
fragments MUST NOT survive into analytics, crash reports or external links.
After native access expiry, refresh credentials through HTTPS and reconnect
the socket with the new access credential; an existing socket cannot extend
the access credential lifetime merely by remaining open.

Default limits: 8 authenticated connections total, 4 per principal; 1 MiB frame;
256 KiB RPC params; 32 pending RPCs per connection; 30 requests/second per
session with burst 60; pairing 5 requests/minute per source and 60/minute host
total. Reject overload explicitly. The owner may lower these limits. Large
lists/snapshots MUST paginate within limits rather than truncate silently.

Owner `collaboration.invites.list`, `requests.list`, `sessions.list` and
`principals.list` accept `{after?, limit?}` with an opaque returned ID cursor,
default limit 50 and maximum 100. Responses keep their named array and return
`nextAfter: string | null`. A byte budget can reduce the requested page size.
Follow `nextAfter` until null. These are live keyset pages over immutable IDs,
not a snapshot: restart without a cursor to discover newly inserted records.

### 4.3 Wire handshake

Preserve JSON-RPC 2.0 and existing `{method, params}` semantics. Version-1 remote
responses use standard `{jsonrpc, id, result|error}`; adapt to existing WebView
`rpc-response` envelopes only at the frontend transport boundary.

After upgrade the only allowed RPC is `collaboration.hello` until negotiation:

```json
{
  "jsonrpc": "2.0", "id": "hello-1", "method": "collaboration.hello",
  "params": {"protocolVersion": 1, "clientVersion": "4.x", "clientKind": "desktop"}
}
```

Result fields: `protocolVersion`, `hostId`, `hostInstanceId`, `connectionId`,
`principalId`, `sessionId`, `grantRevision`, effective `capabilities`,
`resourceRestrictions`, `serverTime`, `limits` and `features`.
An incompatible version returns `protocol_incompatible` and closes. New optional
fields are additive. Native clients use the registry schemas for API version
skew; browser assets are served by the host version. Missing features disable
their UI instead of attempting unsupported calls.

## 5. Authorization

### 5.1 Trusted execution context

Add `RequestContext` with trusted caller origin (`local-owner` or `remote`),
principal/session/connection IDs and current grant revision. Keep legacy
`execute` for trusted local callers; add a context-bearing execution path for
the remote gateway. Context is constructed by trusted code and MUST NOT be
deserializable from caller-controlled RPC fields.

Every registry method receives an explicit remote policy: deny, capability and
resource resolver, or authenticated connection-management operation. Missing
policy means deny, including future methods and generic `call_value` paths.
The existing `sideEffect`, `destructive` and `requiresDesktop` flags are
discovery metadata, not permission enforcement.

Every resource lookup MUST use host-resolved canonical IDs before checking
resource restrictions. Policy applies to reads, writes, discovery, snapshots,
event payloads and plugin assets. `rpc.discover` exposes only permitted methods;
schemas/results remain redacted. Unauthorized requests produce `forbidden`
without revealing whether a hidden resource exists.

### 5.2 Capability families

| Capability | Permitted domain | Additional restrictions |
| --- | --- | --- |
| `live.read` | LIVE status and sanitized feed/backfill | No TikTok cookies or unrestricted debug data |
| `points.read` / `points.adjust` | Leaderboard/viewer data / adjustments | Grant may limit absolute delta; reset excluded |
| `rankings.read` / `rankings.manage` | Ranking reads / configuration | Per-ranking IDs; reset/delete separate explicit grant |
| `competitions.read` / `competitions.manage` | Reads / approved state transitions | Per-competition IDs and transition permissions |
| `workflows.read` / `workflows.edit` / `workflows.toggle` | Read / save / enable-disable | Per-workflow IDs and delegated-effect checks |
| `automation.read` / `automation.edit` / `automation.toggle` | Behavior records and approved state | Same delegated-effect checks as workflows |
| `plugins.read` / `plugins.toggle` | Sanitized installed plugin status / activation | Per-plugin IDs; no install/update/uninstall |
| `plugins.settings.read` / `plugins.settings.write` | Redacted settings / schema-validated changes | Per-plugin and writable-field allowlists |
| `plugins.action.execute` | Approved plugin actions | Per-plugin, action ID and argument restrictions |
| `widgets.read` / `widgets.configure` | Widget reads / approved configuration | Scoped widget IDs and validated references |
| `commands.read` | Caller command outcomes | Other principals require local owner |

Remote host shutdown, Cloud account changes, publisher/registry/package
management, credential import/export, arbitrary `app.state`/globals access,
database/debug exports, filesystem paths and collaboration administration are
denied in version 1. Methods not covered by the completed policy inventory are
denied even if they appear harmless. Local-owner behavior remains available.

Workflow/behavior edits MUST NOT launder authority into automation execution.
For a save eligible for execution or enable, validate referenced actions,
scripts, plugins, resource IDs and host effects against delegated execution
grants. An explicitly disabled proposal MAY be stored before effect approval
only with trusted delegated provenance, author isolation and runtime quarantine.
Ordinary owner edits or enablement MUST NOT silently transfer its authority.
Unclassifiable effects and edits to unrestricted scripts require local-owner
approval before activation. Approved execution manifests are bound to the exact
definition revision/hash and grant revision; a changed definition invalidates
approval. A revoked grant stops future delegated executions. Existing owner
automations MUST NOT be modified through a weaker remote edit permission.

The owner-only `automation.manifest.get` read now describes exact saved
definitions/dependencies and current delegated subjects without approving or
executing them. Its output always has `executionAuthorized:false`. See the
[manifest and subject contract](LIVE-COLLABORATION-AUTOMATION.md#execution-subject-resolution-and-manifest-preview).
Host-resolved saved subjects are separate from access-token authentication:
socket close or token rotation does not create or revoke a persisted approval.
Current principal/session revocation, validated grant revision, host disable and
absolute/idle deadlines MUST be checked at final execution admission. Preview
and execution lookups MUST NOT extend session activity or persist bearer tokens.
An expired request snapshot MUST NOT be reused as a permanent execution grant.
The runtime now carries private context through typed event publication,
enrichment, queues, nested emits and memory, and denies unsupported scoped work
before observations or unrestricted reads/effects. Approval storage/invalidation
and approved effect-specific final-sink authorization remain incomplete. The
[approval/activation design](LIVE-COLLABORATION-APPROVAL.md) keeps exact bindings
over the final enabled state; it does not exclude enablement from the digest.

Secret-valued settings retain the existing placeholder-preserves-secret
behavior. Remote grants cannot reveal secrets; secret updates are denied by
default and require explicit field permission. Public metadata/event payloads
must never contain those values.

### 5.3 Plugin frames and presentation

Remote plugin UI MUST use the existing sandboxed declarative/broker model.
Intersection of plugin permissions, controller grant and resource restrictions
is the effective permission set. No raw RPC escape hatch, credential access,
host file path or same-origin privilege may be given to a plugin iframe.

Opaque-origin asset requests use short-lived scoped asset capabilities, issued
only after session authorization. Prefer a dedicated asset origin and isolated
cookies. Capabilities bind session, plugin ID, asset revision and expiry; they
are rechecked against revocation. Confine canonical paths and reject symlinks,
traversal and unknown assets. Do not pass the main session token to a frame or
embed capabilities in third-party URLs. Unsupported plugin UI stays disabled.

Remote clients MUST NOT call the desktop's `ui.projection` or frontend-ready
handshake. Add connection-scoped subscription/projection state; hiding one
controller cannot suppress events or stop playback on the host. Audio/TTS/native
input/window effects execute only on the host, including legacy host-push paths.
If an effect currently requires a host WebView, advertise that capability and
reject the command when the host presentation runtime is unavailable. Headless
hosts MUST NOT acknowledge an effect they cannot perform.

## 6. Commands, revisions and side effects

### 6.1 Resource versions

Each mutable remote resource has a persisted monotonically increasing integer
revision, serialized as a decimal string. Resource identity includes its kind,
scope (including creator/session where relevant) and canonical ID. Deleted IDs
retain a version tombstone so delete/recreate cannot accept an old revision.
Revision `"0"` means never created; reads of deleted resources return a permitted
tombstone/version response for editor recovery.

Behavior resource versions MUST use `kind:"automation"` and scope `"event"` or
`"action"`; equal raw IDs in the two tables are independent. Grant restrictions
in `resources.automation` MUST use qualified `event:<id>` / `action:<id>` IDs.
Ambiguous bare IDs MUST fail closed and MUST NOT imply both families. Global
behavior resources MUST NOT be exposed under a creator-restricted LIVE grant.
The implemented remote read remains a closed status summary. Durable
`automation.create/update` now store only explicitly disabled, author-owned
delegated drafts; receipts contain `{kind,id,enabled:false,pendingReview:true}`.
Owner and foreign definitions/tombstones remain protected. Runtime quarantine
includes every referenced action and manual/inline execution. Explicit
owner-only acknowledged CAS adoption transfers the selected definition to owner
authority and disables it; automatic operation requires a separate owner enable.
Explicit owner fire/test retains its usual rules after authority transfer. Owner
UI review now captures saved source/provenance/dependencies and uses a host-native
journal; its adoption command checks the complete reviewed versions atomically.
Remote native JSON proposals now use explicitly retained author copies, scoped
inspection tokens and fixed durable bridge operations. The
[native proposal contract](LIVE-COLLABORATION-CLIENT-COMMANDS.md#native-automation-proposal-editor)
specifies the private file format, consent, CAS and recovery boundaries. They do
not expose owner source or transfer authority. Positive delegated activation
remains pending under the execution requirements above.

Reads and successful writes add top-level response `meta.resourceVersions`,
an array of `{kind, id, scope, revision}` for the returned resources. Existing
domain `result` shapes remain unchanged, including arrays and booleans. Lists
include every returned item's version in that metadata. All
writers, including local WebView, CLI, plugin host effects, automation and Cloud
activation application, increment versions and publish invalidations through
the same domain coordination path. Local legacy calls may omit a precondition,
but still increment versions; remote edits MUST supply it.

Compare-and-set and business writes occur in one transaction in the owning
database. `workflows.save`, settings replacement, toggles, deletes and managed
configuration updates require `expectedRevision`. Two saves with the same
revision yield one success and one `revision_conflict`, never a silent overwrite.
Do not implement compare-and-set as an unlocked read followed by the current
unconditional upsert.

Additive operations such as points adjustments use a unique
command and a transaction, without requiring a stale leaderboard revision.
Competition transitions require their existing generation plus a revision
precondition. Each registry mutation declares its concurrency strategy
(`compare-and-set`, `additive` or `exclusive-transition`) and resource resolver.

### 6.2 Mutation metadata

Extend RPC requests with optional top-level `meta`; it is mandatory for remote
mutations and is not part of legacy method params:

```json
{
  "jsonrpc": "2.0", "id": "request-42", "method": "workflows.save",
  "meta": {
    "commandId": "b278ac5a-3f87-4da8-bf8b-21e16cce46f2",
    "hostInstanceId": "df0fed52-a270-4146-8544-29238a072c34",
    "expectedRevision": "17"
  },
  "params": {"graph": {"id": "welcome", "name": "Welcome", "enabled": false}}
}
```

The graph above illustrates the envelope only; actual graphs MUST pass the
existing domain schema plus delegated-effect validation. Successful mutation
responses retain the existing `result` shape and include top-level
`meta.command`: `{commandId, state, resource, committedAt}`. Read-only methods
need no command ID. Command IDs and resource versions are distinct from RPC
request IDs. Command IDs use 1–128 ASCII letters/digits, `-`, `_` or `:`; a UUID
is compatible. A metadata-aware SDK call returns both result and metadata;
legacy typed wrappers may continue to return just the domain result. Remote
editors MUST use metadata-aware reads rather than guessing versions.

Example successful acknowledgement (`result` abbreviated to show its shape):

```json
{
  "jsonrpc": "2.0", "id": "request-42",
  "result": {"id": "welcome", "name": "Welcome", "enabled": false, "graph": {}},
  "meta": {
    "hostInstanceId": "df0fed52-a270-4146-8544-29238a072c34",
    "command": {"commandId": "b278ac5a-3f87-4da8-bf8b-21e16cce46f2", "state": "succeeded", "committedAt": 1794000000000},
    "resourceVersions": [{"kind": "workflow", "id": "welcome", "scope": "host", "revision": "18"}]
  }
}
```

`meta.command.resource` is present for a single-resource command and omitted
for multi-resource/additive commands whose versions are listed separately.
Error responses use `error.data` for permitted current versions and command
state. The existing frontend and native client adapters MUST preserve response
metadata through correlation, including errors and reconnect resolution.

Reject stale instance IDs with `host_restarted` before executing a new command.
For a retained duplicate ID, authenticate and authorize first, then retrieve
the existing outcome without executing, even across a restart. An ID reused
with different canonical method/params/preconditions returns
`command_id_reused`. Store a deterministic payload hash, never secret params.

### 6.3 Durable outcomes

Command states are `accepted`, `running`, `succeeded`, `failed` and `unknown`.
Validation/auth/version failures before reservation have no accepted record.
Once accepted, a command can be queried using `collaboration.commands.get`.
Only its principal or the local owner can query it.

For transactional database operations, write the mutation, revision, final
command result and domain outbox entry in the same transaction. For external
effects, persist acceptance before dispatch, fence duplicate execution, then
record completion. If a crash leaves it unclear whether an external effect
happened, mark `unknown`; do not automatically execute it again. A controller
timeout/disconnect is an unknown client outcome until the host is queried,
not proof that an action failed. This specification does not promise exactly
once delivery to external systems.

Retain full command outcomes for seven days. After pruning, preserve compact
command-ID/hash tombstones for the principal's lifetime, so an old command ID
cannot execute again. A tombstone returns `command_expired` with an unknown
historical outcome. The initial host bounds retained IDs to 100,000 per principal (owner-lowerable
through `limits.commandIdsPerPrincipal`) and 1,000,000 per owning store.
Reject capacity before accepting a new command. Never evict
deduplication records to silently permit re-execution. Revoked principals can
be archived with their tombstones; their IDs MUST NOT be reassigned.

The controller never automatically retries an ambiguous mutation with a new
ID. An explicit user retry after reviewing `unknown` uses a new ID and states
that the prior physical effect may already have occurred.

### 6.4 Database boundaries and audit

Use a host-private `collaboration.db` for principals, credential verifiers,
invitations, grants and redacted audit indexing. Domain resource versions,
command records and outbox entries live in the same SQLite database as their
business write (`tiktok-points.db` or `tiktok-automation.db`). Administrative
commands use `collaboration.db`. Do not claim a transaction spans independent
database connections/files.

Outbox dispatch publishes versioned invalidations and copies audit metadata
idempotently into the audit index. At-least-once outbox delivery is permitted;
consumers deduplicate by event ID/resource revision. Restart can repeat an
invalidation but never rerun its business command. Multi-database operations
require explicit staged outcomes/compensation and remain unavailable remotely
until those semantics are defined and tested.

Implemented workflow delivery uses batches of at most 100 metadata entries,
reserving 50 slots each for resource invalidations and command outcomes. The
host worker checks every 80 ms, imports closed metadata before publication,
and acknowledges the source only after both steps succeed. A separate bounded
projection channel prevents retries from entering the business event bus.
Linked resource/command rows retain exact attribution, including no-op commands.
Fully acknowledged source metadata is removed while command-ID tombstones
remain. Workflow projections now back the subscription/snapshot/resume protocol
in section 7; other domains/feed and controller recovery remain incomplete.

Audit accepted commands and denied attempts with timestamp, principal, session,
method, resource IDs, command ID, result code, version transition and correlation
ID. Include owner-origin changes relevant to collaboration. Audit MUST exclude
raw chat, arbitrary settings/script bodies, credentials and invitation secrets.
Default audit retention is 30 days; deduplication retention is independent.
Imported audit retention uses the original commit timestamp: late retries cannot
resurrect expired records. Event IDs and payload hashes make import idempotent;
reuse of an event ID with different metadata rolls back the import batch.
The implemented store prunes on open, owner audit reads and imports; the host
also prunes an already-open store once a minute without opening one solely for
idle retention. Delivery failures retry with a two-second pause.
If a required transactional command/audit outbox record cannot be written,
fail the remote mutation before dispatching its effect.

## 7. Events, snapshots and reconnection

### 7.1 Subscribe before snapshot

After `hello`, call `collaboration.subscribe` with explicit allowed domains and
optional feed settings. The server subscribes first, creates a bounded recovery
buffer and returns a `subscriptionId`. Each connection can have one active
subscription in version 1; replacing it closes the old stream explicitly.

Then call `collaboration.snapshot` with that ID. The host captures authorized
state under a shared collaboration state barrier used by every relevant writer
and outbox publisher. The barrier covers only commit/version/publication and
snapshot capture, never network calls or external effects. Return a snapshot
ID, resource versions, `hostInstanceId` and `throughSequence` for that stream.
Changes committed after capture appear with a later sequence. Late outbox
invalidations for already-captured revisions are safe duplicates.

The host MUST implement this barrier or an equivalent tested consistency
mechanism. Independently reading several tables and then attaching a guessed
cursor is not a coherent snapshot. Large snapshots are immutable pages held
for 30 seconds with opaque principal/subscription-bound page tokens. A page
expiry or buffer overflow restarts recovery; no partial baseline is editable.

Clients apply the full snapshot, discard buffered events at/below its sequence
and apply later events in order. An invalidation includes resource version;
refresh the affected resource using its authorized read method. Newer versions
replace older cached versions. Permission-filtered snapshots never fetch then
expose forbidden resources in browser state.

An optional explicit display projection may narrow the points baseline:
`pointsLimit` on subscribe accepts 0–1,000 only with the points domain. Omitted
means the complete permitted viewer set; a positive value means a bounded
top-N display baseline; zero means no viewer rows and reliable invalidations
only. The host MUST echo the selected mode in subscribe and every snapshot
page, enforce the aggregate row bound and continue invalidating every permitted
viewer, including those outside the initial display. Clients MUST NOT interpret
an explicitly bounded display baseline as a complete viewer database.
Notifications-only clients finish the baseline before reading a bounded
absolute `points.view`, and refresh that read before restoring display after
every reconnect. They may ACK an invalidation after staging a dirty bit, provided
that bit schedules an absolute display read and reconnect also reads afresh.
This lossy display schedule is independent of command recovery and reliable
cursor order; it never sums events into balances or reruns an award.

### 7.2 Event envelope and retention

Preserve notification method `event` and topic/data, adding subscription fields:

```json
{
  "jsonrpc": "2.0", "method": "event",
  "params": {
    "topic": "workflow.changed", "data": {"id": "welcome", "change": "saved"},
    "hostInstanceId": "df0fed52-a270-4146-8544-29238a072c34",
    "subscriptionId": "stream-7", "sequence": "104",
    "eventId": "event-uuid", "resource": {"kind": "workflow", "id": "welcome", "scope": "host", "revision": "18"},
    "actor": {"principalId": "principal-uuid"}, "commandId": "b278ac5a-3f87-4da8-bf8b-21e16cce46f2"
  }
}
```

Sequence is contiguous per subscription's reliable stream after permission
filtering, not a global business revision. Replay preserves original sequence
and event ID. Keep reliable events for at most 60 seconds, 4,096 entries or
4 MiB per subscription, whichever limit is reached first. Keep replay state
for 60 seconds after disconnect, bound to principal, session, grant revision
and host instance. Enforce the total memory bound implied by connection limits.

Feed/UI projections are lossy and use a separate feed cursor/delivery counter;
they MUST NOT consume reliable sequence numbers. Coalesce absolute ranking,
points and competition display snapshots at up to four updates/second; do not
sum totals from notifications. Keep reliable invalidations/command outcomes
independent of display projection coalescing.

Use the existing bounded `ui.feed.backfill` for permitted render history.
Retained history is not complete LIVE storage. Expired/truncated history shows
a discontinuity marker; it never triggers points, audio or automation again.

### 7.3 Recovery and failure behavior

On reconnect authenticate and negotiate `hello`, then call
`collaboration.resume` with prior subscription ID, instance ID and last applied
reliable sequence. Same instance/session/grant with available retention returns
replay plus a live continuation. Otherwise return `resync_required` and create
a fresh subscription/snapshot baseline. Never replay a stream under an old
permission set. Permission changes invalidate the prior recovery buffer and
clear newly forbidden cached data on the controller.

While disconnected/resyncing, reject new mutations and preserve drafts with
their original base versions. Once current, resolve outstanding command IDs
before offering retries. If a draft's base changed, show local draft and current
host value; keep the draft, accept host, or explicitly merge and save against
the new revision. No automatic whole-object overwrite or silent draft deletion.

Reliable queue overflow emits `event.gap`/`resync_required` when possible, then
disconnects a persistently slow client. Socket writes have a five-second bound.
Feed shedding cannot block the host pipeline or command responses. Heartbeat
every 10 seconds, declare connection lost after 30 seconds without a response;
reconnect with exponential backoff from 500 ms to 30 seconds plus jitter.
Check revocation independently of heartbeat. A session refresh cannot increase
its grant or bypass the recovery handshake.

## 8. Proposed Control API additions

Implementation status: `collaboration.subscribe`, `snapshot` and `resume` now
support `workflows`, `points` and caller-owned workflow/viewer `commands`. Workflows
use their shared write/publication/capture barrier. Snapshot pages expire after
30 seconds and retain at most 4 MiB including envelopes/token indexes. Page
packing reserves the RPC envelope and serialized page metadata, caps its
state budget at 512 KiB and checks each serialized page against the negotiated
frame limit before publishing any baseline. Reliable
notifications are withheld until the complete baseline has been requested.
Resume acknowledges a watermark and replays retained notifications over the
same WSS channel, preserving IDs/sequences; it does not inline a multi-megabyte
replay result into one RPC frame. The host bounds active subscriptions to its
connection limit and total active/detached subscriptions to twice that limit,
evicting oldest detached state when necessary. Unsupported domain/feed requests
fail explicitly. Remaining section-7 requirements still apply to every future
domain and the controller lifecycle; this status does not narrow their scope.

The current gateway also permits `workflows.disable` and `workflows.delete`
with mandatory durable command metadata, `workflows.read` plus toggle/edit
capability and exact workflow resource scope. Identity comes from the approved
session, and is checked again after core and SQLite writer admission, before
resource access/replay. Deactivation has a bounded remote `{id,enabled:false}`
result, persisted with the command; deletion returns `{id,deleted}`. Owner
payloads remain compatible. Current capabilities and resources gate retries,
queries and response/conflict metadata. Replaying a committed operation returns
its original outcome even after owner reactivation or delete/recreation.
`hello.features` distinguishes `workflowDisable`/`workflowDelete` (implemented)
from `workflowSave`/`workflowEnable` (false); the generic `mutations` flag alone
does not authorize a particular operation. Save/activation, executable manifests,
other domain commands and journal integration for other mutations remain mandatory
pending work. Desktop disable/delete now uses a profile-bound durable native
intent, verified receipt/query and explicit unknown-outcome review. The raw
desktop forwarding bridge permits completed reads only; see
[desktop journal implementation](LIVE-COLLABORATION-CLIENT-COMMANDS.md) and
[SDK/CLI command usage](LIVE-COLLABORATION-CLIENT.md#remote-workflow-commands).

All methods MUST have typed params/results, JSON schemas, validator coverage,
SDK wrappers and parity tests. Networking-only ticket/credential exchange stays
in HTTP, not arbitrary generic RPC. Invitation secrets are returned only once
to the local owner and MUST follow the existing secret-redaction conventions.

| Method | Caller | Required contract |
| --- | --- | --- |
| `collaboration.status` | Local owner | Enabled/configured/online counts; redacted listener/trust status |
| `collaboration.configure` | Local owner | Enabled, bind/advertised origins, TLS reference, limits; optional exact `expectedRevision` checked atomically before changes/revocation; returns validated status |
| `collaboration.invites.create` | Local owner | Profile, capabilities, restrictions, expiry; one-time join secret/link |
| `collaboration.invites.list` | Local owner | IDs, grants, expiry/state; never reusable secrets |
| `collaboration.invites.revoke` | Local owner | Invite ID; invalidates unapproved redemption |
| `collaboration.requests.list` | Local owner | Pending redemption requests and proposed grants |
| `collaboration.requests.approve` / `reject` | Local owner | Request ID; approve binds immutable effective grant |
| `collaboration.sessions.list` / `revoke` | Local owner | Independent sessions and revocation by ID |
| `collaboration.principals.list` / `revoke` | Local owner | Approved identities; revoke all sessions and invitations for principal |
| `collaboration.principals.get` | Local owner | Exact principal ID → current bounded identity/grant/revision, independent of list pagination |
| `collaboration.grants.set` | Local owner | Principal, expected grant revision, capabilities and restrictions |
| `collaboration.audit.list` | Local owner | Bounded cursor pages, filters, redacted entries |
| `collaboration.hello` | Authenticated remote | Negotiation result in section 4.3 |
| `collaboration.me` | Authenticated remote | Current effective grant and redacted identity |
| `collaboration.subscribe` | Authenticated remote | Allowed domains/feed settings → subscription ID and bounds |
| `collaboration.snapshot` | Authenticated remote | Subscription/page token → immutable authorized state pages and watermark |
| `collaboration.resume` | Authenticated remote | Previous subscription/instance/sequence → replay or resync requirement |
| `collaboration.commands.get` | Caller or local owner | Command ID → retained state/result or expired/unknown outcome |

Administration calls follow the same command/audit model for local-owner
changes. A configuration change that would sever the owner's management path
must report its resulting endpoint before applying; local management always
remains available. Remote method policies for these administration calls deny
access, irrespective of role labels sent by a client.

The implemented owner settings panel always supplies the configuration revision
as `expectedRevision` (an exact decimal string returned by status). This wire-only
precondition is never persisted as configuration. Legacy calls without it remain
accepted. A revision conflict changes no configuration, sessions, invitations
or audit entries. Grant editors likewise submit the loaded grant revision.
Status includes the active direct-TLS certificate fingerprint and bounded
`sessionConnections` map; sessions include a descriptive `clientLabel`.
Administration command deduplication remains pending: the UI sends each mutation
once and blocks further mutations after an uncertain transport outcome until
explicit review. See [owner settings](LIVE-COLLABORATION-OWNER.md).

Topics added: `collaboration.session.changed`, `collaboration.grants.changed`,
`collaboration.command.changed`, `collaboration.presence.changed` and
`collaboration.resync.required`. Presence is advisory, expires on heartbeat
loss and cannot authorize a command. Session management detail is owner-only;
remote clients receive only their own grant/session/command changes and the
minimal presence data explicitly allowed by the owner.

## 9. SDK, CLI and frontend contracts

Extend the SDK with a transport/backend interface supporting existing direct
and IPC clients plus remote connections. The optional network adapter depends
on shared API types; avoid an API ↔ SDK ↔ gateway dependency cycle. A generic
call cannot bypass context, metadata or policy checks. Preserve the existing
single registry and typed-method parity invariant.

CLI pairing/profile selection and workflow recovery are implemented behind the
SDK `remote-profiles` and CLI `remote-client` features. Examples:

```bash
# Pair interactively; secret is entered privately, never as a process argument.
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
# Committed points reads, recovery streams and the native points UI are implemented.
tiktools --remote-profile live-desk api call points.leaderboard '{"limit":20}' --include-meta --format json
tiktools --remote-profile live-desk api events --topics 'workflow.changed'
```

Remote profiles contain endpoint, host ID, trust reference and credential-store
key, never plaintext refresh credentials. With a remote profile selected,
`--standalone` is a usage error. Destructive confirmation and dry-run behavior
remain enforced; confirmation is not a substitute for host permission. There
is no automatic fallback from remote to local. Pin/profile changes are explicit.

Implementation details and current bounds are in the
[native client guide](LIVE-COLLABORATION-CLIENT.md). Profile refresh writes a
durable nonsecret intent before the one-use request, saves a matching new
generation in the credential backend, then clears intent. A missing generation
after an uncertain refresh requires re-pairing, never replay of the old token.
Real platform keychain/filesystem and controller UI gates remain required.

Desktop controller startup creates a transport bridge and WebView window
without starting `AppCore`, scanning/executing local plugins, connecting to
TikTok, acquiring host ownership or opening host domain databases. Bridge RPCs
use the remote SDK and trusted host's connection context. A separate owner
window/process remains scoped to its own local transport.

Initial implementation: `tiktools-desktop --controller` or the desktop's
`--remote-profile NAME` option selects this boundary before host startup. The owner menu opens a
separate controller process. The native bridge and separate Vue root implement
profiles/pairing, workflow projection/recovery, host identity/connection state,
permission changes and scoped local drafts. Explicit private native draft saves
now survive page/profile reopen and support current-host comparison/local merge;
typing does not persist sensitive text without confirmation. A separate private
JSON proposal editor now submits disabled action/event create/update commands;
its version-4 journal preserves pending IDs and history from versions 1–3.
Other domain stores, the full
controller shell, desktop trust management and remaining domain
controls/journals remain incomplete. Confirmed workflow disable/delete with a
native durable journal, owner settings/approval
UI and scoped SDK/CLI workflow disable/delete are implemented separately;
workflow save, remote activation and other domain writes remain pending. Existing default local
startup is retained; this initial entry point does not satisfy all UI gates.

Each Vue store tree belongs to one host/connection scope. Replacing a host
disposes subscriptions, pending calls and stores before remounting. Session
credentials MUST NOT live in Pinia, route params, localStorage or plugin frames.
Native refresh credentials remain behind the native bridge; browser cookies
remain HttpOnly. Draft storage may persist nonsensitive edits keyed by host,
principal, resource and base revision, with opt-in handling for sensitive data.

Required UI: mode selection; host identity; owner invitation/approval/session
management; effective permissions; connection/resync state; last authoritative
update time; draft conflict comparison; command pending/unknown state; explicit
host-output labels for audio/TTS; unsupported-operation explanations. Hide or
disable inaccessible controls, but rely on host enforcement for security.

## 10. Stable error codes

Use the existing string-code API convention and structured `error.data`.
Never include raw params or credentials in errors.

| Code | Meaning / recovery |
| --- | --- |
| `unauthorized` | Missing/expired/revoked credential; refresh or pair again |
| `forbidden` | Capability/resource denied; request owner access |
| `pairing_expired` / `pairing_pending` / `pairing_rejected` | Pairing lifecycle without secret disclosure |
| `protocol_incompatible` | Version/feature negotiation failed |
| `revision_required` | Remote replacement/transition lacks precondition |
| `revision_conflict` | Stale draft; data contains permitted current resource version |
| `host_restarted` | New instance; recover baseline and command status |
| `resync_required` | Replay unavailable, buffer gap or grant changed |
| `command_id_reused` | Duplicate ID has different payload/precondition |
| `command_expired` | Dedup tombstone exists; historical result unavailable |
| `command_unknown` | Host cannot prove external effect completion |
| `rate_limited` / `overloaded` | Respect `retryAfterMs`; never replay mutations blindly |
| `unsupported_remote` | Method/effect unavailable for remote mode or host capability |
| `host_unavailable` | Host endpoint unavailable; no local fallback |

## 11. Performance and observability

These are release targets to measure, not current guarantees:

- On a LAN with measured RTT ≤20 ms, p95 host commit-to-controller application
  of reliable state invalidations ≤250 ms; p95 simple mutation acknowledgement
  ≤500 ms, excluding plugin/network execution time.
- On an Internet test path with measured RTT ≤150 ms and no injected loss,
  p95 reliable state application ≤750 ms. Report network and host timing
  separately; do not promise a universal Internet latency bound.
- At 1,000 synthetic input events/second for 10 minutes with 8 controllers,
  no lost committed business mutations, no duplicated host side effects and
  bounded per-client queues. Feed projection loss is counted and visible.
- Under the same workload, enabling controllers adds at most 10% to measured
  host event-processing p95 latency versus the same build with no controllers.
  Record hardware, event mix, warmup, memory/CPU and baseline/remote runs.
- Reliable backlog, replay/snapshot memory and pending calls remain within
  specified limits; a stalled controller cannot stall other clients or the LIVE.

Metrics include connected sessions, rejected/denied requests, grant changes,
RPC latency, event enqueue-to-send/apply latency, queue bytes/depth, dropped
feed projections, reliable gaps, recovery count/time, command state totals,
revision conflicts and audit/outbox lag. Diagnostics contain opaque correlation
IDs and permitted metadata, with credentials and sensitive values removed.

## 12. Acceptance and verification matrix

Use isolated application homes and synthetic secrets. Every requirement below
is a release gate; platform/network checks require actual execution evidence.

| ID | Scenario | Required result |
| --- | --- | --- |
| COL-01 | Owner plus moderator in the same app on two physical PCs | Both observe one LIVE and host-confirmed changes without manual Sync |
| COL-02 | Controller startup and local/remote window switching | No controller core, local plugin execution, TikTok session or domain DB writes |
| COL-03 | Viewer, moderator and scoped agent concurrently | Only explicitly granted calls/resources/topics are available |
| COL-04 | New/unclassified method, forged principal or generic RPC | Host denies access before effects; discovery cannot bypass policy |
| COL-05 | Workflow/script edit attempts a stronger action | Delegated-effect validation blocks it or requires local-owner approval |
| COL-06 | Invitation double redemption, expiry, rejection and stolen request ID | At most one approved exchange; no credential disclosure |
| COL-07 | Two independently paired sessions; rotate/logout/revoke one | Other session remains valid; revoked access stops within two seconds |
| COL-08 | Invalid TLS/pin, spoofed Host/Origin, browser CSRF and WebSocket hijack | Rejected with no validation bypass or ambient-cookie escalation |
| COL-09 | Two writers save revision 17, including a local writer | One revision-18 write and one conflict; local draft remains recoverable |
| COL-10 | Delete/recreate and delayed old draft | Tombstone version prevents stale write acceptance |
| COL-11 | Repeated point/action command ID, lost response and reconnect | One accepted operation; query returns outcome; no blind retry |
| COL-12 | Crash before/after DB commit and before/after external dispatch | Atomic DB outcome; external ambiguity becomes unknown, never auto-executed |
| COL-13 | Pruned command result and reused ID with changed payload | Expired/reused errors; old command cannot execute again |
| COL-14 | Snapshot capture races with local/remote writes and outbox delivery | No missing state update; duplicates deduplicated by revision/event ID |
| COL-15 | Short disconnect inside retention | Authorized reliable replay continues in order |
| COL-16 | Long disconnect, overflow, restart or grant change | Fresh snapshot required; no stale editing or forbidden cached data |
| COL-17 | Feed recovery and repeated notifications | No points/audio/automation replay; missing history marked visibly |
| COL-18 | One hidden/stalled controller and one active controller | Owner presentation and active controller continue; queues remain bounded |
| COL-19 | Plugin frame tries RPC escape, traversal or secret access | Denied; scoped assets expire/revoke; supported UI still works |
| COL-20 | Remote TTS/media/plugin action | Effect occurs on host only; unsupported headless effects fail explicitly |
| COL-21 | Domain operations across points/rankings/competitions/workflows/settings/widgets | Complete policy, revisions, audit, recovery and command-outcome coverage |
| COL-22 | Permission changes while command queued or automation scheduled | Queued/unstarted work reauthorized; delegated future executions stop |
| COL-23 | Cloud Sync conflict and collaboration enabled together | Unresolved Cloud record not applied; collaboration versions/events remain correct |
| COL-24 | Feature disabled, old local clients and migration/restart | Local operation preserved; remote sockets closed; additive data migration verified |
| COL-25 | Synthetic load and slow consumers | Section 11 targets and limits measured; no silent reliable state loss |
| COL-26 | SDK discovery/validator parity and CLI remote profile | Same permitted contract; no standalone/local fallback or secret logs |
| COL-27 | Audit/command storage failure and outbox retry | Mutation fails before unsafe dispatch or commits atomically; retries do not rerun effects |
| COL-28 | Host/controller combinations Windows, Linux and macOS; browser and agent | Per-platform evidence, native output/trust/keychain checks and documented unavailable checks |
| COL-29 | LAN and reachable Internet endpoint with TLS proxy if configured | Correct external origin, browser cookies, WSS upgrade, revocation and recovery |
| COL-30 | Secret settings, snapshots, errors, profile export and diagnostics | No credentials, invite secrets or raw sensitive values exposed |

Test layers: policy/DTO/state-machine units; SQLite migration/concurrency/crash
tests; authenticated HTTP/WSS integration; SDK/CLI parity; Vue store/transport
tests; Playwright browser flows; native two-PC desktop acceptance; controlled
LIVE validation. CI fixtures must not require production credentials.

For implementation PRs, run applicable checks from [CONTRIBUTING.md](../CONTRIBUTING.md):
Rust formatting/clippy/tests, frontend lint/typecheck/tests/build and desktop
cross-platform compilation. Documentation-only changes require link and
whitespace validation. Record the exact commands, tested commit and artifacts
in a future `LIVE-COLLABORATION-VALIDATION.md`; do not prepopulate passed results.
