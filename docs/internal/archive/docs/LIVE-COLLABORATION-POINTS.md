# Collaborative points implementation

Date: 2026-11-07.
Status: ordered writers, transactional viewer revisions/outbox, scoped committed
reads, durable remote additive adjustments and points snapshot/replay are
implemented. The handshake advertises `workflows`, `points` and `commands`.
The native points intent coordinator/journal is implemented with a version-2
migration preserving workflow history. The native bounded points view, viewer
lookup and confirmed adjustment form are connected. Recover exact
outcomes with `collaboration.commands.get` even when stream retention expires.

This extends the [product plan](LIVE-COLLABORATION.md),
[protocol specification](LIVE-COLLABORATION-SPEC.md) and
[native command journal](LIVE-COLLABORATION-CLIENT-COMMANDS.md).
The [validation record](LIVE-COLLABORATION-VALIDATION.md) distinguishes executed
checks from the remaining two-PC/LIVE acceptance.

## Available API and agent use

Approved sessions can call `points.view`, `points.viewer.get` and `points.leaderboard` with
`points.read`, and `points.adjust` with both `points.read` and `points.adjust`.
All enforce exact viewer restrictions and deny creator-restricted global
points access. Remote reads return committed data and matching
`meta.resourceVersions`; legacy local-owner reads keep their optimistic cache
behavior and original envelope. Remote reset and configuration methods remain
denied, including full configuration reads. The read-scoped `points.info`
returns only `{currencyName}` for confirmation; creator restrictions still apply.

An adjustment supplies `commandId` and `hostInstanceId` together and rejects a
CAS precondition. Discovery reports `durableCommands: true` and
`concurrency: "additive"`. Get the current instance from `collaboration.hello`.

```json
{"jsonrpc":"2.0","id":"adjust","method":"points.adjust","params":{"uniqueId":"alice","delta":5},"meta":{"commandId":"award-alice-001","hostInstanceId":"HOST_INSTANCE"}}
```

An agent with `commands.read` plus the current points-read/viewer grant can
query its original result. Another principal cannot query it. Changing the
original request under its ID rejects; a repeated identical command returns
its historical result without recalculating the bonus or changing cache state.
Keep that ID and payload before dispatch. A timeout or permission failure after
commit is an unknown outcome; it is not evidence that the adjustment failed.
The SDK/CLI transport never automatically retries it.

```bash
tiktools --remote-profile live-desk api call points.adjust '{"uniqueId":"alice","delta":5}' --command-id award-alice-001 --host-instance-id HOST_INSTANCE --include-meta --format json
tiktools --remote-profile live-desk api call collaboration.commands.get '{"commandId":"award-alice-001"}' --format json
tiktools --remote-profile live-desk api call points.viewer.get '{"uniqueId":"alice"}' --include-meta --format json
```

Watch permitted points invalidations with the native CLI:

```bash
tiktools --remote-profile live-desk api events --topics points.changed,command.changed --timeout 60 --max-events 100
```

Topic selection respects the host's advertised domains and the current grant;
a points-only profile does not need workflow access. Events contain revisions
and receipts; query committed absolute state/outcomes before changing a view.

Use a new ID for a new deliberate adjustment. The CLI validates the discovered
concurrency strategy: workflow CAS commands still need `--expected-revision`;
points adjustments must omit it. SDK agents use `call_value_with_meta` or
`call_with_meta` with `RequestMetadata.expected_revision: None`. The legacy
`points_adjust` convenience wrapper supplies no durable intent metadata and is
not sufficient for a remote adjustment. The desktop's native journal
also supports typed points intents through the private native coordinator.
The native live projection and adjustment form use this coordinator; JavaScript
never supplies a command ID or a replacement balance.

## Checked unit confirmation

`points.info {}` returns only the current unit label under `points.read`.
A durable `points.adjust` may additionally supply `params.expectedCurrency`.
Native points commands always send it. It must be a trimmed, nonempty unit
label of at most 64 Unicode characters without controls. A fresh command
checks it against the frozen current configuration under the points mutation
gate, after actual SQLite admission and before reserving an ID or changing state.
Mismatch returns pre-acceptance `currency_changed`. Local legacy calls without
command metadata reject that optional precondition rather than ignore it.

The label is included in the payload hash only when supplied. Historical
unchecked IDs retain their original hash, while a checked command's unit cannot
be changed under the same ID. A replay/query still returns the original unit
and result after the owner renames it. This is a name fence on the host's points
unit; the application does not perform a currency exchange.

