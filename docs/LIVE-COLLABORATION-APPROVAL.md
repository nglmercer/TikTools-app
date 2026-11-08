# Delegated execution approval and activation contract

Date: 2026-11-08.
Status: **partial implementation, locally tested**. The host now has
durable approval/activation/revocation, API/SDK contracts and a closed core
execution policy. The native owner approval coordinator and Owner UI are now
implemented and locally verified through the packaged browser/IPC/Core path.
Scoped HTTP/audio sinks, exact-topic emits, bounded scripts and metadata-only
external-effect recovery are implemented. Approved custom-topic rules accept
only already approved scoped input. Verification is recorded separately in the
validation document. Delegated plugin execution, HTTP response emission and
full collaboration acceptance remain pending.
The [automation specification](LIVE-COLLABORATION-AUTOMATION.md)
and [full collaboration contract](LIVE-COLLABORATION-SPEC.md) remain authoritative.

## Existing boundary

The host currently accepts author-isolated, explicitly disabled remote
proposals. Trusted provenance remains separate from editable definition JSON.
The owner-only `automation.manifest.get` read describes the complete saved
primary/dependency closure and current saved author subjects; its
`executionAuthorized` field is always false. Ordinary Owner edits, enablement,
manual testing and JSON authority-looking fields do not grant delegated
execution. Explicit Owner adoption is a different authority transfer.

Relevant implementation sources:

- [Manifest builder](../crates/tiktools-core/src/services/automation/manifest.rs).
- [Atomic saved review](../crates/tiktools-core/src/db/sqlite/behavior/review.rs).
- [Behavior version transitions](../crates/tiktools-core/src/db/sqlite/behavior/versions.rs).
- [Trusted provenance](../crates/tiktools-core/src/db/sqlite/behavior/provenance.rs).
- [Durable behavior commands](../crates/tiktools-core/src/db/sqlite/behavior/commands.rs).
- [Command replay/results](../crates/tiktools-core/src/db/sqlite/commands.rs).
- [Saved subject admission](../crates/tiktools-collaboration/src/authority/execution.rs).

## Invariants

1. Approval never changes a record from Delegated to Owner authority.
2. Approval never widens any subject's effective current grant.
3. All primary and dependency bindings are exact, including disabled,
   random-unselected, absent and retained deleted references.
4. The runtime cannot construct an execution permit from definition JSON, an
   RPC-supplied manifest, a digest alone or a historical command result.
5. Enablement changes definition revision and provenance generation. They are
   not excluded from the content digest merely to keep an approval valid.
6. Grant, definition, dependency, plugin, settings, policy or authority changes
   invalidate affected approval. Returning to an earlier body never restores it.
7. Retained command replay is historical and never reinstalls approval or state.
8. Unknown external effects are not automatically repeated after recovery.

## Review, approval and activation

Use the owner-only durable command
`automation.approval.activate`. Its operation is **approve the reviewed candidate
and activate the explicitly reviewed state plan**, in one transaction. This is
not an alias for `automation.adopt` or ordinary `automation.enable`.

The generic local-owner API and typed Rust SDK are implemented. Remote callers
are denied even with every capability. The native coordinator now supplies the
Owner UI with host-generated durable identities and original-result recovery.
Use `automation.approval.revoke` for a durable, revision-checked revocation.
Both methods require explicit acknowledgement and complete standard command
metadata. Receipts describe a historical result and never authorize execution.

Activation returns `kind`, `id`, `enabled:true`, `approved:true`, `policyVersion`,
the **final post-enable** `manifestDigest` and string `approvalGeneration`.
Version metadata contains the complete final primary-first closure. Revocation
returns `kind`, `id`, `revoked:true` and the new `approvalGeneration`, with the
advanced primary version. Original command replay returns the original receipt
without resolving current subjects or reinstalling an old runtime approval.

The request should contain only:

- Primary family/ID and exact reviewed primary revision.
- The complete ordered reviewed versions and captured manifest digest.
- Explicit desired enable states for the primary and selected dependencies.
- Explicit acknowledgement of delegated approval and the displayed effect plan.
- Standard durable command ID and reviewed current host-instance fence, supplied
  by the native owner coordinator rather than chosen by browser code.

The native UI follows the last bullet. The generic local-owner API also accepts
standard caller-managed durable metadata for explicit SDK/CLI integrations.

### Owner UI and native recovery

