# Experimental collaboration gateway

Status: implemented HTTPS/WSS scoped reads, workflow/event stop/delete,
additive points commands and disabled automation drafts, 2026-11-08. This is an
engineering setup guide, not acceptance of the complete two-PC product.
See [the product plan](LIVE-COLLABORATION.md),
[the normative specification](LIVE-COLLABORATION-SPEC.md) and
[executed validation](LIVE-COLLABORATION-VALIDATION.md).
For the implemented local settings panel, use the
[owner administration guide](LIVE-COLLABORATION-OWNER.md).

## Available behavior

`tiktools-remote` serves the existing host's `Arc<ControlApi>`. It never creates
another `AppCore`. Desktop's default features include `remote-control`;
the headless CLI enables it explicitly:

```bash
cargo run -p tiktools-cli --locked --features remote-control -- host --ipc
```

Use either the desktop or one headless host for a workspace. Local IPC remains
the owner administration channel. The feature makes the gateway available;
remote access remains disabled until the owner configures it. A local-only
fresh idle startup does not create a collaboration database or bind a remote
socket. Workflow changes lazily open the private database for their audit index,
even when remote access is disabled.

Owner settings use revision-checked configuration, private invitation display,
approval and per-session/principal revocation. Status reports the active direct
TLS fingerprint and per-session socket counts. These are owner-only diagnostics;
listener replacement fences retired connections from changing current counts.

Implemented: stable host identity; independent approved device sessions;
one-use invitations; grants; per-session refresh/revocation; redacted audit;
TLS; native bearer and browser cookie boundaries; a minimal browser join page;
mandatory protocol handshake; configurable connection/request/frame limits;
host startup, reconfiguration and shutdown integration.

