# Desktop commands and durable client recovery

Status: workflow/event disable/delete, the native points command coordinator,
version-4 journal, bounded points view/adjustment form and automation status
board are implemented, 2026-11-08. Explicit native workflow draft persistence
and local conflict comparison/merge are also implemented. The host separately
accepts author-owned disabled automation proposals over the remote SDK/API and
supports explicit local-owner adoption. The isolated controller also has a
private JSON proposal editor and fixed durable create/update commands. Remote
owner adoption, activation, delegated execution, other domains and complete
release acceptance remain pending. See the [client guide](LIVE-COLLABORATION-CLIENT.md),
[host contract](LIVE-COLLABORATION-SPEC.md) and
[executed checks](LIVE-COLLABORATION-VALIDATION.md).

## Automation events

The **Host automations** board shows closed event/action summaries under the
current read grant. Actions remain read-only. Choose **Disable event** or
**Delete event** on a complete event and confirm its host, name, raw ID and exact
base revision. A points refresh does not invalidate this confirmation. Event,
connection, identity or grant changes require review again; loss of read access
hides rows and closes any captured confirmation.

The native bridge creates `disable-event` / `delete-event` intents with fixed
`automation` resource kind and `event` scope. They share the same pending gate
and local history as workflow and points commands. Version 1/2/3 journals migrate
to version 4 without changing original IDs, pending state, points results or
history; new operations in an old format are rejected. The intent is synced
before dispatch, and event family/ID/revision and the minimal returned receipt
are checked before recording success. Reopen/reconnect resolves the original ID
without sending the mutation again, including after later owner changes or
removal of write permission while read/outcome access remains.

The board never creates local host databases, stores source/config or opens an
owner definition. Its read-only action rows may share raw IDs with events while
remaining separate resources. The separate **Automation proposals** editor below
uses only explicitly retained author text. Owner adoption, activation and
delegated execution remain outside the remote controller boundary.

## Native automation proposal editor

Choose **Automation proposals**, a family and canonical raw ID. The starter is a
disabled JSON definition. Remote status is never converted into source. Inspect
the target: the reply is a closed summary plus its exact family revision, or
absence with revision zero/retained tombstone. The inspection carries an opaque
native token bound to epoch, connection, host instance, grant, family and ID.
Only the latest inspection is valid.

The optional field-based builder can prepare a disabled points action for a
specific viewer or the viewer named by each triggering event. The latter uses
the fixed public `event.user.uniqueId` template; it does not execute or submit
the action. The host and owner still review and approve the saved definition,
and each execution requires an event containing a viewer and the current
viewer/points grants and limits. Preview, local save and host submission remain
separate explicit steps.

For event drafts the builder accepts multiple unique action IDs, lets the
author remove or reorder added IDs and choose all enabled actions in order or
one enabled action at random. The currently typed ID is also included in the
preview; adding it first preserves it before selecting another. The host checks
references and the owner reviews the complete dependency set, including actions
that are disabled or not selected by a random run. Building the draft does not
enable or execute anything.

**Use this status as base** requires explicit confirmation. It chooses create
for an absent target and replacement for an existing one, preserving your text.
It does not save or send. **Save proposal on this PC** separately confirms
retaining the complete text, including sensitive values. **Send proposal for
review** shows the exact saved text, host, family/ID, mode and base. Edited text
must be saved again; a successful submission requires a fresh inspection before
another send. Existing Owner/foreign records remain protected by host policy.

Private `<profile UUID>.proposals` storage is independent of workflow drafts.
It binds profile, HTTPS origin, host, principal and session; family-qualified
keys separate equal event/action IDs. Version 1 stores a decimal global revision
and at most 128 `{kind,id,mode,baseRevision,text,updatedAt}` copies. Text is bounded
to 256 KiB per copy and the file to 4 MiB. Invalid/incomplete JSON can be retained
but cannot be sent. Saves/removals use global CAS, private files, the profile
lease/journal mutex, fsync and atomic replacement. Reads never promote leftover
staging text. Forget cleans both saved/staging files. File errors preserve text;
a catalog refresh never silently advances an open editor's replacement fence.
No proposal source enters WebStorage, host audit or command-journal metadata.