In Owner Automations, select a saved event or action and choose **Review
delegated execution**. This is separate from the existing authority-transfer
review. The dialog inspects saved source and every referenced action, including
disabled/random-unselected entries, trusted provenance, exact versions, effect
and render-field classifications, current author grants and configuration
revisions, and retained approval state. Unsaved editor text never replaces the
reviewed candidate.

The explicit plan enables the primary. Each delegated dependency is omitted
(saved state unchanged) unless the Owner selects its desired Boolean state.
Owner-owned dependencies cannot be toggled here. Editing the plan clears
consent. Unsupported policies, absent authors and missing dependencies block
activation; source and retained approval remain inspectable for revocation.
Both approval and adoption warn about source integers outside JavaScript's
safe display range. Such source remains inspectable, but the browser cannot
approve or adopt a value it may have rounded; retained approval revocation
remains available. Exact Rust/host inspection is independent of that browser
display restriction.

`automation.approval.review` returns one source/status snapshot, a current
manifest and author grants when resolvable, and a closed eligibility reason.
The complete result is bounded to 4 MiB. `approval.current` compares the full
stored manifest/policy binding, not merely a retained `active` label or digest.
Reverse-dependency rows are validated before reporting a current approval.
Both the public manifest digest and runtime permit now pin each subject's
configuration revision. An existing approval from an older manifest shape
requires a fresh review instead of being silently upgraded.

Local Owner IPC and browser responses have a separate finite 8 MiB transport
limit so a valid complete review above 1 MiB is not disconnected. Oversized
responses become a correlated `too_large` error, and oversized notifications
request UI resynchronization. The 1 MiB request limit, 256 KiB params limit and
separately negotiated remote gateway limits are unchanged.

The native methods are:

- `automation.approval.get`: current host identity, one pending operation and
  at most 16 recent metadata receipts.
- `automation.approval.submit`: a strict `operation:activate|revoke` union.
  Activation takes the reviewed versions/digest/explicit plan and effect
  acknowledgement; revocation takes the current primary revision and its own
  acknowledgement. Both take reviewed host identity. Browser callers cannot
  supply a business command ID, source, subjects, grants or a permit.
- `automation.approval.resolve`: query the original ledger identity only.
- `automation.approval.review_unknown`: explicitly archive uncertainty for
  the original pending command, after one final ledger lookup. A recovered
  receipt wins over the caller's assertion that the outcome is unknown.

These methods are LocalOwner-only, reject caller command metadata and are
denied to remote callers at ingress and egress. Owned native tasks outlive a
cancelled page request. The host persists the intent before dispatch in
`owner_automation_approvals`. Private proof contains only reviewed versions,
digest and enable-state metadata, never source/configuration bodies, grants or
credentials. Receipt verification checks exact method/hash/target/base/host and
the complete final revision plan. History/result persistence failure preserves
the pending identity; storage ambiguity is never classified as a safe retry.

Activation and revocation share a pending gate with Owner adoption and native
single-record definition changes. All three UI flows observe that gate and
invalidate consent when another journal changes. Recovery or archiving
uncertainty clears prior consent
and requires a fresh explicit saved-source review before another transition.
Reload restores the native journal and does not resend the operation. Browser
storage contains neither source nor pending command identities.

## Implemented core execution policy

Private policy `core-log-delay-points-http-audio-emit-script-v3` admits exact saved
`core.log`, `core.delay`, `core.points`, `core.points.subtract`, `core.fetch`,
`audio.play`, `core.audio.play`, `core.emit` and `core.code` definitions. Every referenced action,
including disabled and random-unselected actions, must satisfy the policy.
Missing dependencies, forbidden topics or fields, HTTP response emission and
plugin actions still reject approval. Opaque-script review applies only to the
exact saved `core.code` source and never adds a returned effect's permissions.
Existing approvals bound to an earlier
private policy require a fresh saved-source review and activation; updating
code never silently upgrades an approval.

Any rendered field requires `live.read`. Approved event matching and rendering
receive a closed public scalar projection: event identity/type/timestamp, user
ID/handle/nickname, creator handle/room and selected public chat/count/gift
leaves. Objects, arrays, private/enriched fields and strings above 4096 bytes
are removed. Whole-object fields and globals cannot be approved. The exact
allowlist lives in
[the private host policy](../crates/tiktools-core/src/services/automation_approval_policy.rs).
Delegated rendering checks byte limits before allocating expanded output:
8 KiB URL/header values, 256 KiB HTTP body, 4 KiB audio path, 64 KiB log,
64 KiB emitted JSON and 256-byte points viewer. These limits are part of the private approval binding;
repeated public template spans cannot create unbounded intermediate strings.
Delegated rendering always uses empty globals. Event-trigger approval also
requires `live.read`, even for a literal log action.