The requested amount remains distinct from the effective rounded subscriber
bonus and any zero-floor penalty. Native validation binds the exact ID, viewer,
unit, receipt, host instance and positive revision, and bounds effective results
against the host's earning rules. `collaboration.hello.features.pointsAdjust`
advertises this completed native-compatible adjustment contract.

## Existing behavior to preserve

PC A owns the points service, hot cache and `tiktok-points.db`. The controller
never awards TikTok events or maintains a second authoritative balance.
Automatic awards calculate optimistic cache state and enqueue absolute viewer
snapshots into one bounded FIFO writer. Local manual `points.adjust` creates a
missing viewer and uses the existing positive subscriber bonus. The automation
adjustment path changes an existing viewer without that bonus. Negative totals
clamp to zero. Preserve these distinct existing operations explicitly.

Points are currently global to the host, keyed by canonical viewer ID. They
are not partitioned by creator or LIVE room. A creator-restricted `live`
grant cannot safely expose this global table; deny those remote points calls
until a separately scoped projection exists. Viewer restrictions use resource
kind `viewer` and an exact canonical ID, never a prefix or wildcard match.

## Implemented consistency boundary

One service mutation mutex spans cold lookup, calculation, queue admission,
sequence allocation, hot-cache update and enqueue. Configuration changes,
cache eviction/promotion, reset and shutdown use the same mutex. The writer
thread never acquires it and can drain the bounded queue while a producer
waits. Lock order is mutation, then config/viewers, then the queue.

The former gap between releasing the viewer lock and enqueueing a snapshot
allowed another award, adjustment, reset or shutdown to overtake it. A
deterministic test pauses exactly in that gap. Shutdown now closes admission
after the active producer enqueues, then joins the draining writer.

A cold reload waits for that viewer's pending sequence and any pending full
reset. If the bounded wait expires, viewer lookup/award/adjust returns no result
and does not install stale SQLite state. A later successful batch never moves
the contiguous committed watermark past an earlier permanently failed write.
Resolved work and actually committed work remain separate diagnostics; each
successful per-operation fallback is counted exactly once.

New viewer IDs are at most 128 UTF-8 bytes, nonempty, trimmed, free of control
characters and `*`, and without a leading `@` after owner alias normalization.
An invalid single-viewer reset is a no-op, never a full reset. Nonfinite or
overflowing point calculations reject before changing a balance; levels
saturate instead of overflowing. Legacy malformed database identities remain
private local data and are excluded from generated viewer invalidations.
Owner reset notifications are emitted only after queue admission succeeds.

The points database now instantiates the same coordination table definitions
as the automation database. Existing viewers receive revision 1 once, without
rewriting their balances, history or emitting fictitious award events. New
viewers start at revision 1 on their first committed write. Every admitted
viewer write advances the revision, including zero-delta metadata/counter
updates. Reset keeps the viewer and advances its revision; it is not deletion.

Both batch transactions and their per-operation fallback atomically commit:

1. The viewer balance, level, counters and existing history semantics.
2. The `viewer` / `host` resource revision.
3. Its metadata-only invalidation reservation.

Revision exhaustion or invalidation failure rolls back that entire transaction.
A full reset advances every addressable durable viewer, including rows outside
the hot cache, streaming bounded identities rather than loading all viewers.

Delivery namespaces source cursors by database. Each poll admits up to 50
workflow resources, 50 workflow command outcomes, 256 viewer resources and 50
points command outcomes. Both owning ledgers are active. Each database
acknowledges its own tokens. Failure after acknowledging one source replays
only the other source with its stable IDs.

Automatic awards/automation adjustments produce projection metadata with actor
`host-runtime`; they do not create an audit row per LIVE event or initialize the
private identity store. Manual adjustments/resets and all durable commands
require audit. All audit chunks (at most 100 records each) commit before any
publication, and source acknowledgement follows publication. Retry never enters
the business event bus, awards points again, runs automation or plays media.

## Implementation sequence

| Step | Deliverable | Required evidence |
| --- | --- | --- |
| 1, implemented | Transactional reads and additive command coordinator | Deterministic producer/writer barriers; atomic rollback; process death before/after commit; later awards and restart preserve outcomes |
| 2, implemented | Scoped API and command recovery | Exact viewer grants, real SQLite-admission revocation, bonus limits, cross-database IDs/capacity, API and actual TLS lost-ack recovery |
| 3, implemented | Points snapshot/replay | Bounded immutable permission-filtered baseline; gap/grant/host recovery; projection delivery never reruns an award |
| 4, implemented | Native points journal, bounded view, lookup and confirmed adjustment form | Durable pre-send intent, host/viewer/amount confirmation, unknown-outcome query and no automatic resend |
| 5 | Operational acceptance | Concurrent moderator/agent and automatic awards, load/fault tests, real OS store, two physical PCs and a LIVE |

