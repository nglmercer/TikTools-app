# Desktop Cloud v2 implementation and acceptance specification

Status: **the native v2 foundation and read-only Cloud catalog and library UI are implemented. Product sync cutover, local application and full Desktop v2 acceptance are pending.**

The earlier inspected Desktop baseline was 7864562, including the atomic Owner
library delivery e6728c9. The reviewed backend commit is RemakeBackend 1c06410,
including byte-ordered v1 snapshot pagination, SDK pull counts, canonical shards
and SDK capability discovery, following bounded pagination 9f7e642,
catalog compatibility 995d496, integrity changes c0fb951 and v2 foundation
0c552b3, and its
docs/CLOUD-REDESIGN.md, docs/CLOUD-API-V2.md and latest
docs/IMPLEMENTATION-STATUS.md. The backend owns portable cloud contracts.
Import its reviewed contract/client changes without replacing Desktop-specific
authentication, durability or safety fixes with an older copy.
That backend commit includes the digest-repair, recovery, byte-ordered
pagination and browser/CI changes covered below.

[CLOUD-SYNC.md](CLOUD-SYNC.md) describes the existing v1 implementation.
[CLOUD-ACCOUNT.md](CLOUD-ACCOUNT.md) describes authentication.
[CLOUD-REGISTRY.md](CLOUD-REGISTRY.md) describes signed package distribution.
Local application uses the
[atomic Owner library coordinator](LIVE-COLLABORATION-AUTOMATION.md#atomic-owner-library-transitions).

## Scope and current evidence

Cloud v2 synchronizes account configuration between devices. It does not relay
LIVE events, execute actions on the server, authorize another PC to control a
running live, or replace the local collaboration transport. A Cloud moderator,
a local Owner and a delegated LIVE controller are separate authorities.

| Area | Inspected baseline | Target status |
| --- | --- | --- |
| Account login, native bearer session, keychain and local-only operation | Existing Desktop implementation | Retain; verify with v2 |
| Plugin-selection sync and durable v1 state | Existing implementation using the local crates/cloud-client | Explicit migration required |
| Atomic local profile/template/graph changes and original-outcome recovery | Existing native Owner library implementation | Reuse for reviewed local imports |
| Backend portable v2 contracts, native transport and all-five-collection state/store/coordinator | Implemented and locally tested; additive exports preserve v1 | Product adapters and explicit cutover pending |
| Release Cloud origin policy | Runtime overrides ignored in release; build-time provisioning and debug override retained | Four policy unit tests passed; broader trust/product restrictions still pending |
| Single-instance product catalog, private/public library and expanded sync UI | Read-only Cloud catalog and document browsing are present; custom registries and local imports remain | Verified installation, source restrictions, reviewed library import and expanded sync UI pending |
| Actual two-Desktop v2 offline/recovery acceptance | Not established by prior local Owner tests or v1 mock E2E | Pending |
| Production trust, deployed migration and external infrastructure review | Separate backend/operator gates | Not established by this specification |

Tests of v1, local Owner journals, backend HTTP routes, a backend-free web demo
or two SDK sessions are useful evidence for their own boundaries. They do not
by themselves complete the two-Desktop v2 acceptance below.

### Implemented foundation and local checks, 2026-11-08

The native [v2 transport](../crates/cloud-client/src/workspace.rs) provides
validated workspace, catalog, private/public document and account-export calls.
The separate [WorkspaceState](../crates/cloud-client/src/workspace_state.rs)
and [WorkspaceStore](../crates/cloud-client/src/workspace_store.rs) retain all
five collections in private origin/account partitions with generation CAS,
atomic file replacement and independent operation leases. They preserve frozen
base bodies, uncertain requests and newer local edits across restart. Exact
decimal comparison tolerates equivalent JSONB number representations without
conflating distinct values or changing frozen request identities.

[WorkspaceCoordinator](../crates/tiktools-cloud-sync/src/workspace.rs) exposes
explicit bootstrap, durable staging, bounded sync, read-only comparison,
fresh-read conflict resolution and snapshot rebuild. A prepared write is saved
as unconfirmed before HTTP; a lost response cannot be silently replayed or
cleared by an equal current body. Review is bound to the coordinator, identity,
local generation and remote revision/content. Cursor expiry persists a rebuild
requirement; rebuilding changes only the fetched cache and preserves pending
work. An older snapshot cannot rewind point reads or feed state. A delayed
absence read can complete only its own unchanged initial delete generation.

These APIs are exported, but the application still calls the v1 coordinator.
The coordinator now offers guarded sync, review, resolution and rebuild calls
bound to one session and eligibility permit. The authentication manager can
recognize that exact session after logout and same-account sign-in. A native
cutover preview captures and rechecks the legacy snapshot, session and consent;
it does not itself activate v2 and covers plugin selections only. It builds
the candidate outside the admission and identity locks and rejects changed
inputs afterward. The caller still needs a local-source fence. There is no
product preview RPC/UI: Cloud account and consent changes are not yet
broadcast to other windows with ordered invalidation, so a displayed preview
could otherwise expose stale account data. A cooperative cross-process protocol fence
and per-identity v1 partitions are being validated; product wiring still needs
session/opt-in invalidation and a local-source fence. Guard
checks cannot recall an HTTP request already dispatched, or replace an atomic
local-application consent fence.

The in-progress v1 uncertain-upload recovery must not be treated as a finished
product flow. A plugin change can currently be applied locally before its
sync staging succeeds, including while an exclusive recovery lease is held.
Recovery also needs a durable, identity-bound application intent: accepting a
Cloud value must survive a crash and handle an absent Cloud record as disabled,
without overwriting a newer local change. A shared mutation boundary, durable
staging and session/opt-in/source revalidation are required before exposing
that choice as complete. Cooperative locks do not fence older binaries.
The product currently permits read-only review but rejects every uncertain
upload resolution, including keep-local, until those protections are wired.
An internal SQLite transaction can now persist a plugin state, source
generation and identity/consent-bound outbox candidate together; this storage
primitive is not yet connected to product mutations or coordinator delivery.
The in-progress coordinator bridge moves staged candidates to durable receipts
and rechecks the exact pending edit, source generation, consent and runtime
epoch before upload. A durable per-account marker can make cooperating legacy
entry points fail closed, but product activation and compatibility with older
binaries remain unfinished. Package mutation coverage and conflict resolution
still need the same boundary before product activation.
The existing product v1 application path also needs a session, consent and
local-source fence around applying downloaded selections. A network result
alone is not permission to alter a plugin after an account switch, opt-out,
or a newer local edit.
Current persisted plugin state writes and removals also advance the retained
source generation in the same SQLite transaction, including repeated writes
and local-only changes. This invalidates stale receipts across delete/recreate
and remote changes, but it does not yet make legacy writes participate in the
coordinator admission lease, cover all package filesystem transitions, or
atomically enqueue their sync candidates. Those paths still block activation.
The local package install, reviewed update, rollback and uninstall entry points
invalidate source evidence before and after attempting filesystem changes.
Uninstall deletes the persisted plugin row only if its first source generation
is still current; a concurrent update or database error is surfaced even if
files were already removed. These fences make some stale candidates invalid,
but do not stop uploads during package I/O or a stale observation staged after
the final fence. A shared admission lease and durable package-operation journal
are still required before activating this path.
An internal package journal and short cross-process admission callback now
preserve uncertain operations and prevent replay. Sync outbox and local-apply
storage reject a plugin with an active package journal. A private orchestrator
can reserve that journal before a filesystem effect. The installer now offers
a cooperative cross-process store lease and retains the exact reviewed archive
in a private temporary snapshot until installation completes. Product package
operations do not yet use the journal: trusted artifact/runtime reconciliation,
recovery UI, older-binary coordination and a bounded receipt-retention policy
remain required before activation.

An experimental runtime quarantine blocks known unfinished plugin applications
after restart and rejects several activation and late-response paths. Finalized
records now have a separate witness written in the same SQLite transaction;
startup validates retained records and witnesses, failing closed if evidence is
missing, malformed or orphaned. A build without Cloud support validates only
the basic finalized identity and matching witness, not the complete Cloud
contract. Matching copies do not protect against coordinated modification of
both tables. Retention is capped at 4096 records; reaching
the cap blocks new application until a safe compaction and recovery protocol is
implemented. Finalized records made by earlier experimental builds have no
witness and require explicit review and migration; they are not silently
accepted or repaired. This is not an activation gate: an already dispatched
effect cannot be recalled and quarantine is not propagated live to another
process. The product must keep remote application disabled until these gaps and
recovery are addressed.

Account UI transitions coordinate restore, login and cancellation. The current
desktop flow reserves an opaque, bounded, expiring operation identity before
starting login, then binds begin, completion and cancellation to that identity.
After an uncertain begin, cancellation invalidates the same attempt even if
the callback setup finishes late; a stale cancellation cannot stop a newer
correlated attempt. Older clients that call the uncorrelated begin and cancel
methods retain their original cancellation race. Lost reservation responses
leave inert entries that expire; a maximum of 128 can be held for five minutes.
The desktop bridge and browser wait longer for login completion than the
native callback deadline, while other RPC deadlines remain unchanged.
Legacy conflict resolution must also share the outbox admission boundary: a
conflict based on an older staged value cannot be rebased or discarded while
a newer local candidate awaits confirmation or delivery. A keep-local decision
also needs a durable journal joining the reviewed Cloud record, old and new
pending values, source generation and consent before changing either the JSON
state or the SQLite receipt. A later decision must re-read Cloud; the review
alone does not authorize a rebase.
An in-progress native keep-local journal now freezes that decision across
both stores. If a later local edit invalidates the frozen source, recovery
retains the journal and blocks uploads. A separately reviewed native
abandonment can restore the original conflict and retain a newer candidate
when that candidate belongs to the same account and has current source and
runtime confirmation. A separate reviewed abandonment accepts only evidence
of a newer source generation, restoring the conflict without revealing or
copying another account's value; it does not authorize any upload.
Reauthorizing a journal from a previous consent identity remains unimplemented.
These paths remain unavailable in product.
The in-progress consent epoch migration preserves legacy numeric identities
and assigns newly confirmed consent a 127-bit random identity, serialized as
decimal text. Legacy enabled settings without a usable identity require a
fresh opt-in; existing records are retained. The negligible collision risk
does not protect against restoring an old copy of all local state. A durable
initialization marker prevents an individually missing preference file from
silently turning edits admitted through the in-progress product boundary into
local-only edits. The existing v1 product path does not use that boundary yet.
Loss of both files together cannot be distinguished from a new installation. Older
binaries do not understand the new identity and may rewrite the setting using
their old scheme, so deployment must prevent concurrent use or downgrade to
those binaries before the new product path can be activated.

The read-only Cloud catalog uses typed summary and version-page RPCs against
the configured origin, with generation and sizes as decimal strings. Bounded
pages retain filter and generation identity; catalog data never authorizes
installation. The UI offers a user-selected legacy fallback only when the
backend explicitly reports the new capability absent. Existing custom source
controls and verified installation remain separate work. The signed registry
and Cloud backend now use a canonical path for the legacy `..` shard in new
generations. Existing immutable generations and older clients require a
separate compatibility audit and, if affected, a newly signed promotion.

An explicit, read-only `sync.v2.discover` check validates the configured
backend's advertised workspace contract without sending credentials. It is
informational and always reports Desktop v2 synchronization as unavailable;
it does not migrate, sync, or apply account data.

No v2 product synchronization RPC/UI, account eligibility integration, single-engine cutover,
collection-specific local application or durable local-application outcome
adapter is wired yet. The foundation never applies downloaded source itself.
Its bootstrap helper preserves the v1 file and imports no v1 cursor or revision;
it does not constitute product migration consent or a completed cutover.

| Executed local check | Result and boundary |
| --- | --- |
| Focused foundation package suites | 86 passed: 48 client, 5 contracts, 14 authentication and 19 coordinator tests; includes existing regressions, native state/storage and loopback HTTP fixtures |
| Core regression suite | 755 unit tests and 8 integration tests passed; this is not two-Desktop v2 acceptance |
| Control API, collaboration and client regression suites | 302 passed across the selected packages |
| Clippy across the selected 11 Rust packages | Passed |
| Feature-disabled core check | Passed; existing unused-code warnings remain |
| Rust 1.95 foundation compiler check | Client, contracts, cloud authentication and coordinator passed with all targets/features |
| Rust 1.89 SDK compiler check | Passed with all targets/features |
| Current frontend production bundle | Built successfully |
| Packaged Owner library browser regression | Passed against rebuilt current assets in 95.87 s; eight operations in EN/ES, sixteen native receipts and lost-reply recovery; this is local Owner behavior |
| Normal Desktop/CLI build | Passed with the origin override unset and CLI remote-control/local-web features enabled |
| Release-origin policy unit cases | 4 passed, included in the authentication suite above |
| Session fencing and cutover preview | 15 authentication and 26 sync/coordinator tests passed locally, including same-account re-login, in-flight acknowledgement and preview/bootstrap ordering; product wiring remains pending |
| Cloud catalog adapter | Loopback HTTP pagination, exact 64-bit values and oversized-response rejection passed locally; installation and actual Desktop browsing are separate |
| Catalog compatibility and client surface | Five SDK catalog tests passed in each repository; a PostgreSQL pagination test checked legacy-ID routing; client parity (10 tests) and the CLI catalog parser passed |
| Bounded catalog, local registry and UI | PostgreSQL exercised byte-bound pagination and legacy workspace IDs; bounded SDK tests passed in both repositories; a signed local registry round-trip and 31 web tests passed. The packaged Chromium fixture passed against native IPC and loopback Cloud HTTP in English, Spanish and mobile layouts, including 50+1 pages and an exact generation above 2^53; this is not a two-PC acceptance test |
| In-progress v1 outbox admission | 61 coordinator tests passed, including marker, account and restart fences. SQLite receipt and adapter tests passed, as did a loopback HTTP case with a newer edit during an admitted write. The product does not yet use this path; package mutations, conflict resolution and local application still need fencing |
| In-progress conflict recovery | The 98-test coordinator suite passed; SQLite journal tests and loopback HTTP integration covered failed cross-store commits, restart, source reconfirmation and preserving newer edits, including a second account's private candidate. These private paths are not product activation or two-Desktop acceptance |
| Backend populated pre-v2 upgrade, explicit digest repair and actual SQL archive restore | Passed on native PostgreSQL (17.31 s) and pgrust (18.61 s), plus 2 pure URL-guard tests; database-only scope |
| Earlier backend v2 PostgreSQL cases | At the preceding checkpoint, 6 passed on each SQL runtime, including byte-ordered punctuation pagination; the earlier 67-case suite also passed on native PostgreSQL |
| Backend declared minimum compiler checks | Rust 1.95 workspace and Rust 1.89 SDK checks passed |
| Backend authenticated rejected-Origin browser checks and CI coverage | Local browser checks passed with native PostgreSQL and pgrust; admin build/Playwright workflow coverage was added, but GitHub Actions itself was not run |

These checks cover the locally inspected source and disposable fixtures.
The backend recovery drill excludes object volumes,
credentials and a live production cutover. All thirteen acceptance scenarios
at the end of this document remain required; none is discharged merely by this
foundation table.

## Instance, authentication and trust

1. A product build consumes one operator-configured Cloud origin. Account
   login, capabilities, plugin discovery, documents, workspace sync and account
   export use that same normalized origin. Neither browser requests, imported
   documents, sync values nor controller profiles can replace it.
2. Release builds now ignore runtime TIKTOOLS_CLOUD_ORIGIN overrides. Operators
   provision another instance or disable Cloud at build time; debug builds retain
   the explicit runtime development override. Each such instance has its own
   identity, state and independent pins. Disabling optional Cloud preserves local
   operation. The remaining registry-selector and trust UI restrictions below
   are separate product work.
3. Root public keys come from operator/build provisioning through an independent
   trusted channel. Downloaded capabilities, catalog JSON, document hashes and
   fetched root previews never establish trust. Missing pins, wrong roots,
   revoked keys, digest mismatches and incompatible packages fail closed.
4. Remove end-user registry addition, root enrollment, source selection,
   priorities, scopes and registry enable/disable from Settings, native product
   RPC inputs and client wrappers. Hiding the form alone is insufficient.
   Explicit registry arguments and registry selectors inside package spec strings
   must not provide a bypass. Legacy configuration must not silently add another
   network source after upgrade.
5. Preserve update-channel policy, verified package inspection, consent tokens,
   permission review and the native installer. A separately allowed manual local
   package import remains governed by its explicit sideload/consent policy; it
   cannot mutate Cloud origin or registry trust.
6. Public catalog/capabilities are anonymous. Private documents, workspace sync
   and v2 account export use the native session in Rust. Tokens stay in memory or
   the OS credential store, never Vue state, control responses, browser storage,
   sync files, portable documents or logs.
7. Backend moderation remains cookie-only and requires a current Cloud admin or
   moderator role; authors cannot approve their own content. Local Owner status
   and LIVE grants confer no Cloud role. The initial native integration should
   open the fixed instance's browser review portal for privileged moderation.
   A native bearer moderation UI requires an explicit backend contract change;
   do not work around the cookie boundary by copying browser credentials.
   The existing backend admin UI has not acquired document-moderation screens
   merely because v2 routes exist. Implement and verify that browser flow in
   the backend repository, or present the operation as unavailable; a link to
   an unrelated plugin review page is not document-moderation integration.
8. Session/account changes invalidate in-flight UI responses, reviews and write
   eligibility. A late response cannot restore another account's rows, complete
   its draft under a new session or reveal its private document.

## Workspace collections and local behavior

Exactly the following five collections are supported. Unknown collections,
invalid keys and unknown schema fields fail validation; they are not silently
discarded while advancing the cursor.

| Collection | Key and cloud value | Desktop adapter |
| --- | --- | --- |
| plugin-selections | Plugin ID; schemaVersion 1, pluginId, version, enabled | Stage explicit local selection changes. A download does not install, update, enable or uninstall a plugin. Review any proposed local activation separately. Missing or incompatible installed versions remain visibly unapplied. |
| rule-templates | Portable document ID; original templateVersion 1 object | Preserve the original portable document and identity. Import into the local gallery only through an explicit reviewed Owner library operation; execution is a separate local decision. |
| profiles | Portable document ID; original profileVersion 1 object with inline templates and parameters | Preserve profile and template data. Parameter resolution, generated local graph IDs and replacement membership are reviewed together through the atomic Owner library coordinator. |
| plugin-settings | Plugin ID; schemaVersion 1, pluginId, settings object | Export only portable fields under a defined plugin-specific credential policy. Validate against the installed plugin locally and resolve local references before an explicit settings application. Never upload masked placeholders as actual values. |
| preferences | Literal key desktop; schemaVersion 1 and optional theme, locale, reducedMotion, updateChannel | Use a native portable preference model and a reviewed UI projection. theme is system/light/dark; updateChannel is stable/beta; locale is a bounded identifier; reducedMotion is boolean. Unsupported values remain visible and unapplied. |

Current locale persistence is browser-local and AppShell forces dark mode.
Merely storing cloud preference JSON does not implement theme, locale,
motion or update-channel application. All advertised fields need a concrete
native/local consumer, or must be reported as supported cloud data awaiting
local application. Do not silently replace an unsupported locale or setting.

Runtime globals, TikTok/OAuth credentials, cookies, session tokens, signing
material, native binaries and ephemeral LIVE state are not additional sync
collections. Do not serialize whole application state or all plugin settings
as a shortcut.

## Portability, credentials and source review

- Apply the backend-owned portable validator on native upload and download.
  Each portable document/value is at most 256 KiB of encoded JSON. The current
  backend bounds include depth 24, 8,192 nodes, strings of at most 8,192 bytes,
  bounded arrays/maps, and ASCII document IDs of 2–64 bytes. Profile documents
  contain 1–32 template entries. Collection keys and embedded IDs must match.
- These are cloud limits, separate from the broader local Owner library
  budgets. A valid multi-megabyte local profile is not automatically portable.
  Report the offending document/field; never truncate, silently rename or drop
  fields to make an upload pass. An explicit separately named portable copy is
  a user choice, not an automatic migration.
- The existing template text importer defaults to 64 KiB. A valid 256 KiB
  cloud document needs a bounded cloud adapter; do not feed it through the
  smaller text-file limit or enlarge every legacy import without a reason.
- Preserve raw portable documents for revision/hash comparisons. Apply parser
  defaults only to derived display/application data. Cloud document hashing and
  local Owner command hashing are different contracts; do not substitute one.
- Recognized credential fields may contain a local secretRef object or an exact
  globals reference allowed by the backend contract. Desktop currently renders
  globals strings but has no general secretRef resolver. Implement a native
  resolver with explicit field/type policy, missing-reference diagnostics and a
  local preview before application. Never send resolved credentials back to
  Cloud or include them in a rendered comparison.
- Generic validation cannot detect every secret hidden in free text or arbitrary
  settings. Author review and plugin-specific portability rules are required.
  Upload/download does not fetch action URLs, execute templates or grant plugin
  permissions.
- Expand parameters with existing pre-allocation bounds before constructing
  strings/graphs. Freeze generated local IDs and the complete candidate once;
  persist them natively when needed for recovery. A lost reply or restart must
  not regenerate graph IDs and create duplicates.
- Bind a local review to Cloud origin/account, document UUID or collection/key,
  exact remote revision/content identity, local host/instance and the displayed
  local Owner library revision/definition digest. Show human names, affected
  records, membership changes, missing requirements and activation state.
- Use one atomic Owner library operation for related gallery/profile/graph
  changes. Do not loop over single-record journals or write reserved app-state
  keys. Preserve existing authority/quarantine, adoption and approval gates.
  Independent Cloud publication review does not grant local execution authority.
- New remote-derived graphs remain inactive until the Owner explicitly reviews
  activation under the local authority rules. A cloud tombstone does not
  silently delete local source, uninstall a plugin or erase an unsubmitted draft.
- Keep Cloud receipt, local import receipt and execution approval separate.
  A successful local save/import remains successful if a subsequent Cloud upload
  fails; a committed Cloud write remains committed if local application fails.

## Durable coordinator, conflicts and recovery

### State and lifecycle

The implemented versioned v2 store partitions by normalized pinned origin and
account ID. It retains each account's pending edits, remote bases, fetched cursor,
tombstones, conflicts and bootstrap marker. A different account cannot overwrite
that partition. The product adapter must additionally persist local-application
outcomes, fence stale sessions and stop networking on logout while preserving
recoverable nonsecret pending work for the same account.

Persist staged edits before reporting them queued. State writes use generation
checks, private paths, atomic replacement and the existing durability rules.
A stale second process reloads/reconciles instead of overwriting. Surface
storage failure rather than claiming an edit is durably queued. Keep source
and command identities in the native private store, not browser storage.

Network work is bounded, cancelable and scoped to a captured identity/session
generation. Offline mode and opt-out are inert for network/application work.
No background scheduler is a substitute for an explicit opt-in or local review.
Remote application must not feed the same mutation back into the upload queue.

Track fetched cloud state separately from reviewed/applied local state. Saving a
valid downloaded record can advance the fetched cursor without claiming local
application succeeded. Missing plugins, unresolved references, unsupported local
capabilities and rejected local CAS remain visible in the apply backlog.
Validate every record in a page before persisting its entries and cursor
together. Preserve per-record revision ordering and tombstones; an older page
must not rewind a newer record. Malformed, negative or future cursor responses
must not silently reset state or skip work.

### Conditional mutations and uncertainty

Create uses If-None-Match: *. Replace/delete use the exact reviewed strong
If-Match revision. Conflict is not permission to send another write against the
latest revision. Identical server no-ops do not create a new cursor/revision.
Keep cloud revisions/cursors exact in Rust; project them as decimal strings in
browser-facing contracts when JavaScript cannot represent them safely.

When a reply is lost, retain the original candidate, operation, key, base and
identity. Read the current record and original known outcome before offering an
explicit rebase/review. Never infer success merely because some current value
resembles the draft; do not silently discard pending work or resubmit it with a
new identity. Newer local edits remain a distinct generation and cannot be
replaced by a late acknowledgement.

The current workspace HTTP API does not offer the native Owner command ledger
or command-ID lookup. A current-state read is not an invented receipt for an
unobserved cloud request. Distinguish a confirmed original response from an
unknown original write and the current value shown for explicit comparison.

Conflict UI must display the reviewed base, local candidate and current cloud
value/tombstone, with collection and human target. Resolve by a captured
conflict/review identity, not the current key plus a bare keepLocal boolean.
Refresh or account/host changes invalidate consent. Keeping local stages a
new explicitly reviewed base; taking cloud does not automatically authorize
local activation or destructive application.

For native local application, recover the original Owner command/receipt after
an ambiguous reply; never resend a graph with fresh IDs. Persist the association
between cloud revision and native application outcome. Where the local database
owns both, commit the application marker and affected sources together. Across
other persistence domains, use explicit domain CAS and durable recovery; do not
claim a single transaction across independent stores.

### Expired cursors, rates and quotas

An expired cursor returns 410. The foundation persists needs_rebuild and requires
an explicit rebuild call before further sync. It reads the consistent v2 export
and snapshot cursor, validates and atomically replaces the fetched cloud
baseline, retains local pending edits and re-evaluates conflicts. An invalid,
interrupted or stale rebuild leaves the previous baseline intact. The product
must expose that choice separately from local application; rebuilding never
adopts contents into the running host.

The current backend page size is 50 changes; current account limits include
1,000 records including tombstones and 8 MiB of value text. Feed retention is a
suffix bounded by 10,000 changes and 16 MiB plus the documented allowance.
Respect 429/Retry-After, quota 507, request/response caps and cancellation without
busy retry loops. Library quotas and workspace quotas are separate. v2 account
export is bounded at 32 MiB and is not an atomic cross-domain restore image.

## Explicit v1 cutover

1. Stop new v1 push/pull work under the coordinator's identity fence. Finish or
   retain ambiguous in-flight work for explicit recovery.
2. Read the existing v1 durable state without changing its cursor, pending edits
   or conflict flags. Preview which installed selections and pending changes
   will seed v2, including deletions and conflicts. Preserve unrepresentable
   data with an actionable error.
3. After opt-in/review, create a new v2 baseline and stage the selected values
   once. V1 and v2 tables/cursors are independent: never carry a v1 cursor,
   revision or ETag into a v2 request.
4. Persist a migration identity/marker and the staged v2 work atomically before
   considering cutover complete. Crash/restart, a lost reply or repeated Enable
   cannot stage duplicate work or erase a newer pending edit.
5. Run exactly one active sync protocol for that account/device. Preserve the
   old v1 state for recovery/export; do not double-write the same operation or
   automatically switch back to v1 after a v2 network/protocol error.

Migration is local to each device and identity. An account switch does not
acknowledge another account's cutover. Pending v1 conflicts require explicit
handling rather than being hidden by the new status screen.

## Catalog, document library and moderation UX

Plugin Browse consumes the anonymous v2 product catalog. Use its after cursor,
target filter and returned generation for the next page; refresh starts a new
generation deliberately. The current API searches plugin IDs, not rich package
names/categories. Do not expose unsupported filters or invent total/page counts.
Current yanks, advisories and key authorization must remain visible even for a
pinned older generation.

Use returned same-instance metadata paths to inspect the independently verified
signed package document, then retain existing package/permission consent and
native installation. Catalog availability and installation trust readiness are
distinct UI states: public discovery alone cannot make unpinned packages
installable.

The cloud library needs separate Public, My documents and local/imported views.
The in-progress native `cloudLibrary.list` and `cloudLibrary.read` adapters
provide typed, read-only public or signed-in private access; they do not import
or activate documents. Private replies are discarded if the initiating session
changes. Account settings can browse public and private metadata and read an
exact revision without importing it; the view clears private responses when
the visible account session changes. Reviewed import remains to be wired.
Keep backend document UUID/revision/status separate from portable document IDs,
local gallery IDs, local profile IDs and graph IDs. Public reads only expose
published revisions; drafts/pending/rejected/withdrawn content is private.
Editing a private document creates a new revision without replacing the public
pointer. Submit/withdraw require explicit actions; withdrawal needs a reason.
An already downloaded local copy cannot be remotely recalled.

Download to a staged preview, preserve exact remote identity/content, resolve
local parameters/references, show compatibility/activation implications and
enter the existing local atomic review. Navigation, another download or a late
response must not replace an open candidate. Explicitly dismiss/review it first.

Expose private create/list/read/history/update/delete/submit/withdraw and
account export through native typed adapters. Browser moderation should use the
fixed instance's authenticated review portal; independent approval, rejection
and withdrawal stay bound to the exact cloud revision and current Cloud role.
The local adoption/approval dialogs are not that moderation portal.

## Control API and implementation sequence

Exact new RPC names are chosen with the coordinator interface; these required
operations and bindings are normative. Existing sync.* is the v1 surface and
must not silently reinterpret an old cursor/state as v2.

| Slice | Concrete files and responsibilities |
| --- | --- |
| 1. Native contracts and instance policy | crates/cloud-client, backend-owned portable contracts, crates/tiktools-cloud-auth/src/origin.rs; typed v2 validation, one release origin, independent pins and explicit dev policy |
| 2. Durable v2 engine and cutover | crates/tiktools-cloud-sync, crates/tiktools-core/src/cloud_sync.rs and control/cloud_sync.rs; identity-partitioned state, stage/pull/push/rebuild/conflict/migration/apply-outcome operations |
| 3. Product catalog and removal of selectors | core plugin_packages.rs and control/plugin_packages.rs, control-api modules/registry.rs, client/CLI registry wrappers, plugin-package store/types, PluginsPage and SettingsPage; fixed-origin discovery and verified install |
| 4. Cloud documents and local application | Native library/document adapters; entities/rule-template and entities/cloud-*; RuleTemplateDialog, ProfileImportDialog and OwnerLibraryChanges; exact remote revision review into the native atomic library |
| 5. Settings and conflict experience | cloud-sync sync-types/store/CloudSyncEditor, cloud-account identity handling, native portable preference/settings adapters and locales; collection-aware comparison, unapplied backlog, retry/recovery/export |
| 6. End-to-end acceptance | Separate backend fixture plus two real Desktop instances/homes, real local adapters and current packaged Vue; the scenarios below |

The API must supply nonsecret capabilities/status, paged catalog/document reads,
versioned private edits, scoped pending/conflict/application metadata, explicit
comparison/review/resolve operations, opt-in/cutover/disable, bounded sync/export
and original-outcome recovery. Writes require the reviewed account/origin and
operation context; a boolean alone is insufficient authorization. Native APIs
must enforce boundaries even when callers bypass the UI.

Do not import feature UI into entities. Keep the injected original control
client identity shared by local Owner journals. Cloud work must not widen
controller/plugin transport budgets or confer LocalOwner access. The existing
LIVE host origin, TLS configuration, invitations and controller pairing UI stay
independent of the operator's Cloud origin.

## Required acceptance evidence

Run against a disposable real RemakeBackend v2 database/service with two
separate Desktop homes/accounts/session stores as appropriate. Preserve traces
and exact commands/results; do not describe mocks as real infrastructure.

| Scenario | Required observable result |
| --- | --- |
| Two devices, all five collections | A's reviewed portable changes reach B's fetched baseline; each supported local application is explicit and correct, unsupported application is visible, and a settling re-sync stages no feedback loop |
| Offline edit and restart | Both devices edit offline from the same base, restart, reconnect and show a deterministic conflict with base/local/cloud values intact |
| Lost cloud write reply | Drop one actual response after server commit; preserve original key/base/candidate and newer edits, recover/compare without silent overwrite or duplicate staging |
| Lost local import reply | Drop one actual Owner acknowledgement after atomic commit; reload preserves original graph IDs/receipt and newer draft, with no second import |
| Cursor expiry and interrupted rebuild | Force real retained-feed expiry; rebuild from a consistent export while preserving pending edits; interruption leaves the old durable baseline usable and local source unchanged |
| V1 cutover and crash boundaries | Existing records, pending edits, tombstones and conflicts survive one explicit migration; retries/crashes stage once; no v1 cursor reuse or dual writer |
| Account/origin/session switch | A's delayed requests/reviews cannot mutate or expose B's data; switching back resumes A's own pending work; opt-out/offline leaves local LIVE operation working |
| Local source or host changes during review | CAS/host checks reject stale consent; current source comparison is explicit and the newer draft remains; all local Owner journals still gate one another |
| Plugin distribution trust | Fixed-instance discovery, pinned-generation pagination, wrong-root/digest rejection, current yank/revocation handling and existing permission consent; no end-user source or root override |
| Credentials and unsupported data | Local references resolve only on device; resolved credentials and session tokens never enter Cloud or the new review/status DTOs; missing plugins, unsupported preferences/fields and invalid portability remain visible without truncation or activation |
| Bounds and identity precision | Exact UTF-8 256 KiB boundary, depth/node/string limits, bounded expansion, unsafe browser integer refusal and exact native revisions/cursors; malformed replies do not advance state |
| Document publication and roles | Own/private isolation, immutable revisions, author self-approval refusal, cookie-only moderator actions, withdrawal/public 404 and local-copy behavior; LocalOwner/LIVE roles never grant Cloud moderation |
| Desktop/browser behavior | Current packaged UI in EN and ES at desktop and 390 px, loading/read-only retry, selection-independent recovery, retained candidates across navigation, and no credential/command/source persistence in browser storage |

Also run relevant contract, coordinator, storage, native API, frontend, type,
lint and build checks. Verify feature-disabled/local-only builds and update
the API contract documentation. A populated pre-v2 backend upgrade and
coordinated backup/restore drill is a separate deployment gate; do not migrate
the user's configured database as part of a test.

Completion means every required collection has its durable sync path,
compatibility/application behavior and actual two-instance evidence, and every
remaining deployment limitation is recorded. No official key ceremony,
production deployment, production migration, real Google callback or OS-platform
credential integration is claimed unless it was separately executed and verified.