All retained subjects must keep their exact approved grant and host
configuration revisions. Returning grants/configuration or definitions to an
earlier value does not restore approval. Runtime admission reconstructs the
complete stored binding and holds current authorization plus exact runtime
definition fences through each synchronous sink. Multi-action records pin their
original approval across waits. Manual execution accepts the exact saved body;
edited or inline candidates with the same ID cannot borrow its approval.

Points require `points.read`, `points.adjust`, an allowed exact viewer and both
requested and effective rounded/clamped delta within every subject's limit.
The dedicated writer drains earlier LIVE writes under the shared mutation gate,
then holds authorization through the actual balance/version/outbox commit. It
preserves existing automation adjustment semantics: an existing viewer is
required and no subscriber award bonus is added. Dry runs also validate the
rendered viewer and requested delta. Delay rechecks authority after waiting.

Approved points adjustments also consume a private execution identity in this
same points transaction. The trusted native creator/room/provider message,
stable saved event/action slots and literal adjust/subtract operation determine
identity; editable JSON IDs, author sessions and host restarts do not. Exact
saved source and rendered viewer/requested delta are separate semantic
fingerprints. Reusing an identity with changed semantics fails closed. A new
trusted message or explicitly fresh manual execution remains new work.
Missing provider message/creator/room identity starts fresh work; editable JSON
never substitutes for missing trusted identity.

The shared command-coordination gate precedes the automation writer gate and
stays held through commit, so runtime and public commands cannot race the shared
cross-database capacity check. The balance, viewer revision, command receipt
and resource/command outboxes commit atomically through the existing
command-ledger primitives. A retained
invocation rechecks current approval, subjects, viewer scope and its original
effective delta, then returns no historical award or cache state and emits no
second points notification. Even a clamped-zero subtraction consumes identity
before later funds arrive. Result expiry retains its tombstone and never makes
an old message fresh again. The private namespace has a lifetime cap of 750,000
identities and also honors the shared one-million-ID business-ledger cap;
capacity rejects fresh work without evicting prior identities. This prevents
restart/replay-window redelivery of direct approved points adjustments without
a separate preclaim in the external-effects database. Owner legacy adjustments
and ordinary automatic LIVE award semantics retain their existing paths.
This guarantee starts with adjustments committed by the atomic writer. Earlier
versions did not save these identities, so their historical adjustments cannot
be reconstructed from a balance; upgrade/cutover acceptance remains separate.

Scoped completion/points/history notifications reach presentation subscribers
through private bus publication that never enters plugin observer queues.
Separate bounded observer lanes also prevent a scoped burst from manufacturing
an executable `event.gap` callback. Later Owner history publications retain the
scope of prior runs instead of laundering them into plugin callbacks. Remote
workflow/automation/points command outcomes use the same presentation boundary.
Ordinary Owner observer delivery remains available.

The durable database tables are `behavior_execution_approvals` and
`behavior_execution_approval_dependencies`; every behavior version transition
invalidates reverse references in its own transaction. Retained approval
generations and tombstones prevent same-ID recreation or delayed snapshots from
reviving invalid state. Startup validates bounded stored bindings and reverse
relations together with the trusted behavior snapshot.

Do not accept candidate source, provenance, subjects, grants, effect allowances,
plugin identities or prebuilt authority tokens as approval request data. The
writer derives them again from trusted saved host state. Every plan entry must
be unique, canonical and belong to the captured closure. Wrong-family,
extraneous, absent or deleted plan targets reject rather than becoming creates.
The primary desired state is explicitly enabled; omitted dependency states stay
unchanged. The UI must show which disabled dependencies will remain inactive.

There must be no implicit activation of every referenced action. Enabling an
Owner-owned action can affect other Owner events referencing it. Such enablement
belongs to the ordinary checked Owner control followed by a fresh manifest
review; it must not be hidden inside delegated approval. A delegated dependency
included in the explicit plan remains Delegated, and eligibility of any other
referencing event still requires that event's own exact approval/context.

The current preview alone is not sufficient for approval. A separate host policy
validator must distinguish:

