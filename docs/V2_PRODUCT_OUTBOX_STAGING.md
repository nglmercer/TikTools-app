# V2 product outbox staging

This native-only bridge is not selected by the product and does not apply remote state. An experimental native dispatcher can upload through an explicitly provided trusted source adapter; it is not an end-to-end product synchronization feature.

A candidate originates from an account-partitioned SQLite outbox committed with the local source revision. The native adapter holds the plugin invocation gate, verifies the exact source revision and this host lifetime's confirmed runtime observation, and refuses selections whose installed runtime no longer agrees. Package tombstones remain blocked pending filesystem and package-journal reconciliation. The sync coordinator independently checks the current or remembered account, exact consent generation and account scope, and a V2 protocol lease before writing the workspace. Remote application paths must never feed this local outbox.

The private enable/disable adapter can now reserve a v2 local edit using the
same SQLite transaction and runtime confirmation as its v1 counterpart. A
candidate requires scoped v2 consent, a completed v2 protocol and the matching
product-outbox marker, all checked while the cooperative mutation and identity
leases are held. An opted-out or signed-out edit remains local-only and must
be reconciled explicitly. This adapter is not selected by product operations;
the remaining writer, package, runtime and recovery boundaries described below
still block activation. The account mutex is released during marker and SQLite
I/O, so account reads can continue; the identity permit remains held and a
blocked native operation can still delay logout or account changes.

The workspace saves a bounded private source receipt in the same atomic state write as the staged decision. It records the source revision, decision digest, exact edit and resulting workspace generation. Repeating that same decision after an interrupted SQLite acknowledgement returns the existing generation without restaging, including after remote delivery or snapshot rebuild. An older revision or a different decision for the same revision fails closed. Existing workspace files without receipts remain readable; older software that does not recognize the added field must fail closed rather than rewrite it. Receipts are local metadata and are not Cloud records.

After the workspace save, the adapter deletes only the exact still-current confirmed SQLite candidate. A failed save, full workspace, busy operation lock, changed identity or consent, stale runtime, or failed SQLite acknowledgement returns an error and must leave or recover the outbox; retry is explicit and idempotent. The operation lock also excludes staging while a workspace network operation is in flight. Cross-process writers that ignore the protocol, source and workspace fences are outside this guarantee.

The experimental dispatcher accepts only queued plugin selections with an exact private receipt. It reconstructs the bound origin, account, consent generation, source revision and edit, checks the digest, and reserves both the native runtime boundary and SQLite source generation while persisting the prepared request. The reservation is released before HTTP; a later local change must remain independently staged. The adapter rejects tombstones, outstanding outbox or package/apply/rebase journals, a mismatched runtime and stale source generations. Raw workspace sync remains a general-purpose engine and does not offer this product authorization.

After acquiring the cooperative emission lease, the dispatcher rechecks that
same receipt against the native source and runtime before starting HTTP. If
the source changed after preparation, it sends nothing and retains the
prepared request as unconfirmed. This is a second observation, not a source
lock held through HTTP: changes can still occur after the check, and complete
writer/runtime participation remains required before product activation.
The native upload adapter also rejects new source or emission admissions once
host shutdown has begun, including a final check inside its SQLite source
transaction. A shutdown that starts after that check is not yet an atomic
fence for an already admitted or in-flight request; shutdown ordering and
server-side recovery remain necessary before activation.

The dispatcher runs local disk and source checks on a blocking worker rather than a Tokio executor thread. Its caller's cancellation is propagated to that worker, and its complete operation, including waiting for capacity, has a 60-second deadline. The caller receives a timeout even if native I/O remains blocked; checks after a blocked source returns reject cancelled or expired work before a new request. Cancellation or timeout before response publication leaves the durable result `Unconfirmed` for review. An atomic rename already admitted before cancellation can finish afterward, so the caller must inspect the retained state instead of assuming the timeout kept the result unconfirmed. While awaiting HTTP, cancellation or the deadline releases the workspace and protocol leases. Native filesystem or SQLite calls cannot be forcibly interrupted: if one never returns, its worker and any leases it holds remain alive until the operation actually returns. A process-wide limit of four workers retains each permit until the worker really exits, bounding accumulation; saturation fails with a timeout. Product integration must surface this state and account for the remaining in-flight work.

Before enabling product integration, all local plugin mutations and package operations must participate in the source admission/outbox protocol; restart reconciliation and tombstones need explicit review. The shared protocol lease spans HTTP and can make opt-out return busy. The exact-session emission permit described below serializes account transitions for one CloudAuth manager, but not independently constructed managers or other processes. The runtime observation is momentary and does not fence independent processes, direct filesystem writers or every autonomous plugin start. A staged value alone is not upload authority. Changes made while opted out require a separate reviewed reconciliation; they must not silently inherit a prior account's consent.