All native proposal methods require the current
`{epoch,connectionId,hostInstanceId,grantRevision}` tuple. Native read/save/delete
checks automation read and exact family/ID permission before and after local
I/O. A live restriction denies access. Losing the connection or grant clears
the displayed source and captured consent; delayed replies cannot restore it.
Local saves may use a read-only grant. Dispatch additionally requires edit,
the host's `automationDrafts` feature and the shared pending-command gate.

| Native method | Additional parameters | Result |
| --- | --- | --- |
| `controller.proposals.list` | `offset?` | Current tuple, `storeRevision`, metadata rows (32/page), `next` |
| `controller.proposals.get` | `kind,id` | Tuple, `storeRevision`, one bounded `draft` or null |
| `controller.proposals.save` | `kind,id,mode,baseRevision,text,expectedStoreRevision,acknowledgeStorage:true` | Tuple and new store revision; no host write |
| `controller.proposals.delete` | `kind,id,expectedStoreRevision` | Tuple and new store revision; host unchanged |
| `controller.proposals.inspect` | `kind,id` | Tuple, `inspectionId`, `kind,id,revision,summary` |
| `controller.proposals.submit` | `kind,id,inspectionId,expectedStoreRevision,acknowledgeProposal:true` | Shared journal projection |

Rust loads the exact saved copy, checks its inspected base/mode and disabled
JSON shape, generates the command ID and syncs its intent before dispatch.
It rechecks the profile/grant and private store revision after persistence.
The page cannot supply source, expected host revision, arbitrary method or
command ID in the submission call. The host still performs final authorization,
ownership and transactional CAS/shape validation.

The journal uses `create-event-draft`, `update-event-draft`,
`create-action-draft` and `update-action-draft`. Each intent retains only the
private `proposalRevision` string alongside normal target/base metadata, never
source or a guessable source hash. Success checks the exact four-field disabled
proposal receipt, family/ID, original ID/host instance and next revision.
Reopen/outcome lookup uses the original command ID and never reloads the saved
body for replay. Unknown outcomes continue blocking all mutation domains until
resolved or explicitly reviewed. Saved text/base stay historical after success;
the receipt cannot reconstruct a host definition or silently rebase an editor.

## Automation proposals through the SDK/API

Approved SDK/CLI clients can call `automation.create/update` with an explicit
`record.enabled:false`, `automation.read` plus `automation.edit`, current
family/ID permission and all three durable fields: `commandId`, `hostInstanceId`,
`expectedRevision`. A `resources.automation` restriction uses `action:<id>` /
`event:<id>`; omitting it permits the whole automation domain. Their versions
remain distinct for equal raw IDs. A `live` resource restriction
denies these global operations. New IDs become delegated proposals. Existing
records and retained tombstones remain editable/recreatable only by the same
author principal, and owner-owned records cannot be claimed. An action write can
also be denied when an owner or other principal's event references that ID,
including disabled events or unresolved references.

Fresh action proposals require a nonempty canonical `typeId`; events require a
nonempty canonical `trigger`. These names may be unknown plugin/type names, but
must be at most 128 UTF-8 bytes without wildcards/control characters. Known
configuration/filter/schema/mode fields are structurally checked and missing
editor fields receive defaults. Invalid shapes reject atomically rather than
breaking the owner's projection. This is structure validation, not effect
approval; historical receipt replay and legacy owner definitions remain intact.

The result has exactly `{kind,id,enabled:false,pendingReview:true}`, plus
family-scoped versions and the durable receipt in response metadata. Remote
list/get and recovery continue to expose only closed
`{kind,id,name,enabled,definitionRedacted:true}` summaries. Definition text,
config, scripts, full snapshots and trusted provenance are never returned to
remote authors. Keep the submitted candidate locally; this receipt cannot
reconstruct it.

Use `collaboration.commands.get` with the original ID after an unknown response;
it requires `commands.read` and current automation read/resource permission.
Retained create/update replay returns the original minimal result even after
owner adoption, subject to current authoring capability/scope checks, and never
overwrites the later owner definition. `pendingReview:true` in that result is
historical. A fresh update after adoption is forbidden to the previous author.