Current remote RPC allowlist: `collaboration.hello`, `collaboration.me`,
`rpc.discover`, `rpc.schema`, `live.status`, `live.monitor.status` and
`ui.feed.backfill` and `ui.feed.creator`, plus `collaboration.commands.get` for the caller's
historical results under `commands.read` and the corresponding workflow/points/automation
read and resource checks. `workflows.list`/`get` are now scoped remote reads, and
`collaboration.subscribe`/`snapshot`/`resume` implement workflow recovery.
`workflows.disable` and `workflows.delete` accept durable, revision-checked
remote commands. They require `workflows.read`, the corresponding
`workflows.toggle`/`workflows.edit` capability, and an allowed workflow ID.
See [remote workflow commands](LIVE-COLLABORATION-CLIENT.md#remote-workflow-commands).
The native desktop exposes confirmed controls for those same operations with a
[profile-bound durable journal](LIVE-COLLABORATION-CLIENT-COMMANDS.md).
Capability and resource checks can further restrict this allowlist.
`ui.feed.backfill` remains unavailable to creator-restricted grants because
its cursors cover the shared history. `ui.feed.creator` instead checks the
specific creator against the current grant and returns a closed, bounded
projection with a creator-local cursor and a random stream generation. The
host rechecks permission immediately before sending the response. Scoped
history is indexed only when the trusted LIVE ingress generation matches the
captured host connection context; merely adding `creatorId` to JSON or
delivering an unstamped event cannot assign it to a scoped feed. Each creator
retains up to 64 sanitized events independently, so traffic from another
creator cannot evict those rows. At most 64 creator streams are retained;
eviction on creating a further stream can reveal that retention was lost, but
does not identify other creators or their event counts. A changed stream,
creator-local eviction or restart reports a gap. The history is not complete
LIVE storage. If a secure stream identifier cannot be generated, scoped
history reports unavailable until host restart; it never silently presents a
partial feed as empty. The ordinary local feed remains available.
Provider diagnostics in `live.monitor.status.lastError` are replaced with a
generic failure message remotely. Local owner diagnostics remain available.

The owner can use durable workflow commands through local IPC and SDK/CLI
metadata options. Remote workflow writes permit disable/delete; save/enable
remain denied until delegated-effect validation is implemented. See
[the implemented command contract](CONTROL_API.md#version-metadata) for retries,
quotas and expiry. Workflow outbox delivery copies closed audit metadata
idempotently, then publishes invalidations and command receipts to an internal
projection-only channel. Interrupted delivery can repeat the same event IDs;
it never reruns automation or plugins. Authorized workflow subscriptions consume
these projections through a separate bounded recovery journal. The owner can inspect imported metadata through
`collaboration.audit.list` (`details` on workflow entries).

`points.viewer.get` and `points.leaderboard` now return scoped committed viewer
data and versions. `points.adjust` accepts additive durable metadata with
command/host-instance IDs and no CAS precondition. Points read/adjust grants,
viewer restrictions and requested/effective bonus limits are enforced. Global
points data stays unavailable to creator-restricted grants. Reset/configuration
stay denied remotely. The points ledger, audit delivery and original-outcome
queries and points snapshot/replay are implemented. Command subscriptions
include caller-owned workflow and viewer receipts under the appropriate read
grant. The native points journal/coordinator, bounded live view, viewer lookup
and confirmed adjustment form are implemented. `points.view` provides minimal
committed display rows. Optional subscription `pointsLimit` selects an explicit
display baseline (zero means invalidations only); omission keeps the complete
permitted baseline. Native clients read their bounded display after that baseline
and after reconnect. The handshake advertises `pointsProjection`. See
[points implementation and agent use](LIVE-COLLABORATION-POINTS.md).

Remote `automation.list/get` and automation recovery expose only closed
family-scoped status summaries. `automation.disable/delete` operate on complete
events. `automation.create/update` accept only the caller's explicitly disabled
drafts, with current read/edit grants, qualified resource permission and complete
durable metadata. Trusted provenance quarantines these definitions; responses
contain only `{kind,id,enabled:false,pendingReview:true}`. Remote enablement,
action stop/delete, full definitions, fire/test and owner adoption are denied.
See the [automation contract](LIVE-COLLABORATION-AUTOMATION.md). This adds no
RemakeBackend endpoint or sync collection.

Remote executable definition activation, remaining domain writes/subscriptions/feed, the complete Vue browser
app, plugin UI assets and the complete desktop controller remain pending.
An isolated desktop entry point now provides native profiles, pairing and a
scoped workflow view; see the [client guide](LIVE-COLLABORATION-CLIENT.md).
`collaboration.hello.features` advertises `subscriptions` as
true in persistence builds. `mutations`, `workflowDisable` and `workflowDelete`
are true in persistence builds; `workflowSave`, `workflowEnable` and
`browserApp` remain false. `automationSummary`, `automationEventDeactivate` and
`automationDrafts` are true with persistence; these distinguish closed status,
complete-event stop/delete and disabled author-only draft submission.
Capability/resource checks still apply to every call;
the broad `mutations` flag does not authorize arbitrary writes.
`subscriptionDomains` currently contains `workflows`, `points`, `automation` and `commands`.
A successful pairing page alone does
not make the current desktop a remote controller.

The [native SDK/CLI transport and secure profiles](LIVE-COLLABORATION-CLIENT.md)
are implemented separately from the host gateway.

## Subscription and recovery

On an authenticated WSS connection, negotiate `collaboration.hello`, then send:

```json
{"jsonrpc":"2.0","id":2,"method":"collaboration.subscribe","params":{"domains":["workflows","commands"],"feed":false}}
{"jsonrpc":"2.0","id":3,"method":"collaboration.snapshot","params":{"subscriptionId":"<returned ID>"}}
```

Request `workflows` with `workflows.read`, or `points` with `points.read`;
points-only clients need no workflow capability. Points remain unavailable
under creator-restricted LIVE grants. Requesting `commands` additionally needs
`commands.read` and at least one supported domain read capability. Each receipt
is independently filtered by its domain/resource permission and principal.
Invalidations omit another principal's command ID. Unknown/forbidden resources
never consume reliable sequence numbers. Other domain names or `feed:true`
return `unsupported_remote` until their state writers/projections are adapted.

The snapshot result contains `snapshotId`, `hostInstanceId`, `grantRevision`,
`throughSequence`, `workflows`, optional nonempty `points`, `resourceVersions`
and `nextPageToken`. Versions identify rows by `(kind,scope,id)`, not by a
shared workflow/viewer ID or positional alignment across mixed arrays. Follow
each token with `collaboration.snapshot {subscriptionId,pageToken}`. Pages are
immutable for 30 seconds; complete the whole baseline before allowing edits.
The host withholds notifications until all baseline pages have been requested.
Pages reserve half the configured frame budget for state, capped at 512 KiB;
snapshots retain at most 4 MiB including page/token metadata. An individual
record that cannot fit fails with `too_large`; nothing is silently truncated.
If the page count cannot fit even the configured burst plus every token that
could refill during the 30-second snapshot lifetime, capture fails before
returning the first page with `snapshot_rate_budget`. Narrow the resource
grant or increase the configured frame or request limits before retrying.
This is an optimistic upper bound: concurrent requests, network latency,
capture time and an initially depleted bucket can still cause expiry below
that threshold. A larger request limit does not extend retained event history.
Points snapshots are complete permitted sets, not the 1,000-row leaderboard
window. A large full set can exceed the snapshot budget; narrow its viewer
grant. Use the snapshot method when the bounded `workflows.list` response is too large.

Notifications use `method:"event"` and carry `topic`, `data`, `subscriptionId`,
`hostInstanceId`, decimal-string `sequence`, `eventId`, `resource`, `actor` and
an optional caller-owned `commandId`. Apply resource versions monotonically;
late invalidations can describe a version already included in the baseline.
The Rust SDK event type preserves this metadata as `ControlEvent::Reliable`.

Reconnect with the same approved session, negotiate hello again, then send:

```json
{"jsonrpc":"2.0","id":4,"method":"collaboration.resume","params":{"subscriptionId":"<prior ID>","hostInstanceId":"<prior instance>","lastAppliedSequence":"<last fully applied sequence>"}}
```

The reply acknowledges `afterSequence`, `throughSequence` and `replaying`;
replay follows as ordinary notifications with original IDs/sequences before
live continuation. Native credential rotation can preserve the logical session.
Another session/principal, changed grant or host instance, expired history,
incomplete baseline, evicted recovery state or a cursor outside retention
returns `resync_required`: clear the old cache and subscribe/capture again.

There is one subscription per connection; replacing it invalidates the old ID
and page tokens. Reliable history is capped at 60 seconds, 4,096 entries or
4 MiB per subscription. At most the configured connection count is active and
twice that count is retained in total; oldest detached streams can be evicted.
This bounds serialized journals to 64 MiB and active snapshots to 32 MiB at the
eight-connection maximum, plus object/transport overhead. Unsent overflow emits
`event.gap` when possible and closes the connection. Grant changes invalidate
queued projections before send. The initial native desktop controller applies
this workflow protocol and retains separate local drafts; complete domain
recovery and editing/conflict UI remain pending.

## Configure a reachable HTTPS host

Prepare a certificate chain and matching private key for the advertised host.
The certificate must be trusted by the connecting client and cover the host
name or IP used in its URL. No certificate-validation bypass is provided.
Certificate/key files must be regular files, at most 64 KiB each, with no
symlink at the file path. On Unix the private key must have no group/other
permissions, for example mode `0600`.

Save the following as a local configuration file, replacing the example
address and absolute certificate paths:

```json
{
  "enabled": true,
  "bindAddress": "0.0.0.0:8788",
  "advertisedOrigin": "https://pc-a.example:8788",
  "allowedOrigins": [],
  "tlsCertificate": "/absolute/path/certificate-chain.pem",
  "tlsPrivateKey": "/absolute/path/private-key.pem",
  "tlsProxy": false
}
```

Apply it through the running host's local IPC and inspect its status:

```bash
tiktools --json rpc collaboration.configure @/absolute/path/remote-config.json
tiktools --json rpc collaboration.status '{}'
```

Configuration persists separately from Cloud Sync. `configurationRevision`
identifies the requested configuration. `gateway.available` means this host
build has the component; `gateway.running` means a socket is listening.
`gateway.configurationRevision` identifies the applied generation. Applying
configuration is asynchronous: the supervisor normally notices it within
500 ms. Check `gateway.lastError` if enabled configuration cannot start.
Changing the bind address, TLS or limits restarts only the gateway and closes
its connections. Reapplying the same configuration also reloads TLS files;
replace the certificate/key pair first, then reapply. Disabling collaboration
revokes all remote sessions and pending invitations/exchanges without stopping
the LIVE. Re-enabling requires new pairing.

For TLS termination in a separately managed reverse proxy, configure
`tlsProxy: true`, an HTTPS `advertisedOrigin`, no certificate/key paths, and a
loopback `bindAddress`. Forward HTTP/1.1 WebSocket upgrades and preserve the
configured public Host and browser Origin. Only loopback proxy connections are
accepted. `Forwarded` and `X-Forwarded-*` do not establish identity or change
source rate-limit keys; a shared local proxy shares the source IP budget.
Proxy installation, external routing and certificate issuance are not automated.

Public plaintext listeners are rejected. Loopback HTTP without TLS is supported
only for local protocol development; secure browser cookies require HTTPS.
`allowedOrigins` contains exact normalized origins, never wildcards. Additional
origins are not a general CORS/browser-app hosting solution. The pairing page
is served on the advertised host origin.

## Pair and approve a browser

Create an invitation through the owner's local IPC:

```bash
tiktools --json rpc collaboration.invites.create '{}'
```

The result contains `inviteId`, a one-time `secret` and `expiresAt`. This is the
only owner response containing that secret. Construct the browser link locally:

```text
https://pc-a.example:8788/remote/v1/join#inviteId=INVITE_ID&secret=SECRET
```

The join script removes the fragment before making requests. Enter a device
label on the joining browser, then review and approve the pending request on
the host:

```bash
tiktools --json rpc collaboration.requests.list '{"limit":50}'
tiktools --json rpc collaboration.requests.approve '{"id":"REQUEST_ID"}'
```

The default invitation grants `live.read`. A label is informational; compare
the requested grant and the expected device before approval. The join page
only confirms approval in this milestone. It does not load the full controller.
Browser credentials stay in a Secure, HttpOnly, SameSite=Strict host cookie.

Owner invitation, request, session and principal lists accept `{after?,limit?}`
and return their named array plus `nextAfter`. Follow the returned cursor until
null; grant size can reduce the number of records in a page. The default count
is 50, maximum 100, with a 512 KiB item budget. Restart without a cursor to see
new records that precede a previously consumed ID. Audit uses its own numeric
`after`/`nextAfter` cursor.

## Native and browser wire contracts

All HTTP responses use `Cache-Control: no-store`. No endpoint accepts query
strings. JSON pairing and refresh bodies are capped at 4 KiB. Clients must not
log request bodies, cookies, Authorization headers or invitation fragments.

| Endpoint | Implemented contract |
| --- | --- |
| `GET /remote/v1/info` | Public protocol range, host ID, generic name and TLS leaf fingerprint hint; no LIVE data |
| `POST /remote/v1/pair/redeem` | Invitation plus client-generated exchange hash; returns request ID and expiry |
| `POST /remote/v1/pair/status` | Exchange proof; pending is HTTP 202 with `pairing_pending`; approved credentials/cookie collected once |
| `GET /remote/v1/session` | Authenticated caller identity and browser CSRF bootstrap |
| `POST /remote/v1/session/refresh` | Native refresh rotation; previous access stops working; replay revokes that session |
| `POST /remote/v1/session/logout` | Revokes only this session |
| `POST /remote/v1/ws-ticket` | Browser session + exact Origin + CSRF; single-use 30-second HttpOnly ticket cookie |
| `GET /remote/v1/ws` | Native Authorization header or browser session/ticket cookies; authenticated upgrade |

Native redemption, polling and refresh send `X-TikTools-Client: native` with
neither Cookie nor Origin. Native protected requests/upgrades use
`Authorization: Bearer ACCESS_TOKEN`. The native marker is not authorization.
Browser pairing requires an exact allowed Origin. Browser protected writes
require that Origin plus `X-CSRF-Token` from the authenticated bootstrap.
Browser and native credential collection cannot be interchanged. Mixed bearer
and cookie credentials are rejected. Refresh credentials never enter browser
JavaScript or WebSocket/query parameters.

The first WebSocket request, within ten seconds, must be:

```json
{"jsonrpc":"2.0","id":1,"method":"collaboration.hello","params":{"protocolVersion":1,"clientVersion":"your-client-version","clientKind":"agent"}}
```

The response contains host/instance/connection/principal/session IDs, current
grants, effective connection limits and feature flags. Then use the ordinary
registered method names. Remote discovery contains only permitted methods;
unknown and forbidden methods produce the same `forbidden` result. Session
revocation/expiry is checked on requests and before output; sockets are also
checked every two seconds. Access expiry requires native refresh and a new
socket. The gateway does not replay requests or extend access expiry merely
because a socket stays open.

Default limits are 8 connections total, 4 per principal, 1 MiB frames, 256 KiB
params, 32 pending calls per socket, and 30 requests/second per session with
burst 60. The owner can lower them through the `limits` configuration object;
`rpc.schema` describes the exact fields. Pairing admission has additional
persistent limits. Ping/pong checks bound dead connections. Malformed frames,
slow sends and oversized output close the offending socket. Reconnecting
clients must negotiate again; automatic mutation replay is not available.

## Storage and scope

The host lazily owns `<data>/collaboration/collaboration.db`. An exclusive
process lock protects the service before migrations or pairing cleanup. A
second standalone process must use the running host's local API. Unix directory
and database permissions are private. Windows ACL and real Windows/macOS
acceptance are still pending.

This direct-host gateway requires a reachable configured endpoint. It provides
no automatic NAT traversal, Cloud discovery or hosted relay. The
[architecture guide](LIVE-COLLABORATION.md) records the inspected RemakeBackend
boundary; an Internet relay would be a separate backend implementation.
No RemakeBackend runtime change, database migration or deployment was required for this
milestone. Local loopback TLS/network tests do not replace physical two-PC
LIVE acceptance.