The generic workspace coordinator holds a shared protocol lease for the whole
operation, including its HTTP request and state publication. It refuses
generic bootstrap, stage, sync, review, resolution and rebuild for an account
with a product outbox marker or a marked (version 3) protocol record, including
an interrupted cutover. Corrupt marker files fail closed. Product staging and
dispatch require the product outbox marker in addition to a v2 lease. This
cooperative restriction covers already-migrated partitions without rewriting
them and prevents a concurrent cooperative cutover from overtaking a generic
operation. It does not fence older binaries or callers that directly mutate
the lower-level workspace store, and the separate product marker can be
removed by noncooperating filesystem access. The version-3 workspace mode
described below additionally protects a successfully migrated workspace;
safe handling of older processes remains necessary before product activation.

Installing a new product outbox marker now refuses any existing generic v2
pending or staged-next edit while holding the exclusive protocol lease and
both workspace locks. This avoids trapping a cancelled write's durable
`Unconfirmed` state behind the product-only path. It is not proof of remote
quiescence: a previous generic review may already have cleared an uncertain
request while its server handler remained in flight. Both protocol and product
markers can also be deleted by noncooperating software; before migration,
an older workspace with no product receipts is indistinguishable from generic
state when both markers have been lost. A server-backed write barrier remains
necessary for product recovery.

For product dispatch, pull state, push acknowledgements and conflict results are prepared and fsynced under the workspace CAS lock before acquiring an exact-session publication permit. With the cooperative v2 protocol lease still held to exclude consent changes, that permit covers only the final atomic rename; logout that wins before it prevents publication. A push whose response cannot be recorded under the captured session remains `Unconfirmed`, even if the server processed it, and requires explicit review. The account mutex is not held during preparation or rename. A rename syscall that does not return can still delay logout while the identity permit is held; directory fsync follows after the permit is released. These guarantees do not fence noncooperative filesystem writers.

A local HTTP integration test holds successful and conflicting responses while the real CloudAuth manager requests logout or session replacement. The transition waits for the admitted HTTP future, and the persisted request remains unconfirmed if the transition began before publication; control cases with the original session record the acknowledgement or conflict. Cancellation releases the local admission but cannot cancel a server commit already in progress. This does not establish the full product workflow or physical two-device acceptance.

An internal workspace recovery primitive now supports an explicit keep-local
rebase after an uncertain write or conflict. The comparison is obtained from
a fresh authenticated read and publication is fenced by the captured session
and consent; the later choice must still match another live read, the original
product receipt and the current native source under its short mutation fence.
When an uncertain write has a newer staged edit, the receipt and review must
identify that newer decision; the original request is not resent during review.
The resulting edit is queued, but the choice itself neither retries HTTP nor
changes the local runtime. Accepting Cloud data and recovering local effects
still require a separate durable application journal and are not enabled by
this primitive. It is private and is not yet wired to a product review UI.
An eventual adapter must hold the appropriate v2 protocol lease, use the
session/consent publication guard, run blocking filesystem and source checks
on a bounded worker, propagate cancellation and give the review a limited
lifetime. Direct calls to these primitives do not supply those guarantees.

An experimental, still private offline fallback atomically records an
untracked-local marker together with the SQLite plugin state. A cooperating
v2 product emitter takes a shared cross-process file lease, checks that marker,
and retains the lease while the local HTTP future is pending. The fallback takes the
exclusive lease without waiting; if a request is held it reports busy and
makes no change. Cancellation or the 60-second deadline drops an asynchronous
HTTP request and its lease, while the prepared request remains unconfirmed.
As described above, native calls that never return cannot be forcibly stopped.
The marker survives restart and subsequent local edits; there is no automatic
clear or product review flow yet. The public plugin writers still follow their
existing paths, and legacy v1 sync and other potential emitters do not yet
participate in this fence. It therefore must not be treated as a complete
safety guarantee or used to enable the fallback in product. Before activation,
all emitters and all local writers must share the same cross-process admission,
including already in-flight requests, and a durable reviewed reconciliation
must safely clear the marker. Older or otherwise noncooperating processes are
outside the advisory lease guarantee.

The private product selector also records this marker for edits while opted
out or without a matching v2 identity, using the exclusive lease across the
SQLite transaction. Its legacy v1 branch still relies on v1 staging and must
be reconciled before v2 cutover. Neither selector nor fallback is enabled in
the product, and other existing writers do not yet record the marker.
The experimental selected and explicit-v2 mutation paths now retain their
exclusive product emission lease through runtime confirmation, and the offline
fallback retains it through the runtime effect. A cooperating in-flight v2
emission makes these mutations fail busy before changing SQLite or the runtime.
Public explicit plugin start and stop also hold the same exclusive lease while
changing the runtime; a cooperating emission makes them return busy without
starting or stopping the plugin. The fence is independent of Cloud account and
consent state, but an unavailable local fence also makes these calls fail.
Public enable/disable and installed-state changes also take that lease around
their SQLite and runtime mutations. With Cloud Auth and persistence enabled,
they now atomically record a durable untracked-local marker with the source
change; v1 staging still runs as before. The marker blocks the experimental
v2 emitter until a future explicit reconciliation flow clears it. Direct
package install and uninstall, and the filesystem commit and rollback phases
of package jobs, take the lease around
source invalidation and package activation. These operations still use legacy
staging and do not atomically create the product outbox or untracked marker, so
their lease alone does not make them safe for v2 product activation. Automatic
worker changes, including subscriber startup after a package job, legacy v1
emitters, and noncooperating processes are not yet fully fenced; product
activation still requires those paths to participate and be verified together.