- The approval-required marker, which this transition can satisfy.
- Classifiable effects with complete effective-grant/final-sink policies.
- Dynamic or opaque effects requiring exact-source Owner review plus enforceable
  current runtime restrictions.
- Unsupported effects or missing capabilities/resources/plugin bindings, which
  approval cannot override.

For example, reviewing a script does not add HTTP destinations or secret/global
access absent from current grants. Unimplemented global/secret-read policy stays
unsupported. Plugin names do not establish descriptor/artifact/permission or
settings-generation bindings. Unknown or unsupported paths remain ineligible
until their policies and sinks are completed.

### Avoiding the disabled-approval trap

An approval of the existing disabled manifest cannot simply survive a later
enable operation: [version advancement](../crates/tiktools-core/src/db/sqlite/behavior/versions.rs)
also advances trusted provenance generation. The safe transition is:

1. Capture the exact current disabled candidate and intended enable-state plan.
2. Rebuild and compare that pre-transition review under the actual writer and
   current-subject admission boundary.
3. Apply only the explicit Boolean state changes, without editing source or
   changing authority.
4. Advance changed dependency versions/generations. Advance the primary's
   version/generation once for the fresh approval transition, including when it
   was already enabled. Avoid advancing it twice when its enabled bit changes.
5. Rebuild the final **post-transition** manifest from that same transaction and
   the same current authorization/catalog bindings.
6. Persist approval for the exact final closure, then commit enable states,
   approval, command result and metadata outboxes together.

The owner reviews both the original candidate and the intended state delta.
There is no unreviewed source change between those two manifest states. A crash
must leave either the entire previous state or the entire committed approved
state. A separate later ordinary enable, edit, adoption, delete/recreate or
dependency change invalidates the existing approval.

## Minimal host-owned persistence

Keep approval in `tiktok-automation.db`; do not add a RemakeBackend collection,
permission shortcut or PostgreSQL migration. A minimal layout is:

| Table | Purpose |
| --- | --- |
| `behavior_execution_approvals` | One retained primary family/ID identity, monotonic approval generation, policy version, exact final manifest digest, bounded private binding JSON, Owner approver/time and revoked/invalidated state |
| `behavior_execution_approval_dependencies` | Primary-to-definition reverse index with ordered family/ID bindings, revisions, provenance generations, presence and content digests; includes missing/tombstoned references |

The private binding JSON also binds every subject principal/session/grant
revision, host identity, closed effect/render policies and complete trusted
plugin/catalog/artifact/permission/settings generations. It contains no bearer
tokens or settings values. Apply the same or stricter count, depth and byte
budgets as the manifest preview. Decode storage as untrusted data and validate
every relation before constructing a non-deserializable runtime permit.

Approval generation must not reset after deletion, recreation or revocation.
Never seed approvals during legacy migration, infer approval from an enabled
bit or delete the retained approval identity with its business record. Command
result retention is separate from approval identity retention.

Every behavior writer invalidates reverse-indexed approvals in the same
business transaction as its version transition. This includes legacy Owner
writes, bulk enablement, reference cleanup, adoption and recreation. Missing or
inconsistent approval/dependency storage fails closed; it is not repaired into
approved or Owner state. Runtime eligibility always revalidates the complete
binding, rather than trusting a stored `valid=true` bit.

Grant/revoke state lives in the collaboration store, and plugin state may live
outside the automation DB. Therefore final current-state validation remains
mandatory even when eager invalidation is also implemented. A fresh matching
body or restored grant does not revive a previous invalidated approval.

### Plugin and settings prerequisites

The current processor settings revision is an in-memory cache generation. It
is not a durable settings binding and resets on restart. Plugin approval needs
trusted artifact/descriptor identity, declared permissions, activation/catalog
identity and a separately persisted settings generation. Do not substitute a
guessable hash of secret settings values.

All relevant plugin/settings writers need a common ordering with the approval
reader, or another compare-and-commit fence that proves the captured binding
stayed current through approval commit. The existing write-file-then-bump-cache
sequence is insufficient for that proof. Until those writers are integrated,
the corresponding plugin path remains unsupported. After restart, plugin
approvals cannot become valid merely because a volatile generation starts again
at zero; every durable binding must be reverified or the approval invalidated.

## Locking and transaction checklist

The intended order follows existing command coordination and writer admission:

1. Host-wide command namespace/publication coordination gate.
2. Automation writer/runtime-index admission gate.
3. Required trusted catalog/settings lease using one documented order shared by
   every relevant writer; do not add an opposite-order plugin path.
4. Automation SQLite `BEGIN IMMEDIATE` reservation.
5. One batch saved-subject admission lock for the complete subject set.
6. Validation, mutation, approval persistence, command finish and **actual
   automation transaction commit while that batch lock remains held**.
7. Release collaboration/catalog locks, install the fresh runtime index under
   its admission gate, then publish/import metadata and audit work.

Never nest single-subject admission callbacks: they use the same collaboration
DB mutex. Never await or reenter collaboration from a batch callback. Check host
shutdown and current subject deadlines immediately before final admission.
Snapshots made before waiting for any writer are not a substitute for these
checks.

The existing `behavior_transaction` helper commits **after** its apply callback
returns. Wrapping only that callback's inner approval work in `with_current`
would release authorization before commit. Approval needs an owned transaction
committed inside the admission callback, or an equivalent helper retaining the
same lock through commit. A revoke/disable/grant writer must be ordered wholly
before or after the approval commit; it cannot commit in an authorization gap.

Synchronous admission ordering does not prove completion of an asynchronous
external effect. Release these locks before network/media/plugin awaits, retain
the trusted context, and reauthorize each later final sink under its own
specified admission boundary.

## Durable identity, replay and revocation

Reuse the existing shared command namespace, capacity, semantic hash, durable
result and metadata outbox machinery. The semantic identity includes method,
primary target/base, complete reviewed versions, captured preview digest,
explicit state plan and acknowledgement. Wire request IDs and reconnects do not
create new business identities.

For an authenticated local Owner:

1. Check retained command identity/hash before fresh review/approval work.
2. A retained same-ID/same-semantics success returns its historical result and
   versions. Do not resolve expired subjects first and deny access to that
   Owner's historical receipt because an author was later revoked.
3. A retained ID with different semantics rejects as reuse.
4. Only a fresh command checks the current host-instance fence, complete review,
   subjects, policies and eligibility before mutation.
5. Replay never enables records again, restores an old approval, reinstalls
   historical runtime state or replaces a later revocation/Owner edit.

The fresh result should remain a bounded receipt/identity summary. A success is
historical, not a reusable runtime permit. The native owner coordinator persists
its generated ID before dispatch, retains unknown outcomes and resolves only
the original ledger result. Browser storage must not contain source, credentials
or pending authority transitions.

An owner-only durable approval revocation operation retains the approval
identity, advances its generation and fences the primary runtime generation.
Repeated retained-command replay is idempotent. Revocation stops queued or
unstarted delegated effects; it does not claim to undo effects that already
crossed a final dispatch boundary. Definition/grant/plugin invalidation has the
same future-execution stopping requirement.

## Implemented external-effect grants and handoff

The Grant JSON retains its original five fields when no external scopes exist.
A nonempty optional sixth `effects` field contains closed `http`, `audio` and
`emitTopics` collections. New capabilities are `http.request`, `audio.play`,
`events.emit` and `scripts.execute`. A capability alone grants no destination
or file. Emit topics require `events.emit` and an exact normalized topic from
every retained subject; scripts require `scripts.execute` plus `live.read`.
Limits and validation live
in [the typed grant policy](../crates/tiktools-collaboration/src/effects.rs).

HTTP requires one complete rule from **each** retained execution subject;
fields from different rules cannot be combined. Rules bind canonical origin,
exact URL paths, uppercase methods, explicit query names, lowercase header
names, public render fields, top-level JSON body fields, an explicit text-body
allowance, request/response byte ceilings and private-network consent. Unknown
fields, wildcard paths, duplicate query names, host/framing/proxy/hop-by-hop
headers and ambiguous paths are denied. Authorization/Cookie headers require
explicit names and can use only the reviewed literal/public rendered source;
this does not grant access to host secrets or globals. HTTP origins use HTTPS;
explicitly permitted local/private HTTP targets require private-network consent.

`core.fetch.maxResponseBytes` defaults to 2 MiB. A tighter grant requires a
matching lower action budget, which is checked at review and at dispatch.
The host checks the complete rendered request, resolves and checks all DNS
addresses (including IPv4-mapped IPv6), and pins those addresses into a private
request client with proxies and redirects disabled. It rechecks the exact
approval, all current grants/configuration revisions and deadlines after DNS
and through the durable dispatch commit. Replies expose status/duration only;
response bodies, destination queries and transport errors are not placed in
scoped run output. Nonempty `emitResponseAs` remains unavailable: HTTP response
read/flow authorization needs a separate policy, even though ordinary scoped
emit execution is now supported.