### Transactional reads

Read committed SQLite state and matching viewer revisions in the same read
transaction. Do not pair an optimistic hot-cache value with a durable revision.
Bound row text in SQL before materialization, filter hidden viewers before
loading profile fields, and bound page/frame/total bytes. Rank permitted rows
with the existing deterministic points/coins/chats/ID tie-break; do not rank an
unrestricted board and then filter it. An unrestricted owner keeps its existing
local API behavior unless explicitly requesting the committed projection.

### Additive command contract

The implemented wire call keeps params `{uniqueId, delta}` and supplies
`meta.commandId` and `meta.hostInstanceId`. It must reject `expectedRevision`:
this operation adds to the current balance rather than replacing an observed
balance. Discovery must advertise `additive` concurrency only when implemented.

Require `points.read`, `points.adjust` and permission for that viewer. Apply the
grant's optional `maxPointsDelta` to both the requested delta and the actual
rounded subscriber-adjusted delta. Keep the legacy meaning of the returned
`PointAward.delta`; it may exceed the balance removed when a penalty clamps
the total to zero. Validate finite arithmetic and level saturation before SQL.
Remote reset and points-configuration mutation remain denied in this slice.

The host's commit order defines the result. Penalties retain the legacy zero
floor: starting at zero, adding five then subtracting ten differs from
subtracting ten then adding five. Additive classification applies each request
to current state without a stale snapshot replacement; it does not remove
those existing order-sensitive business rules.

Under the host command coordination gate, establish the exclusive owner lease
and check the principal/command ID in every owning business ledger. A principal
must not reuse a workflow command ID for a points adjustment, including after
outcome expiry or host restart. Store ID/hash tombstones permanently; result
retention remains seven days. Capacity must apply across the ledgers, without
evicting an older ID to admit a new command.

For a new command ID, acquire the points mutation gate, freeze the current configuration, and wait
boundedly for all previously admitted FIFO work to commit. The admission budget
is 100 ms, including waiting for the mutation gate and drain; SQLite admission
uses the remaining budget. An unavailable
writer or any unresolved/permanent failure gap rejects before acceptance.
Do not write a remote balance directly while older absolute snapshots remain
queued. The writer does not need either host coordination or mutation gate.

Begin the actual SQLite write transaction, then recheck the live authorization,
session, viewer scope, delta limits and new-command host-instance fence.
Canonical payload hashing excludes transport/session/instance data and contains
no CAS precondition. An identical retained command returns its original outcome
without recalculating the bonus or modifying the cache from an old result.
Reusing the ID with a different semantic payload rejects.
Known retained IDs use a separate read transaction with current authorization;
they do not acquire a SQLite writer reservation or drain the award queue.
The same transaction keeps the semantic hash, original receipt and outcome
coherent while a concurrent writer holds uncommitted balance changes.

For a fresh command, commit balance/history, viewer revision, resource outbox,
exact outcome/receipt, command-ID/hash and required audit/command reservations
in one `tiktok-points.db` transaction with the command durability setting.
Only after commit may the service update the resident balance/level and release
the mutation gate. Preserve current identity/counter data, including viewers
not resident in the cache. Database failure leaves the cache untouched.
Publish a fresh business notification once; recovery publishes metadata only.

Before materializing an existing command target, SQL bounds its profile strings
to 16 KiB. Over-budget or malformed state rejects before acceptance; it is not
silently truncated and saved back. Snapshot reads also enforce raw/serialized
frame budgets. Scoped boards enumerate at most the grant's 256 IDs and perform
primary-key lookups. Unrestricted boards use the complete deterministic ranking
index, without sorting the entire viewer population. Full load/overhead
acceptance remains required; query-plan tests are not a substitute for it.

Route `collaboration.commands.get` to the correct business ledger while keeping
caller-owned queries, current points-read/viewer restrictions and expiry
behavior. Reauthorize receipts, results and conflict metadata at egress. A
permission failure after commit still means unknown outcome to the client.

### Recovery and controller

The implemented `points` subscription needs `points.read` and no workflow
capability. `commands` needs `commands.read` and at least one implemented domain
read capability. Its workflow/viewer receipts are independently filtered by the
current domain/resource grant and the caller's principal. Global viewer rows
remain unavailable to a creator-restricted session.