A private SQLite primitive can inspect one marker and atomically transfer a
still-matching installed source revision into an account outbox candidate
without runtime confirmation. A changed source, competing account outbox or v1
receipt, unresolved package/apply/rebase journal, tombstone, or failed commit
leaves the marker in place. The transaction alone does not authorize upload:
the future caller must hold the emission, exact identity/consent and runtime
fences, verify the installed artifact and version, and establish a server
barrier for older in-flight writes before transferring any marker. There is no
product or UI call to this primitive, and no automatic reconciliation.

Dropping a local HTTP future does not cancel work already received by the
server: the server can commit that request after the lease is released, after
a local edit, or even after a subsequent authenticated GET. Such a GET is
not a quiescence barrier. The private product keep-local review now requires
the original pending request's persisted epoch, a fresh authenticated check
that the server requires epochs, and a successful server barrier that advances
beyond that original epoch before it reads the remote record. Legacy uncertain
requests without an epoch remain pending; no epoch is assigned retroactively.
This relies on every backend instance enforcing the epoch policy and does not
cover legacy or noncooperating servers. It does not yet enable product review,
clear the local marker, or provide durable local application recovery.
The account-wide barrier POST also holds an owned permit for the captured
CloudAuth session through the HTTP future; a transition that wins the final
session check prevents dispatch. Cancellation can still leave the server
processing a barrier, so a later attempt must obtain a new successful response.
An isolated coordinator HTTP test pauses a fenced request before the server
processes it, cancels the client, stages a newer local decision, and performs
the barrier-backed review. The simulated server rejects the delayed request
with 412 after the barrier; the newer decision remains available for explicit
keep-local rebase. This fixture models the epoch policy and is not a substitute
for a coordinator-to-PostgreSQL race test or a physical two-device test.
The prepared request is persisted as unconfirmed before the emission lease and
marker check. Therefore a busy lease or an existing marker can require review
even when no request was sent. This is conservative and may reduce availability.

The native coordinator can now capture an exact, durable cached v2 plugin
selection and revalidate it under the same account session, scoped consent,
protocol, product-outbox marker and workspace state lock before a short local admission callback. It
rejects missing or rebuilding caches, pending local edits and any changed
workspace generation. This is not a fresh server read, and the callback must
itself fence the source, runtime, package, outboxes, untracked marker and
application journal. The primitive does not apply or activate a plugin and is
not connected to the product. The opaque review exposes no record outside its
revalidated callback; a future UI must also fence any later asynchronous
response against account or session changes. Account permits and workspace locks can be held
during native I/O, so a product adapter needs bounded worker admission. The
server barrier described above and durable application recovery remain required
before enabling remote application.

Marked product workspaces now persist their mode within a version-3 workspace
envelope. Generic coordinator mutations reject this mode even when both
separate protocol markers are missing, and the store refuses to downgrade it
to the version-2 format used by older readers. An explicit local upgrade of an
existing marked workspace takes the exclusive protocol and workspace operation
locks, verifies both original product markers and preserves the state through a
generation-checked save. This increments the generation and invalidates prior
workspace reviews. The upgrade changes no runtime state and sends nothing.
Unmigrated version-2 files with product receipts remain readable for upgrade,
but current generic operations and unguarded saves reject those receipts.

Protection against missing markers starts only after a successful workspace
upgrade. An older version-2 partition with no product receipts that has
already lost both markers is indistinguishable from a generic partition and
must not be inferred to be product-owned. This guard coordinates current
participating clients; direct storage writers and older processes that bypass
the protocol are outside its guarantee. Server barriers, source admission and
durable local application are still required before enabling product v2.

The private dispatcher now obtains a fresh authenticated write epoch and
requires the server to report epoch enforcement before preparing an upload.
The account-bound epoch is persisted with the exact `Unconfirmed` original
request before PUT or DELETE; the same epoch is sent in the request header.
A newer local decision remains separate, and an unknown response or HTTP 412
or 428 leaves the original unconfirmed without automatic replay. Product
tombstones without a known base are retained rather than cleared from an
absence GET, which cannot establish quiescence. These paths remain private.

The mode reported by a backend does not protect against older backend
instances that do not enforce it. The dispatcher checks consent before
acquiring an owned permit for the exact captured session, then checks the
session, cancellation and deadline without filesystem I/O. It holds that
permit through the local PUT/DELETE future and releases it before publishing
the result. A transition that begins after admission waits; one already in
progress prevents admission or the final check. This applies only to the
same CloudAuth manager and its clones, and the account mutex is not held.
Cancellation or timeout drops the future and permit when its worker can poll;
an unyielding poll or blocked native operation can delay release past the
deadline, so logout is not guaranteed to finish within 60 seconds. Dropping
the permit does not prevent an already received server request from committing.

The source/runtime admission is still released after preparing the request
and can change before or during HTTP. Neither the durable epoch nor server
policy closes that window. Product activation still requires homogeneous
backend rollout, source fencing, fresh authenticated barrier recovery, and
durable local application.
