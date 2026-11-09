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

The dispatcher runs local disk and source checks on a blocking worker rather than a Tokio executor thread. Its caller's cancellation is propagated to that worker, and its complete operation, including waiting for capacity, has a 60-second deadline. The caller receives a timeout even if native I/O remains blocked; checks after a blocked source returns reject cancelled or expired work before a new request. Cancellation or timeout before response publication leaves the durable result `Unconfirmed` for review. An atomic rename already admitted before cancellation can finish afterward, so the caller must inspect the retained state instead of assuming the timeout kept the result unconfirmed. While awaiting HTTP, cancellation or the deadline releases the workspace and protocol leases. Native filesystem or SQLite calls cannot be forcibly interrupted: if one never returns, its worker and any leases it holds remain alive until the operation actually returns. A process-wide limit of four workers retains each permit until the worker really exits, bounding accumulation; saturation fails with a timeout. Product integration must surface this state and account for the remaining in-flight work.

Before enabling product integration, all local plugin mutations and package operations must participate in the source admission/outbox protocol; restart reconciliation and tombstones need explicit review; and account, consent, protocol and cancellation must be fenced through actual request emission. The current shared protocol lease spans HTTP and can make opt-out return busy; a session can also change between the last eligibility observation and request emission. The runtime observation is momentary and does not fence independent processes, direct filesystem writers or every autonomous plugin start. A staged value alone is not upload authority. Changes made while opted out require a separate reviewed reconciliation; they must not silently inherit a prior account's consent.

For product dispatch, pull state, push acknowledgements and conflict results are prepared and fsynced under the workspace CAS lock before acquiring an exact-session permit. With the cooperative v2 protocol lease still held to exclude consent changes, the permit covers only the final atomic rename; logout that wins before that permit prevents publication. A push whose response cannot be recorded under the captured session remains `Unconfirmed`, even if the server processed it, and requires explicit review. The account mutex is not held during preparation or rename. A rename syscall that does not return can still delay logout while the identity permit is held; directory fsync follows after the permit is released. These guarantees do not fence noncooperative filesystem writers and do not close the request-emission window described above.

A local HTTP integration test holds both successful and conflicting responses while the real CloudAuth manager logs out or replaces its session. In all four transitions the persisted request remains unconfirmed; control cases with the original session record the acknowledgement or conflict. This does not establish the full product workflow or physical two-device acceptance.

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

Dropping a local HTTP future does not cancel work already received by the
server: the server can commit that request after the lease is released, after
a local edit, or even after a subsequent authenticated GET. Such a GET is
not a quiescence barrier. Before enabling review, clearing the marker or
publishing a rebase, recovery needs a server-backed barrier or a verified
protocol that rules out any later commit of the older request; the current
private keep-local primitive does not supply that guarantee by itself.
The prepared request is persisted as unconfirmed before the emission lease and
marker check. Therefore a busy lease or an existing marker can require review
even when no request was sent. This is conservative and may reduce availability.