Request `collaboration.subscribe {domains:["points","commands"],feed:false}`,
then retrieve every `collaboration.snapshot` page before consuming updates.
A page's `points` array contains committed viewer profiles/balances; it is
omitted when empty. `resourceVersions` covers both workflow and viewer rows.
Match by `(kind, scope, id)`, never by ID alone or by zipping a mixed array.
The complete baseline is permission filtered before loading profile bodies;
it is not truncated at the leaderboard's 1,000-row public limit. Per-record,
page and total budgets reject with `too_large` rather than return partial state.
Scope a large host to a bounded set of viewers when its full baseline cannot fit.

`points.changed` carries metadata `{id,change}` and the committed viewer
version, without an award payload. Fetch absolute state through
`points.viewer.get`; apply newer versions monotonically. A committed but late
outbox invalidation can be older than the immutable baseline. Resets keep the
viewer and advance its version. Sequences are contiguous after filtering and
replay preserves their original IDs; grants, gaps or host changes require a
fresh baseline. Recovery never awards points or repeats a business action.
The SDK `RecoveryStream` assembles points/mixed pages and acknowledges only
after the caller applies an event. It resumes unacknowledged points updates
under the same session or replaces the baseline after permission changes.

### Bounded native display

`points.view {limit?: integer, uniqueId?: string}` requires `points.read` and
the current exact viewer grant. Creator-restricted sessions cannot read this
global table. Limit defaults to 20 and clamps to 0–1,000; zero requests only
the optional selected viewer. The result is
`{currencyName,viewers,selectedId,selected}`. Each display row contains only
`uniqueId,nickname,points,level,isSubscriber`; avatars, user IDs and counters
are not loaded. One SQLite read transaction supplies board, selected row and
matching `resourceVersions` for their unique union. A permitted missing lookup
returns `selected:null` plus that ID's version (zero if it never existed).
SQL filters permission before ranking and limits rows before loading text.
Serialized responses obey the negotiated byte budget; oversized data fails
explicitly rather than silently truncating.

`collaboration.subscribe` accepts optional `pointsLimit` (0–1,000) only with
the points domain. Omission preserves the complete permitted baseline above.
An explicit positive value requests a top-N display baseline; zero requests
no point rows and only reliable invalidations. The selected mode is echoed
in the subscribe response and every snapshot page. Invalidations still cover
all permitted viewers, including those outside the initial board. A bounded
baseline must never be treated as a complete viewer database.
The SDK exposes this through `RecoveryStream::start_projected` and checks the
mode and total row bound on every baseline and after reconnect.

Native controllers require hello feature `pointsProjection`, subscribe with
`pointsLimit:0`, finish the baseline, then fetch absolute `points.view` data.
They display up to 20 permitted leaders (reduced for small frames) or one
looked-up viewer. A reliable invalidation stages a dirty bit before ACK; the
native timer groups display reads at no more than four per second, slower
for negotiated low request budgets. A two-second idle refresh detects unit
label changes. Reconnection always reads fresh absolute state before restoring
the panel, including on cursor resume. No event payload is added to a balance.
The point revision is separate from the workflow display revision, so frequent
awards do not force workflow reloads or invalidate workflow confirmations.

The English/Spanish form searches by exact ID (normalizing a leading `@` in
the UI), displays committed balance or a missing-viewer notice, and confirms
host/origin, viewer, signed amount and unit before one native dispatch. Read-only
grants have no adjustment form. Changes to profile, host instance, session,
connection, grant, selection or unit require a new confirmation; balance-only
updates preserve an additive confirmation. A new connection is also fenced
in the native command coordinator. Success history shows requested and effective
amounts separately. Pending/unknown commands block both workflow and point
mutations and can only be resolved by original-ID query or explicit review.

The native journal now has a typed points adjustment containing viewer ID,
requested delta, unit label and the existing profile/host/principal/session binding. Native
code generates and syncs the command ID before dispatch. No JS-generated
command ID, owner IPC fallback, implicit host selection or blind resend is
allowed. Confirm host, viewer, amount and currency. Show committed state,
pending intent and recovered result distinctly. Resolve the exact original ID
after cancellation/disconnect/reload; manual unknown review never claims that
the adjustment did not happen.

The version-2 journal atomically migrates version 1 under the exclusive profile
lease, preserving every pending workflow ID/binding and recent record. Invalid
or unsupported data fails closed. Tests terminate a child before replacement,
after rename and after directory sync, and cover synced points intents/results.
A points success stores verified effective delta, total and level separately
from the requested amount; it never installs that historical balance as current
state. The existing history/unknown-review panel renders both amounts and units.
Physical two-PC, real operating-system credential storage and LIVE acceptance
remain pending; see the validation record for automated UI/native evidence.