Audio rules bind exact canonical file paths, output `host-default`, a finite
maximum volume from zero to one and explicit `allow`/`restart`/`drop` overlap
modes. The host resolves relative paths below its data directory, checks file
type/size and repeats resolution plus all subject checks at final admission.
A host without an audio sink rejects before acceptance. These are **pathname
permissions**, not sealed file-content or inode grants: local replacement of an
authorized path is outside this contract. Owner audio behavior remains on its
existing local path.

Approved emits use a separate publication path. Nested payload strings are
inventoried at review and rendered under byte/depth/node limits. Every sink
checks all current subjects' exact topic grants, commits its durable handoff,
and publishes a private child envelope. The payload is reprojected to the closed
public event fields before local presentation and scoped rule matching. It
never enters executable plugin/domain observers, enrichment processors or Owner
test-preview memory. Typed child depth and all ancestor approvals survive.

A delegated event with a non-`tiktok.*` trigger must store an exact lowercase
normalized topic and every subject must retain `events.emit` for that topic.
Its approval is valid only for an already approved incoming context. Owner or
plugin-origin input cannot start it. An Owner-authored downstream rule retains
the incoming subjects and a private fence over its complete saved closure;
every referenced event/action resource and final points delta or effect must
still fit every inherited grant. The Owner review UI labels this input condition.

Approved scripts run in a separate blocking worker using a fresh QuickJS
runtime, the public event projection and empty inputs/host globals. There are
no host I/O APIs. Console output is captured with returned logs rather than
traced through an Owner callback. The scoped evaluator bounds source to 128 KiB,
result to 256 KiB, JSON depth to 32, nodes to 8192 and total effect intents to
64. It permits at most 40 log entries, each at most 4096 bytes, within a 64 KiB
aggregate log budget. VM limits are 64 MiB memory, 512 KiB stack, five seconds,
2000 interrupts and 2000 pending jobs. The host admits at most four concurrent
delegated VM evaluations, with no queued backlog. Saturation rejects before VM
allocation or a Script receipt. The owned blocking worker retains its admission
permit through VM unwind, including when the awaiting caller is cancelled.
Current authorization/cancellation is
checked before, during and after evaluation, and each returned effect is
separately reauthorized at its final sink. These actual evaluator limits are
part of the private approval binding.

An opaque script can turn public input into apparent literals. Therefore every
HTTP request derived from a script must authorize **all 18 public input fields**
in one complete HTTP rule from each subject, regardless of which fields were
present or apparently used. A private script-flow marker is monotone through
child events and context intersections; emitting and triggering another rule
cannot erase it. Script capability never substitutes for HTTP, audio, topic,
viewer, amount or plugin permissions. The binding records this conservative
flow policy explicitly.

Only approved delegated external sinks use the private durable effect journal.
Trusted native ingress derives its root from the actual creator/room/method/
canonical event kind/upstream message ID, never editable JSON `event.id`.
Missing upstream identity and explicit manual execution start fresh host roots.
Queues/enrichment/intersections preserve the root. Saved-event ID, saved-action
ID and the occurrence among repeated references select stable child slots;
reordering unrelated actions never creates a new identity for old work.
Changing source or rendered request under an existing identity is a conflict,
not permission to dispatch again.

Before dispatching any returned script intent, the host reserves a Script
handoff bound to the complete bounded evaluated input/result; only digests and
receipt metadata are persisted. Every retained Script
state refuses **all** returned effects, including when reevaluation would change
their order or count. This prevents moving earlier work to a fresh child slot.
Per-category stable child slots identify emitted, HTTP and audio intents.
The binding records `retained-invocations-never-reemit-intents-v1`; no receipt
authorizes resuming a partially executed plan automatically.

A fresh effect commits `accepted`, then rechecks authority through a durable
`dispatched` transition before handing work to the sink. SQLite admission and
completion run on owned blocking tasks; after admission resumes, the executor
rechecks the current exact authority/target before the actual sink call. Completion records
`succeeded`, definite `failed`, or `unknown`. Cancellation, ambiguous network
failure and a restart with unfinished work produce `unknown`; none is retried.
A successful dispatch is the irrevocable handoff point: later revocation cannot
undo an already-started request/playback, but blocks subsequent derived work.
All retained states, including expired results, prevent another dispatch.
Seven-day result expiry keeps an identity tombstone; the finite total identity
capacity fails closed rather than pruning deduplication protection.