The local owner reviews the current full definition, dependencies and matching
`automation.authority.get` revision, then separately calls
`automation.adopt {kind,id,acknowledgeAuthority:true}` with durable command ID,
current host instance and reviewed `expectedRevision`. The owner-only authority
read returns `{kind,id,provenance}` with authority kind, delegated principal/
session when applicable, decimal-string generation and matching resource
metadata. A never-created ID has null provenance. Adoption returns
`{id,adopted:true}`; transferring a delegated record changes trusted authority
to owner, advances version/generation and disables that record in one
transaction. Each delegated dependency needs its own adoption. Activation
requires a later owner enable for automatic operation. Explicit owner fire/test
retains its usual rules after authority transfer. An already owner-owned target is a no-op.
Typed SDK `automation_authority_get` and `automation_adopt` wrappers preserve
their metadata; remote sessions cannot invoke either operation.

Ordinary owner edits, enablement, deletion/recreation and manual fire/test do
not transfer a proposal's authority. Delegated action dependencies block an
owner event even when disabled or omitted by a random choice. Effect manifests
and runtime execution under a delegated grant remain unimplemented. Explicit
owner adoption transfers future execution authority; it does not widen a remote
grant. Native proposals use the fixed bridge above; the board and raw
`controller.forward` do not submit definitions or adopt them. See the [API contract](CONTROL_API.md#version-metadata)
and [agent use](AGENT_API.md#typed-rust-sdk-tiktools-client) for submission.

## Use the controller

Connect the isolated desktop controller to an approved profile. The owner must
grant `workflows.read`, the exact workflow resource and `workflows.toggle` for
deactivation or `workflows.edit` for deletion. Reviewer presets remain read-only.
The host must advertise the corresponding implemented feature. Select a workflow
and choose **Disable workflow** or **Delete workflow**. The confirmation shows
the exact host origin/identity, workflow ID and expected revision. If the view
changes, review it again; the client never silently changes that precondition.

Only Rust creates the command ID and chooses the host instance from its active
connection. Before sending, it durably writes an intent. There is one unresolved
intent per profile; new mutations stay blocked until it is resolved or explicitly
reviewed. Projection updates remain authoritative; a command receipt does not
replace the workflow with an optimistic local value. Local drafts remain
separate, including after deletion. Missing-host drafts are accessible from
**Retained local drafts** only while the current grant still permits reading
that resource. Unsaved edits remain memory-only; explicitly saved private drafts
survive page/window close, as described below.

After a connection failure, the command may have completed. **Check outcome**
queries the same ID under `commands.read`; it never sends the mutation again.
Reconnection/profile reopen also attempts that read. An unknown, expired,
forbidden or unavailable result keeps the intent. **Review unknown outcome**
requires a separate confirmation: it archives the ID as reviewed unknown,
without claiming failure or undoing anything. Any later mutation is a new,
separately confirmed operation against current state.

The panel shows native processing, the pending ID and the latest 16 resolved
metadata records. A persistence failure blocks mutations until the profile is
reopened and its state reviewed. Profile switching fences old replies and cannot
send an old intent to the new host. A second controller cannot concurrently own
the same profile journal. CLI `remote forget` refuses an active journal or an
unreviewed pending command; disconnect after resolving/reviewing before forgetting.
Forgetting removes resolved local history and saved drafts as part of that
explicit action. If local draft cleanup fails, the profile metadata remains so
forget can be retried; revoked/deleted credentials are not silently recreated.

## Local drafts and conflict comparison

The native controller advertises `controller.status.draftsSupported`. Only that
local feature enables the draft file controls; no additional host capability is
implied. Opening an approved profile or reading drafts does not create a draft
file. A save requires `acknowledgeStorage:true` after the GUI shows the selected
host/origin, workflow ID, base revision and full-text/sensitive-value consent.
Typing, host updates, reconnects and page reloads never save a draft implicitly.
The native writer performs no host RPC or command dispatch.

The version-1 `<profile UUID>.drafts` file sits beside the command journal in
the private `commands` directory under the same exclusive profile process lease.
Its binding includes profile UUID, canonical origin, host, principal and device
session. It contains a decimal-string store revision and up to 128 records
`{id,baseRevision,text,updatedAt}`. Each text is at most 256 KiB UTF-8; the entire
serialized file is at most 4 MiB. Full capacity is an explicit `draft_limit`
error, never eviction. Unsupported, malformed, misbound, oversized or unsafe
files fail closed and remain intact. New Unix files use 0600; loaded files must
be owned/private, regular, single-link and opened without following symlinks; the containing
directory is private. Windows/macOS platform acceptance remains separately
required, including inherited filesystem protections.

The writer takes the journal's local write mutex, reads the current file,
compares `expectedStoreRevision`, validates the full candidate, writes a private
temporary file, flushes/syncs, atomically replaces and syncs the directory where
supported. There is one private `<profile UUID>.drafts.tmp` staging file per
leased profile, reused by the next explicit save after a crash rather than
accumulating unbounded orphan copies. Reads never promote an unconfirmed
staging copy. Forget cleans both saved and staged text. Every save/removal advances the store revision, including deletion
of the last draft; an old page cannot resurrect a removed draft using its old
revision. A changed copy returns `draft_conflict`. An uncertain write is not
retried by the page: review the saved copy before further replacement/removal.
Disk errors preserve the editor text; reads never overwrite a newer unsaved
edit. Neither the draft text nor local storage errors enter host audit/outcomes.

List reads return only paginated metadata (32 rows/page); a selected read
returns one bounded text. Native reads check the current ready epoch/view and
workflow-read/resource grant before and after file I/O, and filter metadata
before bridge output. Recovery or profile/permission changes fence delayed
responses. The frontend applies the same scope and generation checks. Private
files survive grant narrowing, but hidden drafts and saved copies are not
projected into that newly restricted view.

The conflict UI compares the current authorized host graph with the local draft,
shows both base/current revisions, and preserves the old base until an explicit
local merge choice. The saved copy is separately reviewable. Discarding removes
the saved draft before dropping editor text; a failed removal keeps the text.
These workflow draft controls do not submit host automation proposals, enable
remote activation or satisfy delegated-effect requirements.

## Native boundary

These are private desktop bridge methods, separate from the host Control API.
They never instantiate a local core or forward into owner IPC.

| Method | Parameters | Contract |
| --- | --- | --- |
| `controller.workflow.command` | `{epoch,viewRevision,id,expectedRevision,operation:"disable"|"delete"}` | Validate the current authorized row; generate/sync ID; send once; record a verified outcome or retain pending |
| `controller.points` | `{epoch,pointsRevision}` | Read the current bounded native display; never dispatch a host mutation |
| `controller.automations` | `{epoch,automationRevision,offset?}` | Bounded closed summary/version pages for both families; distinct family keys and independent display revision |
| `controller.automation.command` | `{epoch,automationRevision,connectionId,hostInstanceId,grantRevision,id,expectedRevision,operation}` | Only `disable-event` / `delete-event`; fixed event scope; native ID, synced intent and current identity/revision checks |
| `controller.points.select` | `{epoch,uniqueId?:string}` | Select one permitted viewer lookup or return to the top board |
| `controller.points.command` | `{epoch,connectionId?,uniqueId,delta,currencyName}` | Validate current profile/grant and supplied connection fence; generate/sync a points intent; dispatch once with `expectedCurrency` and no CAS metadata |
| `controller.drafts.list` | `{epoch,viewRevision,offset?}` | Local scoped metadata pages plus `storeRevision`; no host read/write or full draft bodies |
| `controller.drafts.get` | `{epoch,viewRevision,id}` | One current-grant saved draft or null, with its store revision |
| `controller.drafts.save` | `{epoch,viewRevision,id,baseRevision,text,expectedStoreRevision,acknowledgeStorage:true}` | Explicit private full-text save with local CAS; never a host mutation |
| `controller.drafts.delete` | `{epoch,viewRevision,id,expectedStoreRevision}` | Explicit private removal with local CAS; editor text is retained if removal fails |
| `controller.command.resolve` | `{epoch,commandId}` | Read only; query that profile's exact pending ID and validate the returned identity/operation/resource/revision |
| `controller.command.review` | `{epoch,commandId,acknowledgeUnknown:true}` | Explicit local review; retain an unknown-outcome record, permit a new deliberate operation |
| `controller.status` | `{}` | Includes `commandJournal`, `commandBusy`, `commandErrorCode`; no credentials/graph bodies in journal data |

The raw `controller.forward` bridge permits the completed read allowlist only;
it rejects mutation metadata and mutating/unclassified methods, including
automation create/update/adopt. SDK agents can use the generic remote SDK for
disabled author-owned automation proposals with their own durable metadata;
owner adoption requires the local-owner API. The desktop uses its native journal
coordinator for its implemented stop/delete and points mutations.

The coordinator validates selection/version before persistence and checks the
target/connection again before dispatch. A switch during disk I/O records a
known unsent rejection. Dropping a page request does not cancel the owned native
task or abandon persistence of a known result. Closing the process leaves the
last synced record for recovery. Native mutation work is serialized without
blocking projection reads; lifecycle changes can close/fence the old connection.

Success requires a matching command ID, host instance, principal (for queries),
method, target resource and committed receipt. Workflow success requires the
exact CAS revision transition. Points success requires a positive committed
viewer revision and verified unit, finite effective amount, total and level;
it must not invent a compare-and-set precondition.
The original result can be confirmed after the owner later changes the resource;
the recovery read never reapplies it. Only known pre-acceptance failures are
recorded as rejected. In particular, `forbidden`/`unauthorized` replies can be
egress checks after commit and cannot prove failure. Protocol/internal/storage
errors and lost responses remain uncertain.

## Storage and bounds

The selected profiles directory contains
`commands/<profileId>.journal` and an independent process lock. The journal binds
the profile UUID, canonical HTTPS origin, host UUID, principal and session.
Rebinding/copying another profile's file fails validation. Opening does not load
credentials; the existing profile engine still validates host identity before
OS-store access. An invalid journal fails closed.

Journal data contains only binding, ID, operation, resource ID, host instance,
timestamps and bounded outcome metadata. Workflow intents contain an expected
revision. Points intents contain requested delta and unit instead, and successful
records retain verified effective delta, total and level separately. It contains no tokens,
invitation/refresh secrets, workflow definitions or arbitrary server diagnostic
bodies. Pending intents are never pruned; resolved history is capped at 16 and
the complete file at 64 KiB. Decimal revisions remain exact strings.

Version 4 migrates version-1 workflow, version-2 mixed workflow/points and
version-3 complete-event
journals under their exclusive lease, using the same atomic sync/replace path.
Pending IDs, complete bindings, recent records and verified points results are
preserved. Pending commands block all mutation domains. New event/proposal
operations in older formats, any `proposalRevision` field in formats 1–3,
unsupported versions, malformed shapes and invalid point/CAS
fields reject without discarding an intent. An uncertain migration sync failure
prevents connection/dispatch until reopening succeeds. Older binaries must not
overwrite a version-4 file they cannot interpret.

The points coordinator is callable through the native bridge and SDK. The
points view and confirmation form are connected; the GUI always supplies the
reviewed connection ID and requires the current selected viewer and currency.
The common history
and unknown-review panel already shows requested units, effective adjustment
and confirmed historical balance. Those history values do not replace the
current host projection, including after a later award or unit rename. Native
status and command replies hide historical host result amounts while recovering
or outside the current points-read/viewer grant; the private durable record
remains available for later authorized recovery.

Writes use a private same-directory temporary file, file sync, atomic replacement
and directory sync on Unix. An uncertain persistence failure cannot dispatch a
request or replace an unresolved intent. Unix paths reject symlinks, hard links,
wrong ownership and public file permissions. The existing Windows private-ACL
and platform/power-loss acceptance requirements remain open; Linux checks do
not establish Windows/macOS durability or native keychain acceptance.

Tests cover real subprocess death before intent, after synced intent and after
synced result. A separately terminated native points coordinator also verifies
zero/one host adjustments at those boundaries and after the committed TLS reply.
Migration tests terminate before replace, after rename and after directory sync.
Independent TLS coordinator tests and a packaged Vue/Wry
test through the actual bridge/journal/TLS owner. The GUI test uses an isolated
in-memory credential-store fixture and no production account or LIVE. Physical
two-PC, real OS-store, other-domain and external-effect acceptance remain required.