Owner-only reads `automation.effects.get` (`effectId`) and
`automation.effects.history` (`before`, `limit` from 1 to 100) expose only opaque
identity, kind, state, timestamps and expiry metadata. They are explicitly denied
to remote callers and accept no command metadata. There is no retry/replay RPC.
The closed receipt-kind set is `http`, `audio`, `plugin`, `emit` and `script`;
the plugin kind does not make delegated plugin invocation available. The host
migrates the earlier three-kind table constraint while retaining identities,
states and deduplication tombstones.
Recovery is initialized once at host startup; failed journal recovery blocks
receipt reads and new effect admission. No request bodies, URLs, media paths,
plugin settings, credentials, response bodies or raw errors enter this journal.
See [runtime handoff](../crates/tiktools-core/src/automation_runtime/external_effects.rs),
[HTTP/audio sinks](../crates/tiktools-core/src/automation_runtime/approved_effects.rs),
[scoped emit/script sinks](../crates/tiktools-core/src/automation_runtime/approved_flow.rs),
[bounded scoped evaluator](../crates/tiktools-core/src/services/script/scoped.rs)
and [durable storage](../crates/tiktools-core/src/db/sqlite/effects.rs).

Plugin package/settings binding and one-shot protocol isolation are groundwork:
artifact snapshots, descriptor/catalog/activation generations and durable
settings generations do not make native OS execution a sandbox. Plugin effects
remain unavailable for delegated approval until enforceable execution isolation
and final per-intent policies are connected. No broader runtime approval is
implied by adding those binding APIs.

## Runtime and external-effect requirements

An approved root creates a private execution context only after validating its
complete final manifest and current subjects. Owner rules triggered by a
delegated event inherit the intersection of incoming and saved-definition
restrictions. Neither event JSON fields nor nested emits can create Owner
authority. Every queue, script result, plugin intent, HTTP response emission and
enrichment path preserves that context. Remembered events and later replay must
not discard the private context and recreate Owner authority from a bare JSON
value. Points/ranking/competition-derived work carries the same restrictions.
Combining contexts requires the same owning host service/admission mutex, not
merely equal persisted host-ID strings from potentially copied stores.

Each final sink resolves dynamic targets after rendering and checks exact
resources, amounts, action arguments, render fields and current policy. Globals
and settings are filtered before reads as well as before effects. Native plugin
effects may occur during invocation; validating returned intents alone is not
sufficient. Unsupported effects must remain unavailable throughout execution.
Host shutdown is checked again after plugin/network awaits and before plugin
result observation, settings/global reads or any returned effect.

For nontransactional HTTP, media/TTS, native-input and plugin effects, persist
acceptance and a stable effect identity before dispatch. Deduplicate that
identity. A crash between dispatch and confirmed completion produces an unknown
outcome; startup/reconnect must not blindly repeat it. Provider-supported
idempotency may permit a separately specified recovery protocol using the same
key, but may not be assumed for a generic plugin or output device. Headless
hosts reject unavailable output capabilities.

## Required proof before release

- Exact-source review, explicit enable plan and complete final post-transition
  manifest remain one coherent approval operation.
- Same raw action/event IDs, disabled/unselected/missing/tombstoned references,
  multi-subject restrictions and shared dependency effects are covered.
- Dependency, grant, plugin, settings, policy and authority changes invalidate
  approval, including delete/recreate and change-away/change-back cases.
- A deterministic writer/revoke race proves authorization stays held through
  the actual business commit; batch failure leaves no partial approval.
- Real process termination covers before intent, before business commit, after
  commit, before reply and before/after local result persistence.
- Historical replay after revocation/edit/restart returns the original result
  without reinstalling its state; expiry and ID-reuse behavior remain correct.
- Nested emits, scripts, plugin observation/intents and response events retain
  restrictions and cannot read unrestricted globals or execute as Owner.
- Every final sink enforces current permissions after delays, queueing,
  rendering, revocation and shutdown; unknown external outcomes are not replayed.
- Concurrent Owner/moderator/agent, physical two-PC/LIVE, real OS credential
  stores, platform and workload acceptance remain separate required gates.
