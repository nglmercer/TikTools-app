# LIVE collaboration implementation evidence

Date: 2026-11-08.
Status: persistent identity/policy, transactional workflow commands, recoverable
metadata delivery/audit, workflow snapshot/replay, scoped durable remote
workflow disable/delete, scoped committed points reads/additive adjustments,
points snapshot/replay and experimental HTTPS/WSS host transport.
The native SDK/CLI supports explicit remote profiles and workflow/points recovery.
The isolated desktop controller supports profiles, workflow views and a bounded
live points board/lookup with confirmed additive adjustments.
The native command coordinator uses a preserving version-4 journal for
workflow, points, complete-event operations and disabled proposals, with checked unit labels. Its read-scoped history is verified with live grant changes and
real process termination around send/commit/result persistence. Complete domain UI and real OS-store/platform acceptance are incomplete.
Complete controller and physical two-PC acceptance remain pending.
Local owner settings support TLS configuration, invitations/QR, approvals,
grant editing and scoped revocation. A real browser → local adapter → IPC
host → two TLS SDK clients test passes with isolated local data, separately
from the controller's Wry tests. No real TikTok connection was made.
Local behavior revisions, checked owner editing/conflict review and
exact-definition execution admission are now integrated. Scoped remote automation
status/recovery, the native automation board and durable whole-event disable/delete
are implemented. Author-only disabled definition drafts are available through the
durable API/SDK/CLI. Trusted host provenance keeps delegated definitions out of
execution until explicit owner adoption or a valid closed delegated approval.
Owner saved-source/dependency review and
adoption recovery are now connected to the UI and a host-native journal.
Native remote JSON proposal editing now uses explicit private files and fixed
create/update operations. Owner-only durable core approval is now implemented;
scoped HTTP/audio execution and durable external-effect receipts are now available.
Broader remote activation, plugin/script/emitted-event policies and full product
acceptance remain pending.

## Scoped HTTP/audio and external-effect recovery — latest extension

The closed delegated policy now includes saved `core.fetch`, `audio.play` and
`core.audio.play`, with exact current typed grants and approval/configuration
fences. HTTP pins checked DNS addresses into a private no-proxy/no-redirect
client, bounds request/render/response sizes and returns no response payload in
scoped run output. Audio validates canonical path/default output/volume/overlap
against every subject. Unsupported output is rejected before acceptance.
Rendering checks byte limits before allocating expanded template strings.

A host-native journal commits accepted/dispatched/outcome metadata under final
admission and retains deduplication identities after result expiry. Trusted
native source identity survives reconnect/restart; saved event/action IDs and
per-action occurrence slots survive unrelated reordering. Changed semantics
conflict rather than receiving a new effect ID. Owned blocking tasks keep
SQLite fsync off the LIVE executor; the resumed executor rechecks authority
before the sink call. Cancellation/uncertain outcome/restart never dispatch a
retained identity again. Owner-only API/SDK receipt/history reads expose no
request, URL/file/settings/response content and provide no retry method.

Executed evidence:

- Full core with persistence/native/HTTP/plugin/Cloud features: **647 passed,
  10 ignored subprocess helper entry points**, using `--test-threads=4`.
  Parents invoke real child-process death fixtures, including six new external
  acceptance/dispatch/sink/completion boundaries. Existing approval/adoption,
  points, transport-context, observer and native LIVE regressions also pass.
- New runtime fixtures use an actual local TCP HTTP server and a fake audio
  sink. They prove positive permitted execution, one dispatch per retained
  root, semantic-change conflicts, public input projection, no response-secret
  reflection, cancellation-to-unknown/no replay and revocation after dispatch.
  Separate policy tests cover every-subject canonical audio scope, malformed
  options, private/mapped/mixed DNS addresses and bounded rendering.
- API/SDK/authorization suites: **266 passed** in total. This includes SDK
  **111 library + 8 integration**, collaboration **58**, and control API
  **34 library + 55 integration**. Seven opt-in SDK fixtures remain ignored.
  New receipt methods are denied to remote callers even with all capabilities;
  strict clients reject malformed/cross-ID/oversized/unsorted result data.
- Plugin loader: **66 passed**, including frozen artifacts, independent one-shot
  protocol workers, settings projection, closed Owner push bus and uncertain
  timeout without replay. Core package/settings/lifecycle binding tests pass.
  This is protocol isolation, **not an OS sandbox**; delegated plugin execution
  remains unavailable in the approval policy.
- Frontend: **772 passed / 130 files**, followed by **19 focused** strict-review
  checks. Final `vue-tsc`, full ESLint and Vite build pass. The new Owner effect
  editor preserves old five-field Grant JSON when scopes are empty and uses a
  bounded optional sixth field otherwise, with explicit EN/ES permission UI.
- Seven-package all-target Clippy (`--no-deps -- -D warnings`): passed. Direct
  loader Clippy passes with the four previously recorded Node-API lint classes
  allowed on the command line; the initial strict loader run still reports
  those five existing issues. New fixture/renderer/slot lints were fixed.

Final targeted checks after the allocation/lint refinements passed **4 bounded
render/projection tests** and **5 actual HTTP/audio sink tests**. Featureless-core
`cargo check --no-default-features --tests` passed; its existing empty native-pump
warning remains. These final checks use the same closed runtime policy and keep
Owner behavior unchanged. Final normal Desktop/CLI build passed offline with CLI
`remote-control,local-web`, default Desktop features and the test-only
`TIKTOOLS_CLOUD_ORIGIN` override explicitly unset.

The first core run exposed two incorrect new plugin settings fixtures, which
were corrected to the real settings schema without weakening projection.
A later highly parallel run hit the existing 100 ms points-writer admission
deadline in a fixture that never constructs the effects journal. Inspection
confirmed one durable original award, no accepted command and intact SQLite;
the isolated test and the complete four-thread suite pass. Production admission
limits were unchanged. This is not a high-load latency acceptance result.

Logs are under `/home/meme/.cache/tiktools-test-runs/` with the
`external-effects-` prefix. Earlier sections below retain evidence for previous
stages; their HTTP/audio/recovery gaps are superseded only by this bounded
slice. Plugin/native-input/TTS/graph recovery, scoped emits/scripts/responses,
remaining domains, all-platform/real-keychain and physical two-PC LIVE
acceptance remain open. No real TikTok LIVE, production account or backend
service was used. RemakeBackend runtime/schema and optional relay remain
outside this direct-host implementation.

## Owner approval UI and native recovery — current extension

The Owner Automations page now exposes a separate delegated execution review,
explicit dependency enable-state plan, approval/activation, revocation and
native original-result recovery. Atomic review includes saved source, all
dependencies, current grants, public grant/configuration revision pins and
retained approval status. Its current-state flag validates reverse dependency
relations and the complete stored binding. Changed source/configuration or
permissions require new review; a retained active label is not an execution
permit. Unavailable authors and tombstoned primaries remain reviewable for
revocation.

The new `owner_automation_approvals` journal stores metadata proof and receipts,
not source, grants or credentials. It shares pending admission with adoption.
Both UI flows invalidate consent after recovery, source/plan changes and
uncertainty archival. Browser storage carries no pending IDs/source. Unsafe
source integers are visibly flagged and block approval/adoption consent,
while retaining inspection and approval revocation.

Local Owner responses now have a finite 8 MiB transport allowance independent
of the existing 1 MiB request/256 KiB params limits. A real IPC review above
1 MiB and a later request exercise the path; oversized replies return a
correlated error and oversized notifications request presentation resync.
Remote gateway negotiated limits are unchanged.

Executed evidence so far:

- Core library: **617 passed, 9 private/opt-in helpers ignored**. This includes
  14 real process deaths across activation/revocation before intent, after
  intent, before dispatch, before/after the business commit, after effect and
  after result persistence. Restart/expiry/author revocation/deletion recover
  the original outcome without reinstalling approval or repeating an effect.
  Corrupt proof/ledger/reverse relations, storage failures, cross-journal
  pending state, concurrent Owner submissions and aggregate budgets are covered.
- SDK: **107 library + 8 integration passed, 7 private helpers ignored**.
- API: **33 library + 53 integration passed**, including all-grants remote
  ingress/egress denial and caller ID/source/host forgery rejection.
- Collaboration authorization: **53 passed**.
- Whole frontend: **760 tests across 128 files passed**. Subsequent focused
  approval/adoption/catalog checks after the shared precision guard:
  **60 passed**. Whole-app typecheck, scoped lint and production Vite build pass.
- A strict-host preflight regression caught and fixed structural target spreads:
  both UI review paths now send exactly `{kind,id}`. **55 focused tests** passed
  after that fix, followed by fresh typecheck/lint/Vite build.
- The opt-in `owner_approval_browser` Rust fixture passed against one isolated
  IPC host, a real paired TLS agent and the actual packaged Vue/local browser
  adapter. Its **two Playwright cases** cover English desktop and Spanish
  390-pixel layout, exact saved source/provenance/dependencies, explicit plan,
  actual points execution, history after reload, stale dependency consent,
  revocation (also after author-session revocation), four native receipts and
  zero source/command identities in WebStorage. No generic activate/revoke or
  adoption command is sent by the browser. Both event approvals remain revoked;
  delegated provenance and omitted Owner/alternative dependency states remain
  intact. No TikTok connection was made.
- The 18 local-web tests passed, including >1 MiB bounded Owner responses and
  correlated oversize errors/presentation gaps.
- Final featureless `cargo check --tests` passed with only the previously
  recorded loader and empty native-pump warnings.

Logs are under `/home/meme/.cache/tiktools-test-runs/owner-approval-*`.
The screenshots `owner-approval-en-review.png`, `owner-approval-en-revoked.png`,
`owner-approval-es-review.png` and `owner-approval-es-revoked.png` were inspected.
The seven-package all-target Clippy command (`--no-deps -- -D warnings`) passed.
The final normal Desktop/CLI build passed offline with CLI remote-control and
local-web enabled and the temporary Cloud-origin test override unset. Both
repositories pass `git diff --check`, and all 52 checked local documentation
links resolve. This does not claim dependency-inclusive Clippy or physical
platform acceptance.
Physical two-PC/LIVE, OS credential store/platform,
broader effect policies and external-effect recovery remain open.

## Durable delegated core approval — preceding extension

The implemented owner-only activation/revocation API and Rust SDK bind complete
saved definition closures, final enabled versions, provenance, grant and host
configuration revisions. Approval preserves Delegated authority. Startup loads
validated bounded approval/reverse-dependency state with the trusted definition
snapshot. Changes, deletion/recreation, revoke/reapprove and retained historical
command replay cannot reinstall an old execution permit.

The first policy supports only log, delay and points adjustment/subtraction.
Approved filtering/rendering receives public scalar LIVE fields and empty
globals. Final points mutation shares ordered writer admission and holds all
subject/definition fences through the actual balance/version/outbox commit.
Scoped outcomes stay in presentation channels; separate plugin event lanes
prevent direct notifications, mixed later Owner history or overflow gaps from
invoking arbitrary observer plugins. Ordinary Owner plugin delivery is retained.

Executed checks for this extension:

- Final core library: **601 passed, 8 private/opt-in helpers ignored**. The 31
  added cases cover durable approval transactions/replay/restart, bounded
  storage rollback, complete exact dependency fences, saved versus inline
  bodies, grant/configuration changes, queued revocation, revoke/reapprove
  during a real delay, cross-runtime permits, final rounded viewer deltas,
  ordered point writes and real plugin-observer positive/negative controls.
- Core without default features: **317 passed**.
- Persisted collaboration authorization: **53 passed**.
- SDK: **101 library + 7 integration passed, 7 private helpers ignored**.
- Control API: **28 library + 48 integration passed**.
- Public LIVE throughput integration: **8 passed**.
- HTTPS/WSS gateway regression: **12 passed**, including native/browser
  pairing, permissions, rotation/revocation, bounded frames, durable command
  replay, lost acknowledgements, snapshots/reconnect and TLS reload.
- Seven-package all-target Clippy with `--no-deps -- -D warnings`: passed.
  This does not claim dependency-inclusive Clippy is clean; the previously
  recorded Node-API loader lints remain outside that command.
- Final featureless `cargo check --tests`: passed. Only the pre-existing loader
  warnings and the featureless `start_live_event_pump` warning remain.
- Final normal Desktop/CLI build: passed with the temporary test Cloud-origin
  override unset, including CLI remote-control and local-web features.
- Both repositories pass `git diff --check`; 36 touched local documentation
  links resolve.

Local logs are in `/home/meme/.cache/tiktools-test-runs/approval-full-suite.log`,
`approval-core-final.log`, `approval-featureless.log`, `approval-clippy.log`,
`approval-tls.log` and `approval-normal-build.log`.

The later Owner UI/native coordinator above completes that interface/recovery
slice. Broader script/plugin/HTTP/audio/emit policies, external-effect outcomes, real
two-PC LIVE operation and OS/platform/load acceptance remain open. See
[the approval contract](LIVE-COLLABORATION-APPROVAL.md).

## Runtime authority and private event contexts — preceding foundation

Trusted runtime authority now retains saved principal/session, provenance
generation, resource revision and tombstone state. Coherent index installation
and authority-only invalidation keep queued bodies from outliving their trusted
authority, including disabled, missing and random-unselected dependencies.
Explicit inline Owner work remains distinct from persisted approval.

The private `AutomationEvent` envelope and `ExecutionContext` are neither
serialized nor deserialized. Explicit trusted producers, publication, domain
fan-out, queues, processors, action/transport dispatch, nested emits/plugin
intents and remembered-event storage now preserve or require that context.
Context intersections keep all subject restrictions under one service admission
mutex; equal host labels cannot substitute another mutex. Trusted child depth
cannot reset through JSON source/authority/depth fields.

Current unsupported contexts fail before globals/settings, processors, plugin
observation, rankings, queue slots or effect transports. The globals read itself
requires context. HTTP, media, native/declarative plugins and returned intents
check before dispatch, including after plugin awaits. Owner presentation/replay
getters filter scoped remembered envelopes rather than reclassifying their
payload as Owner input. Shutdown is checked at these boundaries. This plumbing
alone does not approve or activate delegated work; its generic unsupported-effect
guard remains Owner-only. The later approval extension above adds a separate
closed admission path for approved core effects.

Executed coverage:

- Whole core library: **570 passed, 8 private/opt-in helpers ignored**. Sixteen
  new cases cover full trusted provenance, corruption/quarantine, generation
  changes without body changes, conjunctive contexts, cross-service admission,
  revocation, no global reads/effects under scoped context, shutdown, real
  processor positive control, private depth and remembered-event filtering.
- Core public throughput integration: **8 passed**, preserving exact totals,
  bounded cache and restart correctness. Reported rates are not a physical
  two-PC performance acceptance claim.
- Collaboration: **53 passed**, including a new same-host-label/different-lock
  intersection regression and current-subject checks after revocation.
- API: **26 library and 46 integration tests passed**; SDK: **96 library and 6
  metadata/parity integration tests passed**, with seven private helpers ignored
  in the SDK parent suite. Actual TLS draft/recovery and remote denial remain
  covered alongside direct/IPC metadata.
- Seven implementation packages passed all-target Clippy with `--no-deps` and
  warnings denied. No frontend source changed in this extension.
- The actual packaged Vue/Wry → native journal → TLS host fixture passed after
  the context/provenance changes, explicitly invoking the display-dependent
  test. Seven confirmed workflow/points/event/proposal commands and reloads
  retained their outcomes; no behavior executed and the controller created no
  local host business data. This uses private test credentials, not an OS-store
  or physical two-PC acceptance run.
- Featureless core automation: **28 passed**; persisted-authority getters remain
  gated behind persistence, with no invented saved subject or resource revision.
- Final normal Desktop/CLI build passed offline with default Desktop features
  and CLI remote-control/local-web, with `TIKTOOLS_CLOUD_ORIGIN` unset. The
  test-only Cloud-disable override is not embedded in the delivered binaries.

The new [approval/activation design](LIVE-COLLABORATION-APPROVAL.md) is explicitly
pending implementation. It requires the current subject admission lock through
the actual business commit, exact post-enable definition bindings and historical
replay that never reinstalls old approval or state. Persisted approval,
effect-specific final-writer authorization, plugin/settings bindings, external
effect outcomes, remaining domains and physical/platform acceptance stay open.
Local logs use `/home/meme/.cache/tiktools-test-runs/execution-context-*`.

## Persisted execution subjects and owner manifest preview — current extension

The host now resolves saved delegated principal/session subjects independently
of short-lived request bearer tokens. Non-deserializable single/batch handles
check current persisted grants, exact identity, revocation, enabled host and
absolute/idle expiry without touching activity. A batch holds one DB mutex
through synchronous admission; the callback never implies later async effect
completion. Empty all-Owner subjects remain an empty set, not remote authority.

The owner-only `automation.manifest.get` read uses the atomic saved-source
review and returns exact definition/version/provenance/content-digest bindings,
all delegated subjects' current grant revisions and closed effect/render-field
classifications. Disabled, missing, tombstoned and random-unselected references
remain included. It emits no source/config/script/settings values and always
reports `executionAuthorized:false`. API and SDK validate the closed contract;
remote sessions cannot use the preview. No manifest approval, activation,
execution-context propagation or final-effect policy is claimed by this work.

The manifest's render inventory follows the existing renderer's legacy prefix
and empty-segment behavior instead of assuming a different path. Audio target
classification follows actual fileRef/file/filePath/path precedence. Plugin
descriptor/permission/settings bindings and unsupported effects are explicit;
global/secret access is unsupported until a real grant policy exists, rather
than becoming permitted through an Owner review.

Executed coverage includes:

- Collaboration: **52 passed**. Nine new cases cover bearer expiry/rotation,
  socket independence, no activity touch, restart/current grants, exact and
  cross-host bindings, all-or-nothing batches, expiry/disable/revoke, bounded and
  corrupt state, writer ordering and rollback.
- Whole core library: **554 passed, 8 private/opt-in helpers ignored**. The 14
  new builder/core cases cover deterministic complete bindings, multiple
  subjects/tombstones, source exclusion, current grant/revocation and restart,
  missing primary, corruption/budgets, render fidelity and unchanged quarantine.
- API: **26 library and 46 integration tests passed**, including strict owner
  preview parameters/metadata, complete version bindings, no mutation/source
  output and all-capability remote ingress/egress denial.
- SDK: **96 library and 6 metadata/parity integration tests passed**, with seven
  private helpers ignored in the parent suite. This includes direct/IPC preview
  validation and actual TLS denial for both action/event manifest requests in
  the draft/provenance recovery fixture. No submitted behavior was executed.
- Seven implementation packages passed all-target Clippy with `--no-deps` and
  warnings denied. All 10 pure manifest tests passed again after the equivalent
  canonical-key sorting cleanup requested by Clippy.
- Featureless core automation: **28 passed**. The new manifest builder remains
  persistence-gated; the public read reports the unavailable capability in that
  build rather than inventing persisted revisions or saved author authority.
- Normal Desktop/CLI build passed offline with default Desktop features and
  CLI remote-control/local-web. `TIKTOOLS_CLOUD_ORIGIN` was unset for this final
  build; the test-only Cloud-disable override is not embedded in the binaries.

Logs are retained locally at
`/home/meme/.cache/tiktools-test-runs/execution-manifest-*` and
`execution-subjects-collaboration.log`. These are private verification artifacts,
not application state or evidence of a real LIVE.

The remaining full-plan gates include persisted approval/invalidation, trusted
plugin/catalog/settings bindings, context propagation through nested effects,
filtered globals and final sinks, external-effect outcomes, other domains and
physical two-PC/LIVE plus OS-keychain/platform/load acceptance. These additions
require no RemakeBackend runtime endpoint or migration.

## Native private proposals — current extension

The isolated native controller has a separate action/event JSON proposal editor.
It confirms full-text local retention, inspection-based base changes and sending
the exact saved copy. A fixed native bridge chooses method, host precondition
and durable ID; browser code cannot provide a replacement body during submit.
The version-4 metadata journal shares one pending gate across domains. Formats
1–3 migrate without changing original IDs, points results or history, and cannot
smuggle proposal fields or operations. No body/hash enters command metadata.

New checks cover both families with equal raw IDs, create/update receipts,
incomplete local JSON, changed saved copies, forged dispatch fields, inspection
replacement, narrowed/restored/read-only grants, delayed private reads after
revocation, and original-result recovery after later owner edits. Private file
checks exercise binding, CAS, count/UTF-8/file bounds, unsafe links/permissions
and actual process death before/after atomic replacement. Staging text is never
promoted by a read.

The packaged Vue → Wry → native journal → actual TLS host fixture passed with
seven confirmed commands: workflow stop/delete, points adjustment, event
stop/delete and action proposal create/update. Reloads retain private text and
native history; the final proposal remains disabled with delegated authority.
No automation ran and the controller created no host business data directories.
Credentials are a test memory store, so this does not establish OS-keychain or
physical two-PC acceptance.

Chromium consent/access-loss checks passed in English at desktop width and
Spanish at 390 px. Both screenshots were inspected; there was no horizontal
overflow or page error, and source did not enter local/session storage. These
layout fixtures complement the real native/TLS fixture; they do not stand in
for it. Full delegated manifests, execution contexts, effect policies, other
domains and the release matrix remain unfinished.

Executed regression results for this extension:

- SDK/native library: **94 passed, 7 private/opt-in helpers ignored**. The parent
  tests invoke their subprocess helpers where applicable, including proposal
  private-file replacement deaths. The new blocked-I/O grant test passed.
- Frontend: **732 passed across 126 files**; TypeScript and focused ESLint passed.
- Chromium proposal consent/access loss: **2 passed**, English desktop and
  Spanish 390 px; screenshots inspected.
- Real packaged Wry/native-journal/TLS integration: **1 passed**, explicitly
  invoking the display-dependent fixture. Its seven host outcomes include both
  native action proposal operations and later reload/history.
- Seven implementation crates: all-target Clippy passed with `--no-deps` and
  warnings denied. The unchanged dependency-inclusive Node-API loader limitation
  described below remains separate.
- Normal Desktop/CLI build passed offline with default Desktop features and
  CLI remote-control/local-web, with `TIKTOOLS_CLOUD_ORIGIN` unset. The test-only
  Cloud-disable override is not embedded in these final binaries. The updated
  Vite production bundle also built successfully.

Logs for the SDK, frontend, Clippy and normal build are under
`/home/meme/.cache/tiktools-test-runs/native-proposal-*` in this local workspace;
they are evidence artifacts, not application state.

## Owner source review and native adoption recovery — current extension

The owner Automations page reads saved source and trusted provenance atomically,
including all action references and their versions. It keeps the unsaved editor
candidate separate. Adoption submits exact captured host identity and the entire
reviewed version list; the writer checks dependency changes in the authority
transaction. Generic owner adoption retains its earlier primary-only behavior
when the optional list is omitted.

The host-native coordinator persists a random intent ID and private semantic
binding before dispatch. One pending intent blocks new coordinator submissions
across owner windows. Reload/restart resolution only queries the original
ledger result; explicit unknown review performs a final lookup and never
repeats or undoes adoption. Definition source and command identities never enter
browser storage. A transfer stays disabled for automatic operation, with owner
fire/test and a separate enable retaining their normal contracts.

Executed checks include:

- Core automation regression: 56 passed, including the nine coordinator cases
  and actual process termination at all five intent/dispatch/result boundaries;
  one private helper ignored in the parent harness.
- Behavior DB regression: 34 passed, three subprocess helpers ignored in their
  parent harness; includes coherent review, budgets, corruption-local repair,
  dependency-fenced adoption and the new draft shape matrix.
- API: 16 passed across provenance/review, legacy adoption/version compatibility
  and native owner coordinator tests. All remote grants remain denied access to
  the four native methods, and caller metadata/forged IDs are rejected.
- SDK: 83 library tests and five metadata/parity integration tests passed,
  including direct/IPC typed review, complete-envelope validation and TLS denial;
  six private process helpers ignored in the parent.
- Frontend: 720 passed across 124 files after the projection-race fix; TypeScript and focused ESLint passed.
  Real Chromium component checks at 390×844 in English/Spanish showed no page
  errors or horizontal overflow. Source/consent/recovery screenshots were
  inspected. These component layout checks use fixture APIs and are separate
  from the real owner adapter integration.

The final whole-core run passed 540 library tests and eight throughput
integration tests. Eight child/opt-in stress helpers were ignored in the parent
harness. Featureless-core automation passed 28 tests. All-target Clippy for the
seven implementation packages passed with `--no-deps` and warnings denied.
Journal history now follows durable insertion order; a deterministic clock-
rollback regression proves a new completed ID cannot be pruned by future-dated
older rows and existing audit timestamps are not rewritten.

The actual owner-browser → packaged local adapter → IPC → isolated Core test
passed in English desktop and Spanish 390 px against real TLS-authored drafts.
It freezes full source/trusted metadata, updates a disabled dependency through
another owner connection, requires explicit reread/consent, then adopts the
event and same-ID action separately. Unsaved editor text survives adoption;
native history survives page reload, no source/IDs enter WebStorage, no enable/
fire/LIVE effects occur and four exact host-generated receipts remain. Real
desktop/narrow source, dependency and history screenshots were inspected.

The initial browser run exposed a real projection race: an older library
revision invalidated a newer atomic review. The fix ignores older projected
versions and tracks when each reviewed identity has caught up before treating
its disappearance as deletion. Explicit rereview resets that tracking. Four
new model/component regressions and the complete real-browser rerun passed;
preflight full-source comparison and atomic server dependency fences remain.

The API library's 26 cases passed after the new routes. The actual packaged
Wry/native-journal/TLS-controller regression also passed with the final frontend:
workflow draft restore and stop/delete, points adjustment and event stop/delete
survive reload without creating client host-data directories. This remains
separate from physical two-PC/LIVE and real OS credential-store acceptance.
The final normal Desktop/CLI build passed offline with default Desktop features,
CLI remote-control/local-web and the test-only Cloud-origin override unset.

New fresh remote-draft structure checks prevent missing type/trigger or malformed
known editor fields from poisoning the owner projection. Defaults are applied
only after current authorization, ownership and original-ID replay in the same
writer transaction. Invalid writes leave body, provenance, versions, receipts
and outboxes unchanged; owner legacy data and historical receipts are preserved.
This validates structure and retains quarantine; it does not approve effects.

## Trusted automation provenance and disabled drafts — prior extension

Host-owned SQLite provenance now distinguishes owner definitions from delegated
ones independently of editable JSON. A one-time migration seeds legacy records;
later missing/corrupt metadata cannot restore owner authority. Definition writes,
versions, provenance generations, receipts and outboxes share the writer
transaction. Delegated identity remains reserved across delete/recreate and
restart. Ordinary owner edits, enablement and bulk writes retain that identity.

Remote `automation.create/update` require explicit `enabled:false`, current read
and edit permissions, qualified family scope and complete durable metadata.
Only the original principal may update its draft. Owner/foreign records and
retained identities are denied. Owner/foreign event references also reserve
action IDs, including disabled events and unresolved dependencies. Dependency
inspection is bounded and fails closed. Results contain only
`{kind,id,enabled:false,pendingReview:true}`; status reads retain the existing
closed five-field summary and never expose the submitted definition.

The runtime quarantines delegated definitions across matching, manual/test/inline
execution and hotkey registration, including every referenced action before
random selection or enabled-step filtering. Authority-load failures quarantine
the runtime. Owner-only `automation.authority.get` exposes trusted provenance;
`automation.adopt` requires explicit acknowledgement and a durable CAS command.
Transferring delegated work disables it atomically. A separate owner enable is
required for automatic operation. Explicit owner fire/test retains its usual
rules after authority transfer. This transfers authority to the owner; it does not
implement execution under the moderator's grant.

The final full core/API test run passed: 519 core library tests, eight core throughput
integration tests and 60 API library/integration tests, including three new
provenance API scenarios. The core parent harness leaves seven subprocess or
opt-in stress fixtures ignored. Runtime cases exercise real points effects (10 → 8)
only after adoption and a separate enable, retained tombstones, forged JSON,
same-ID event/action separation, failed metadata loads and restart. Database
tests include process death at transaction boundaries, one-time migration,
authority retention, dependency reservations and bounded malformed-data denial.
The API tests also verify immutable original receipts after owner adoption and
scope-narrowed egress.

The trusted loader checks that live definitions,
retained versions and every authority sidecar form a consistent catalog.
Missing versions, deleted/live mismatches, orphan authority and missing tombstone
authority cannot disappear behind an inner join and acquire inline owner
authority. Version seeding now shares the one-time provenance migration, so
restart cannot reset missing retained history to revision one. Ordinary owner
repair reads and persisted plugin activation remain readable independently of
the trusted runtime loader; corrupt authority still prevents runtime admission.

The shared SDK/collaboration/gateway run passed with serialized harnesses:
78 SDK library tests (six private subprocess helpers ignored in the parent),
four SDK metadata/parity integration tests, 43 collaboration library tests, two
collaboration integration tests and 12 actual TLS gateway tests. The new TLS
draft scenario covers action/event families, author isolation, forged JSON,
minimal results, fresh-connection original-outcome recovery, adoption, historical
replay, narrowed grants and revocation. A legitimate revision-zero no-op command
receipt is accepted without permitting zero-revision live-row invalidations.

The first SDK run exposed invalid dotted IDs in the new denial fixture; those
IDs were corrected to valid command IDs so the test reaches authorization. An
existing migration test also encountered `command_journal_in_use` during the
parallel run. The complete serialized rerun passed, including that migration
test; production lease checks were not weakened. Frontend label/status changes
passed 70 focused tests across 12 files, TypeScript, targeted ESLint and a new
production bundle. Generated contracts also passed their check.

An intermediate full core run passed all new automation cases but exposed a retained
points-replay race while an award writer was active: replay requested an
unnecessary immediate SQLite writer transaction with a 100 ms deadline. Replay
now uses a coherent read transaction, preserving authorization, the original
receipt and cache state without requesting writer admission. The points fixture
also precreates all three business databases to prevent legacy data import.
The deterministic replay regression passed while a separate `BEGIN IMMEDIATE`
transaction held an uncommitted balance of 99. Replay returned the exact original
2-point receipt; cache, events, committed balance, revision and the single
ledger/history entry remained unchanged after rollback. The original
process-death points case also passed in its isolated rerun.

The final whole-core/API rerun includes both points recovery tests and all
catalog-integrity regressions. Featureless-core automation compatibility passed
28 tests. All-target Clippy passed for the seven implementation crates with
`--no-deps` and warnings denied, after naming the trusted runtime snapshot type.
Dependency-wide loader lint limitations remain recorded below.
The packaged Vue/Wry/native-journal/TLS-owner regression also passed again with
the final provenance and read-only points replay implementation: local workflow
draft restore, workflow disable/delete, points adjustment and complete-event
disable/delete survive page reloads, with five confirmed journal records and no
client host-data directories. Its credentials remain memory-only fixtures.
The final normal Desktop/CLI build passed offline with CLI remote-control and
local-web enabled, default Desktop features and `TIKTOOLS_CLOUD_ORIGIN` unset;
the test-only Cloud-disable override is not embedded in the delivered binaries.

This stage adds no RemakeBackend runtime route or migration. Execution manifests,
approval invalidation, saved-principal grant lookup, filtered template globals,
authority propagation through nested emits/plugin intents and authorization at
each final effect remain required. Current grants do not express arbitrary HTTP,
audio, emitted topics or secret-global access. The private JSON proposal UI is
covered by the current extension above. Physical two-PC/LIVE, OS-store,
platform and load acceptance remain open.

## Native automation board and event command recovery — prior extension

The isolated controller now shows only closed automation status, keyed by
family/raw ID, and confirms whole-event disable/delete through native intent
storage. Action steps remain read-only. Definitions never enter the owner editor
or draft store. Automation pages and confirmation use their own display revision;
points refreshes preserve it. Connection/profile/authority loss hides rows and
closes captured-label dialogs; same-grant event changes require explicit review.

The native v3 journal preserves pending/history from both older versions and
rejects new operations smuggled into an old file. Event, workflow and points
commands share one unresolved-intent gate. Fixed event scope, connection, host
instance and grant revision are checked before and after journal sync. Native
receipt validation checks family/ID/revision and a closed minimal body. Recovery
queries the original ID without sending a mutation again. Stream rebuild also
handles newly granted readable domains while existing domains remain available.

| Check | Result |
| --- | --- |
| Full frontend suite | 696 passed across 122 files |
| TypeScript, targeted ESLint, Vite production bundle | Passed |
| Chromium automation board and confirmations | 2 passed, English and Spanish including 390px; captures exact revisions, preserves uncertain outcomes without resend and keeps actions read-only |
| Visual QA | Desktop/narrow screenshots inspected; labels and confirmation controls fit |
| Native SDK library | 77 passed, 6 private subprocess helpers ignored in the parent harness; metadata/parity integration added 4 passes |
| Native controller integration | Two independent TLS controllers, family-scoped rows, exact stop/delete receipts, hidden payloads, pagination and points-independent confirmations; grant addition/removal, scope recovery, pending cross-domain blocking and profile switch during journal sync |
| Actual process termination | Both event operations × five coordinator boundaries: before intent, after synced intent, before send, after host reply, after recorded result. Unsent IDs are never blindly replayed; committed revision2 receipts recover after owner changes to revision3 and removal of write access |

The actual packaged Wry/native-journal/TLS fixture passed: workflow draft save/
restore, workflow disable/reload/delete, points adjustment/reload, then event
disable/reload/delete. Five confirmed records remained, event tombstone3 and
same-ID action revision1; no automation ran and no host-data directories appeared
in the client home. Memory-only fixture credentials do not prove OS-store
acceptance. The final native projection/authority regression passed all seven
cases after the byte-budget and permanent-failure clearing refinements.
All-target Clippy for the seven affected crates passed with `--no-deps` and
warnings denied. The final normal Desktop/CLI build passed offline with CLI
remote-control/local-web and the test-only Cloud-origin override unset. Physical two-PC/LIVE, real OS
store, Windows/macOS, delegated definitions/activation and the other full-plan
requirements remain open. The native board does not close release acceptance.

## Scoped automation status and event deactivation — prior extension

The app now exposes closed remote `automation.list/get` summaries and durable
whole-event disable/delete. Owner single-record behavior CRUD/toggles use the
same command primitives. Qualified automation grants distinguish event/action
families, including the same raw ID. Automation stays unavailable under a LIVE
creator restriction. This adds no RemakeBackend runtime route or migration.

New evidence exercises:

- Status SQL does not select arbitrary payload JSON. Malformed oversized payloads
  do not leak config/scripts/dependency IDs; names, byte budgets, empty/legacy
  grants and matching family versions are checked.
- CAS/retained receipts, explicit durable create IDs, shared workflow command
  namespace, owner action-delete cascade versions and original-ID replay after
  subsequent owner changes. Missing durable targets preserve automation errors.
- Real process death before/after the behavior transaction commit leaves
  definition/reference/version changes, durable result and outboxes atomic.
- Whole-event disable/delete cancels pending effects after an actual runtime
  delay. Owner reenable/recreation followed by original-ID replay does not revive
  the old event generation. Revocation while SQLite writer admission is blocked
  denies the command without recording it.
- Permission-filtered immutable automation pages, identical IDs in two families,
  late owner invalidations, automation-only command subscriptions and new
  baselines after grant narrowing. An attempted second principal on the same
  synthetic connection was correctly denied; the fixture now assigns independent
  connection IDs, matching real gateway behavior.
- SDK rejects malformed/unredacted summaries, wrong family scopes, duplicate
  cross-page keys and inconsistent command targets. Two actual TLS clients get
  scoped snapshots/invalidations; only the caller gets its command receipt. A
  fresh TLS connection resolves the original result after a later owner change.
  All-capability action-step mutations remain denied; revocation fences calls.
- Owner grant UI explicitly labels automation/points capabilities and requires
  `event:ID` / `action:ID` restrictions. Browser tests exercise qualified grants,
  grant editing and independent session revocation in English, plus narrow
  Spanish setup. This is owner administration UI, not a remote automation board.

| Check | Result |
| --- | --- |
| Full frontend suite | 673 passed across 120 files |
| TypeScript and targeted ESLint | Passed |
| Packaged Vue frontend | Passed; grant examples use literal `event:ID` / `action:ID` without HTML-message warnings |
| Chromium owner grant/setup flows | 2 passed: qualified event/action permissions, grant editing and revocation in English; Spanish setup at 390 × 844 |
| Desktop/narrow visual QA | Both screenshots inspected; fields/controls remain usable and no horizontal page overflow observed |
| SDK with remote profiles | 63 passed, 5 private subprocess fixtures ignored; includes actual two-client TLS behavior summaries/deactivation/original-outcome recovery and existing native workflow/points regressions |
| Full core with persistence/HTTP/native/Cloud/plugin features | 495 passed, 6 subprocess fixtures ignored |
| Automation API integration | 4 passed, including durable owner cascade receipts, scoped event stop/delete, original outcomes, narrowed-grant egress and domain-correct missing-target errors |
| Shared collaboration library | 43 passed, including family-qualified grants |
| All-target Clippy for the seven affected crates, `--no-deps` | Passed with warnings denied, normal Desktop/CLI features and no Cloud-origin test override |
| Final normal Desktop/CLI build | Passed offline with CLI remote-control/local-web and default Desktop features; test-only Cloud-origin override unset |
| Revocation after SQLite admission, final targeted check | Passed for fresh and retained-result transactions after capturing a valid identity; direct transaction invocation prevents an earlier core check from masking removal of the post-admission reauthorization |
| Featureless-core automation compatibility | 28 passed; local in-memory owner edits remain available and persisted/remote commands remain unavailable |
| Final gateway with isolated business databases | 12 passed, including small-frame snapshots, reconnect/replay, points lost acknowledgement, revocation, TLS reload and local-only startup |
| Actual owner browser → local adapter → IPC → two TLS clients | Passed with the final packaged frontend: the owner selected qualified event/action grants, approved independent clients, one durably disabled the event and queried its receipt, the other read the new status; summaries omitted hidden labels/config; independent session revocation and listener disable preserved the host |

Two gateway assertions initially omitted `automation` from the expected hello
domain list. Both assertions now include the advertised domain. The corrected gateway run passed all 12 cases. Its private-home helper now
precreates all business databases so legacy `./data` import cannot contaminate
transport fixtures; the final rerun includes that isolation change. The actual
owner UI/TLS integration also passed with qualified automation grants. Final
static analysis and normal Desktop/CLI build passed as recorded above.

The initial dependency-inclusive Clippy check stopped on five existing Node-API
loader lints (alignment modulo, a platform cast, checked unwrap and borrowed
closures) outside this extension. Those loader implementation files are unchanged.
The final check targets the seven affected crates with `--no-deps`; dependency
lint cleanup is recorded separately from functional/build results.

At that stage the native board/journal upgrade was pending; the newer extension
above implements it. Remote definitions/activation and delegated manifests/authority, physical two-PC/LIVE, real OS keychain and supported
platform/load acceptance remain open. No release acceptance row is closed by
these isolated local checks.

## Checked owner automation editor — prior extension

The behavior editor now captures an immutable opening body/revision and keeps
its candidate separate from current host snapshots. Writes carry the captured
family-scoped revision. Background refreshes do not replace drafts or advance
their bases. Save/delete failures retain text and dirty state; unconfirmed
outcomes require an explicit current read/review before another submission.
An absent update never becomes an automatic create. Reviewed tombstone
recreation and compare/rebase are separate from the subsequent Save action.

Successful acknowledgements cannot overwrite newer typing or change a different
selection, including a same-ID reopen. Current reads are also fenced by editor
lifecycle. Library toggles confirm a frozen ID/revision, restore committed
checkboxes on cancel, and require rereading after a conflict. Profile adoption
uses the originating profile and distinct action/event membership for equal IDs.
Snapshot bodies/revisions publish atomically to synchronous observers. Only an
explicitly non-persistent local owner host uses unchecked in-memory edits,
with a visible run-only notice; missing metadata otherwise never downgrades CAS.

| Check | Result |
| --- | --- |
| Final complete Vitest frontend suite | 672 passed across 119 files, including owner editor/data/async lifecycle, profile adoption, controller and existing plugin/owner UI regressions |
| Actual Chromium owner comparison/recreation | 2 passed, English desktop and Spanish at 390 × 844; actual WebView RPC wire fixture exercises family-separated versions, stale edits, explicit local rebase, deleted-ID tombstones and separate recreation without firing/enabling actions |
| Narrow review visual QA | Spanish comparison dialog inspected; footer actions retain full labels, no horizontal dialog overflow or truncated button labels. The initial clipped footer was corrected to stack its actions on narrow screens |
| First actual owner browser/adapter/IPC host run | Passed: two real packaged editors opened revision 1 on one isolated headless host, one saved revision 2, the stale editor retained its draft and explicitly reviewed before revision 3; external deletion produced tombstone 4 and separately confirmed recreation produced revision 5. The same raw action ID stayed at revision 1 and the host recorded zero automation runs |
| Final TypeScript, targeted ESLint, packaged frontend build | Passed; no Rust production/API change was required for the owner editor |
| All-target Clippy over core/collaboration/API/SDK/gateway/CLI/Desktop | Passed with full core/remote features and warnings denied, including the new actual-owner integration fixture |
| Final combined actual-owner integration fixtures | 2 passed with the final packaged frontend: two concurrent owner editors over the real adapter/IPC host, plus owner settings controlling two real native TLS sessions with independent revocation and listener disable. Both ignored-by-default browser fixtures were explicitly invoked |
| Final native controller Wry regression with the updated bundle | Passed: private draft save/restore and workflow/points commands/reloads against the TLS fixture, with controller isolation assertions retained. The ignored-by-default display fixture was explicitly invoked |
| Normal Desktop/CLI build | Passed offline with CLI remote-control/local-web enabled and no test-only Cloud-origin override |

No real TikTok session, production Cloud
request, OS credential-store validation or physical two-PC acceptance is implied.
At that step remote behavior reads/commands were unfinished; the newer extension
above adds scoped status and whole-event stop/delete. Native automation UI,
manifests/owner approval and delegated effect propagation remain unfinished. See the
[automation implementation and owner contract](LIVE-COLLABORATION-AUTOMATION.md).

Focused reproduction (use a private short `TMPDIR` on a disk with free space):

```bash
bun x vitest run --maxWorkers=2
bun x playwright test tests/e2e/automation-editor.spec.ts
bun scripts/task.ts frontend-build
TIKTOOLS_CLOUD_ORIGIN= cargo test -p tiktools-cli --features remote-control,local-web --test collaboration_owner_browser --locked --offline -- --ignored --test-threads=1
```

The last command requires Bun and Playwright Chromium. It runs against private
fixture homes and the actual packaged frontend. Rebuild normal Desktop/CLI
without the test-only Cloud origin override after native tests.

## Executable behavior consistency and admission — preceding extension

The local action/event writer path now has family-scoped revisions/tombstones,
atomic CAS and outbox reservations, cross-family bulk enablement and action
reference cleanup. The owner API exposes metadata while keeping result shapes.
One atomic runtime index and host-generated definition tokens stop stale queued
steps after definition/dependency changes, including identical delete/recreate.
Configuration writes use a dedicated epoch/notification barrier; waiting
admission yields, audit delivery stays independent, and shutdown wakes waiters.
Exhausted or panicked definition writers reject subsequent admission/mutations.

At this preceding step, remote automation APIs and workflow save/enable were denied.
Durable behavior commands, scoped remote reads/recovery, trusted provenance, manifests/approval,
per-effect delegated authority and owner-editor base/conflict integration are
unfinished; the newer evidence above completes the owner editor and the scoped
status/whole-event deactivation stage. Definition editing/activation stays denied. Stored workflow graphs still have no connected interpreter.
See [the automation specification](LIVE-COLLABORATION-AUTOMATION.md).

The earlier full-core run
had 488 passing tests, 5 ignored fixtures and one shutdown assertion failure.
The failed case passed in isolation. Its fake-host helper shared one
process-global business database across independent hosts, allowing checkpoint
ownership to be superseded. The helper now uses private per-host databases;
an added two-host shutdown regression exercises that isolation. Production
checkpoint fencing and shutdown success requirements were retained.

A subsequent parallel run passed the corrected shutdown cases but failed a
points durability-gap fixture: diagnostics showed that the writer had failed
to open, so both submitted operations failed rather than exercising the intended
failed-then-successful write sequence. The case passed in isolation. That
fixture now establishes WAL before its worker and test inject competing
schema/lock operations, and precreates private empty databases rather than
importing legacy checkout data. The SDK and packaged native fixture also
precreate their private databases. Production startup/failure policy was not
relaxed. The final default-parallel full-core run passed with those fixture
corrections.

| Check | Result |
| --- | --- |
| Final full core with persistence/native TikTok/HTTP/plugin install/Cloud features | 490 passed; 5 ignored: 4 subprocess helpers invoked by their parent crash tests and 1 opt-in Linux RSS profiling fixture. No actual LIVE connection |
| Behavior storage cases within that run | 6 passed, including same-ID family scopes, concurrent CAS, canonical disabled-reference cascade/all affected response versions, cross-family rollback, revision exhaustion, actual before/after-commit process death and non-destructive repeated migration |
| Actual execution admission cases within that run | 5 passed: delayed pending point effects invalidated by edits/toggles/ABA, unrelated edits preserved, writer wait yields, audit lock independence, shutdown wake, poisoned/exhausted writer rejection and metadata-only audit delivery |
| Shared collaboration/API/SDK/gateway regression suite | 175 passed; 5 subprocess helpers ignored in parent harnesses. Includes SDK behavior CAS/error metadata over direct and actual IPC. This preceded the final all-affected delete-response version assertion |
| Owner automation plus controller frontend regressions | 50 passed across 8 test files; owner edit/conflict completion is still pending |
| Final featureless-core automation regression | 28 passed with `--no-default-features`; persisted preconditions remain unavailable and remote mode is not enabled |
| Final automation API regression after cascade metadata refinement | 2 passed, including primary action and dependent event versions in the same delete response, stale recreation, atomic bulk, schema metadata and remote denial |
| Final SDK metadata regression | 1 integration test passed over both direct and actual local IPC, including workflow/points durable commands and behavior CAS/conflict/tombstone envelopes |
| Final all-target Clippy across core/collaboration/control API/SDK/gateway/CLI/Desktop | Passed with full core/remote features, `--no-deps -- -D warnings` |
| Final packaged Vue/Wry controller with TLS host | Passed with empty private fixture databases: confirmed private draft save/restore, workflow disable/reload/delete, additive point adjustment/reload/history and retained draft discard; one ignored-by-default display fixture invoked explicitly |
| Final normal Desktop and CLI build | Passed offline with CLI remote-control/local-web enabled. The shell had no `TIKTOOLS_CLOUD_ORIGIN` override; normal artifacts were rebuilt after all isolated tests |
| Documentation local links and whitespace | 7 relevant documents have no missing local link targets; `git diff --check` passed |

These are local synthetic regressions; they do not close delegated execution, external-effect
recovery, physical two-PC/LIVE or platform/OS-store acceptance.

Core/API/SDK reproduction (set `TMPDIR` to an existing private directory on a
disk with free space; the full-core run above used the default test parallelism):

```bash
TIKTOOLS_CLOUD_ORIGIN= cargo test -p tiktools-core --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --lib
TIKTOOLS_CLOUD_ORIGIN= cargo test -p tiktools-core --no-default-features --locked --offline --lib automation -- --test-threads=1
TIKTOOLS_CLOUD_ORIGIN= cargo test -p tiktools-control-api --test automation_versions --locked --offline
TIKTOOLS_CLOUD_ORIGIN= cargo test -p tiktools-client --features remote-profiles --test metadata --locked --offline
```

The packaged fixture uses the exact ignored Wry command documented in the draft
section below. After tests, unset the test-only Cloud origin override and rebuild
the normal artifacts:

```bash
cargo build -p tiktools-desktop -p tiktools-cli --bin tiktools-desktop --bin tiktools --features tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline
```

## Explicit local drafts and conflict comparison — preceding extension

The native controller now provides scoped private draft storage with explicit
full-text/sensitive-value consent, local store revision checks, bounded metadata
pages and selected-text reads. Saved copies survive page/controller reopen;
unsaved edits are never persisted automatically. Host edits retain the draft's
base and text while exposing the current authorized host version for comparison.
Choosing a local merged base and saving that text are separate explicit actions.
No draft operation dispatches a host mutation or creates a Cloud Sync record.

| Check | Result |
| --- | --- |
| Full SDK/client suite with `remote-profiles` | 60 unit tests plus 4 integration/parity tests passed; 5 subprocess-only fixtures ignored by the parent and invoked explicitly by their respective parent tests. This preceded the final bounded staging-file refinement |
| Final focused native/storage draft suite | 6 passed plus 1 child helper ignored in the parent; tests include profile reopen, exact read-grant narrowing/restoration, delayed-read epoch fencing, private-file/link checks, UTF-8/file/count limits and no silent eviction |
| Actual draft crash children | Parent terminated children before replacement (including a repeat), after rename and after directory sync. A complete old/new saved copy survived; repeated crashes left only one bounded staging file, and explicit removal cleaned both copies |
| Profile forget cleanup | A rejected unsafe draft-file cleanup leaves profile metadata available for retry; a subsequent private-file cleanup removes the saved text. Pending-command/active-profile protections remain in place |
| Final controller model/component suite | 25 passed, including restore, host comparison/local rebase, frozen consent target, late saved-copy reads, lost-save ACK with explicit review and no retry, grant hiding, and a discard awaiting I/O during host switching |
| Chromium controller workflow/points/draft regressions | 6 passed across English and Spanish, including draft cancel/save/reload/compare/rebase/removal; Spanish draft dialog visually inspected at 390 px, with no horizontal overflow or page errors |
| TypeScript, targeted ESLint and packaged frontend build | Passed; final guard checks/build are recorded by their current logs |
| Final packaged Vue/Wry/native private-file/TLS host fixture | Passed: explicit sensitive-text confirmation, saved draft restored after reload, workflow disable/reload/delete, point adjustment/reload/history, retained-draft discard, final local store revision 2 with no saved record, four page generations and no controller host-data directories |
| Final Clippy, all targets across core/collaboration/control API/SDK/gateway/CLI/Desktop | Passed with full core/remote features and `-D warnings`, after the bounded staging-file refinement |
| Full Desktop binary unit suite | 103 passed; 2 display-dependent Wry fixtures ignored in the ordinary harness and the extended command/draft fixture invoked separately |
| Final normal Desktop and CLI build | Passed offline with CLI remote-control/local-web enabled and no test-only Cloud-origin override; normal shell confirmed the override was absent |

The native workflow/points Wry fixture saves and restores a draft through actual
Vue/native bridge/private-file I/O before its existing TLS host commands, then
discards the retained draft after host deletion. These are synthetic/private
Linux fixtures with injected memory credentials; physical two-PC/LIVE, real OS
store and cross-platform/Windows ACL acceptance remain open.

Focused reproduction commands (use an existing short private temporary directory
on a disk with enough free space):

```bash
TIKTOOLS_CLOUD_ORIGIN= cargo test -p tiktools-client --features remote-profiles --locked --offline --lib drafts::tests:: -- --test-threads=1
bun x vitest run src/web/features/controller
bun x playwright test tests/e2e/controller-drafts.spec.ts tests/e2e/controller-commands.spec.ts tests/e2e/controller-points.spec.ts
```

The separately invoked ignored Wry fixture requires a Linux display and a fresh
`bun scripts/task.ts frontend-build`. Its exact test name remains
`controller::smoke::commands::packaged_controller_confirms_tls_commands_and_keeps_history_after_reload`.
Blank `TIKTOOLS_CLOUD_ORIGIN` isolates these fixtures. Rebuild normal Desktop/CLI
artifacts with that variable unset afterward; it is also a tracked compile-time
origin setting, not only a runtime test switch.

## Bounded points display and confirmation — preceding extension

The latest points slice adds transactional minimal `points.view` reads,
optional explicit `pointsLimit` subscription projections, independent native
points display revisions, grouped refreshes, exact viewer lookup and an
English/Spanish confirmation form. The connection ID, host identity, grant and
unit are fenced; balance-only changes preserve an additive confirmation.
Display invalidations are acknowledged after staging a dirty bit and never
replay an award. Native recovery fetches absolute display state before readiness.

Executed evidence for this extension (the older sections below retain their
earlier-stage counts and limitations):

| Check | Result |
| --- | --- |
| Shared collaboration/API/SDK/gateway suites, offline, SDK `remote-profiles` enabled | 166 passed before the final additional SDK projection test; covers new API policy/egress checks, projection mode echo/bounds, both native points scenarios, and workflow regressions |
| Final SDK remote transport/recovery suite | 9 passed after the complete/top-N/notifications-only TLS baseline test and stricter body/version pairing; the earlier shared count overlaps this suite |
| Core `points::` suite with persistence/native-TikTok/HTTP/plugin-install/Cloud features | 48 passed; one child crash fixture ignored in the parent and explicitly invoked by its parent test. Includes the 1,005-viewer minimal display projection and 100,000 ordered-award regression |
| Minimal core without default features or persistence | 14 points tests passed |
| Native points TLS scenarios | Coalesced 100 automatic adjustments, kept workflow revision stable, looked up a viewer outside the board, resumed with a fresh balance, rejected an old connection's confirmation, and removed data after grant narrowing |
| Controller frontend unit/component suite | 20 passed: additive confirmation survives balance changes, identity/grant/unit changes invalidate it, missing viewer is explicit, cancel sends nothing and lost ACK blocks writes without resend |
| Chromium `controller-points.spec.ts` | 2 passed, English at 1,100 px and Spanish at 390 px; requested/effective amounts and host/viewer/unit/create confirmation, native-only RPC trace, no page errors or horizontal overflow; Spanish screenshot visually inspected |
| TypeScript, targeted ESLint and packaged frontend build | Passed |
| Clippy, all targets of core/collaboration/control-API/client/gateway/CLI/desktop with full core and remote features | Passed with `-D warnings` |
| Final normal Desktop and CLI build, offline, CLI host/local-web enabled | Passed without the test-only Cloud-origin override; normal shell confirmed it was absent |
| Packaged Vue/Wry/native journal/real TLS host fixture, run separately | Passed: disable workflow, reload/history, delete workflow, look up Alice, confirm -3 Points, verify host balance 2 and reload with all three records; no client host-data directories |

These fixtures use private temporary hosts, synthetic awards and injected
memory credentials. They do not establish two physical PCs, a real OS keychain,
a real LIVE, complete domain UI, or Internet relay operation.

## Implemented foundation and transport

- Trusted `RequestContext` cannot be deserialized from JSON-RPC params.
  Existing local-owner execution stays compatible. Remote handlers receive an
  approved session context and never proxy calls into trusted-local IPC.
- The private collaboration store persists a stable host ID, invitations,
  independent sessions, credential verifiers, grants, refresh reuse detection,
  admission budgets, configuration generations and redacted audit. Restart
  preserves approved devices and invalidates interrupted pairing exchanges.
  An exclusive process lease is acquired before migration/exchange cleanup.
- Revocable authorization views share committed grant/session changes without
  per-event SQL reads. Service shutdown invalidates surviving views.
- Fifteen owner-only administration methods and the authenticated `hello`/`me`
  methods have typed SDK and validation coverage. Owner identity/session/invite
  lists use cursor and byte-budget pagination, including historical records
  beyond active-device admission caps. Audit cursors survive empty pages.
- `tiktools-remote` takes the existing `Arc<ControlApi>`. Desktop integrates it
  through its default `remote-control` feature; CLI hosts opt into that feature.
  Fresh idle local-only startup does not bind a remote listener or create its
  database. Pending workflow delivery opens the private audit store lazily,
  including when remote access is disabled.
  Builds without persistence reject management before opening the database;
  builds without the gateway reject enabling it.
- Direct TLS and explicit loopback TLS-proxy configuration are supported.
  Public plaintext is rejected. Native pairing/refresh use explicit native
  headers without cookies/Origin; native protected requests use bearer headers.
  Browser sessions use separate Secure/HttpOnly cookies, Origin/CSRF checks and
  single-use WSS tickets. Cookie/native collection cannot be interchanged.
- The minimal browser join page consumes invitation fragments locally and
  requests approval. It does not serve the complete controller application.
- WSS requires protocol-1 `collaboration.hello`. Connections, frames, params,
  pending calls, session/source rates and send/heartbeat deadlines are bounded.
  Configured lower limits are enforced and advertised for the actual connection.
  Responses are reauthorized immediately before send. Revocation, expiry and
  refresh invalidate connected credentials independently of other devices.
- Host supervision restores enabled configuration, restarts the gateway for
  configuration changes, reloads TLS files even at unchanged paths, and closes
  sockets on disable/shutdown. Desired/applied configuration revisions are
  visible in owner status. Disabling revokes remote access and keeps the host
  running.
- Every RPC defaults to denied remotely. The implemented allowlist is
  `collaboration.hello`, `collaboration.me`, filtered `rpc.discover`/`rpc.schema`,
  `live.status`, `live.monitor.status`, `ui.feed.backfill` and scoped
  `collaboration.commands.get`, scoped `workflows.list`/`get` and
  `collaboration.subscribe`/`snapshot`/`resume`. LIVE read grants
  and creator restrictions are enforced. Scoped shared backfill stays denied
  until its filtered projection is implemented. Monitor provider diagnostics
  are sanitized remotely. Remote workflow disable/delete is now implemented;
  definition save/enable and remaining domain writes remain unavailable. Points
  reads and additive adjustments are now available with scoped durable outcomes;
  see the points API evidence below. The handshake
  advertises subscriptions for `workflows`, `points` and `commands`;
  other domains and feed subscriptions fail explicitly.
- Workflow save/delete/toggle writes now compare versions and commit graphs,
  version tombstones and invalidation reservations in the same owning-database
  transaction. Legacy local writer paths participate. Atomic snapshot reads
  return matching versions. Non-destructive initialization seeds old workflows
  once; revision exhaustion and outbox failure roll back business state.
- Workflow API envelopes preserve domain result shapes and add
  `meta.resourceVersions`; stale saves return `revision_conflict`. The
  metadata-aware direct/IPC SDK and Vue bridge preserve these fields and
  conflict details. The local browser adapter forwards request/response metadata.
  Unsupported preconditions and command metadata on unadapted methods fail
  before execution. Owner workflow commands now persist exact outcomes with
  graph/version changes and required audit reservations; identical IDs replay
  without executing, mismatched payloads reject, and new stale-instance IDs
  fence before writes. Seven-day pruning retains ID/hash tombstones; configurable
  per-principal and store admission caps never evict deduplication records.
  The delivery/retention task is cancelled/joined on shutdown and in-flight
  command coordination drains before shutdown completes. Remote workflow access stays denied while the remaining command/effect and
  recovery requirements are incomplete.
- `collaboration.commands.get` uses trusted caller identity, current command/
  workflow read grants and resource scope, including expired outcomes and
  queued failures. An agent cannot select another principal. Local owner can
  select an approved identity explicitly. SDK/IPC/Vue transport retains command
  receipts and answering host instance IDs. CLI `api call` supports explicit
  command/precondition flags and metadata output; malformed flags fail before
  connecting, JSON errors retain redacted conflict/outcome data, dry runs do
  not reserve IDs, and no command is automatically retried.
- The host drains workflow resource/command outboxes in balanced bounded batches
  into a transactional audit index and a separate 1,024-entry projection channel.
  Audit import precedes publication; source acknowledgement follows both.
  Retry preserves event IDs and never enters the automation/plugin DomainEvent
  bus. Exact links preserve attribution when a no-op command shares a revision.
  Source cleanup waits for both linked deliveries, removes acknowledged metadata
  immediately, and keeps command-ID deduplication. Original timestamps enforce
  30-day audit retention without resurrecting old records. Open stores also
  prune on owner audit reads and the host's one-minute maintenance tick.
  Delivery failures use a two-second retry pause. New workflow writes
  reject malformed/unbounded IDs before reserving metadata.
  Desktop startup supplies its owned runtime explicitly because Winit builds
  the API outside an entered Tokio context; a synchronous-startup fixture
  verifies delivery and joined shutdown through that handle.
- Workflow capture holds the same barrier as local writes and outbox
  publication. A 30-second immutable paged baseline carries resource versions
  and its stream watermark. Permission filtering happens before hidden bodies
  are materialized; SQL size checks and per-page/total byte budgets reject
  oversized allowed records before retaining a partial baseline. Notifications
  wait until the complete baseline has been requested. A failed fresh capture
  clears readiness of the earlier baseline until a complete recovery succeeds.
- Reliable stream sequence is contiguous after filtering. A bounded per-stream
  journal retains original event IDs/sequences for resume under the same
  principal/session/grant/host instance. Native access-token rotation preserves
  the logical session. Invalid/expired cursors, incomplete baselines, replacement
  and permission changes require resync. Queued events and snapshot pages are
  reauthorized before send; a superseded snapshot cannot leak from a queued
  response. Unsent count/byte/age overflow breaks recovery without blocking the
  business publisher. Detached state and token pages expire without another
  mutation. SDK/CLI event envelopes preserve reliable metadata separately from
  DomainEvents; malformed reliable sequences force a gap.
- Cloud Sync withholds records with pending local edits from automatic
  application. Unresolved conflicts therefore do not toggle a local plugin.
- The optional native SDK backend pairs and negotiates over HTTPS/WSS, validates
  system trust or an explicitly approved certificate pin, checks stored host
  identity before loading credentials, and preserves metadata/filtered reads.
  Probe mode rejects the TLS handshake after capturing an unverified fingerprint;
  it cannot send an invitation or credential. Pins check dates, SAN/name, server
  usage and handshake signatures. HTTP redirects/retries and ambient proxies are
  disabled; transport/session errors never echo credential values.
- Native profiles store only metadata and an opaque credential reference. The
  production adapter directly selects the platform OS backend and reports
  unavailable storage without a file fallback. One process lease serializes a
  profile open; a durable nonsecret refresh attempt and saved keychain generation
  fence uncertain one-use rotations. Caller cancellation does not abandon a
  known reply between keychain/profile writes. Duplicate stored copies of one
  session are rejected. Profile operations create no client host databases.
- `RecoveryStream` validates every baseline page, hides state while recovering,
  and advances its cursor only after explicit application acknowledgement.
  Reconnect distinguishes retained replay from a new baseline, including grant
  changes. Native CLI profile selection rejects local-runtime/host/sandbox
  combinations. `api events` reconnects only its selected remote profile with
  bounded backoff; ordinary domain calls are never automatically replayed.
- The desktop parses `--controller`/`--remote-profile` before host logging,
  installer handling, the owner instance lock or runtime construction. Its
  distinct window/bridge builds no local core, router, listener, plugin/media
  services or plugin asset handler. Invalid mixed arguments fail before any
  host files. The owner menu opens a separate controller process with fixed
  argv. The packaged Vue entry chooses a separate root without owner stores.
- The initial controller offers profiles, private invitation entry, explicit
  pin confirmation, owner approval, identity/connection status and permitted
  workflow data. Rust owns credentials and the acknowledged workflow
  projection. Selection epochs fence late RPC replies; revision-bound pages
  reject changing views. The page hides host data during recovery and retains
  separate in-memory drafts scoped to origin/host/principal/session. Effective
  permission changes rebuild domains, including connections without a stream.
  At that initial stage, complete domain UI, remote saves, desktop trust/profile
  management and persistent drafts were incomplete. The latest extension at the
  top now records explicit native draft persistence and local conflict comparison.

Setup and exact wire behavior: [transport guide](LIVE-COLLABORATION-TRANSPORT.md).
Native profile setup: [client guide](LIVE-COLLABORATION-CLIENT.md).
Owner setup: [settings guide](LIVE-COLLABORATION-OWNER.md).

## Owner settings evidence

- The section mounts only when selected, uses the local control client and
  never enables access on mount. English/Spanish labels, narrow layouts,
  strict grant editing and explicit confirmations are implemented. The public
  remote pairing page does not expose owner settings.
- Configuration CAS checks run atomically with configuration, revocation and
  audit changes. Tests prove stale disables have no side effects, simultaneous
  writers have one winner, and the wire precondition is never persisted.
  Legacy unfenced calls remain supported.
- Owner-only `collaboration.principals.get` validates exact IDs and bounds grant
  bytes before parsing. Sessions can open an editor for a principal outside the
  current list page. Device labels are descriptive, not authorization.
- Status reports the active TLS fingerprint and per-session socket counts.
  An internal listener generation prevents retired connections decrementing a
  replacement's counts. Shutdown clears current diagnostics.
- Invitation secrets stay in the mounted panel and use URL fragments. They are
  excluded from history/persisted frontend stores and cleared on hide, expiry
  or unmount. QR canvas generation is local and opt-in. Tests cover escaped
  labels, exact request approval, private display and expiry.
- Polling/conflicts preserve form edits. Mutations are sent once; unknown
  outcomes block new mutations until explicit review. Late unmount/tab/host
  replies are rejected. Short audit pages continue until empty; unsafe numeric
  cursors are rejected instead of rounded.
- Chromium fixture tests cover grant editing, local QR, approval, revocation
  and Spanish narrow layout with a fake control transport. A separate real
  integration starts one actual headless owner, its loopback browser adapter
  and packaged Vue UI. The UI configures generated TLS, issues/approves two
  native clients, revokes one while the other still reads, then disables the
  listener. Both clients validate an independently computed certificate pin
  and share one host instance. Owner IPC still works after disable. Paths are
  temporary, Cloud disabled, credentials remain in private pipes/memory, and
  no production account or LIVE is involved.

## Remote workflow command evidence

- SDK/CLI callers can disable/delete workflows under exact workflow scopes,
  read plus toggle/edit capability, and mandatory command/instance/revision
  metadata. Current owners keep their previous payloads and writer paths.
  Save/enable remains denied pending delegated-effect manifests and execution
  enforcement; this does not complete remote workflow editing.
- The core captures identity from the live approved session, rechecks after its
  writer gate and again inside the SQLite immediate transaction, before reading
  the resource or replaying a command. The queued-work test independently holds
  the core gate or the business database's real writer lock, removes scope or
  revokes the session, then releases admission: no change/command is committed.
  Injected required-audit failures also roll back both remote disable and delete,
  their resource revisions and command/outbox records atomically.
- The API test covers filtered discovery, missing metadata/read permission,
  hidden resources, CAS conflicts, independent principal namespaces, changed-ID
  payload rejection, original-result replay after owner reactivation, deletion
  replay after owner recreation, unknown/missing resource deletion, scoped
  outcome queries and egress revocation of both results and conflict metadata.
- A real TLS test uses 4-KiB frames and a large owner definition. Remote disable
  returns and persists only `{id,enabled:false}`, so a graph body cannot overflow
  its command receipt. A separate socket receives the caller-owned command
  notification. Reconnect/retry/query returns the original result while the
  owner's later reactivation stays intact. Delete follows the latest revision.
  The imported audit entry has the remote principal/session and is deduplicated.
- `hello.features` advertises disable/delete separately from unsupported
  save/enable. Owner grant editing exposes the two permissions with their read
  prerequisite; reviewer presets still grant no write capability or resource.
  The desktop now uses confirmed native-journal commands as described below;
  raw SDK/CLI callers still manage command IDs explicitly.
- Remote internal/storage failures redact diagnostic messages/data. A handler
  fixture returns private SQL/path markers and proves only local-owner callers
  retain those diagnostics. Public revision/command errors retain their typed
  metadata and scope checks.

## Desktop durable-command evidence

- The native controller owns a private metadata-only journal bound to profile,
  origin, host, principal and session. A synced intent precedes every mutation.
  There is one unresolved intent per profile, up to 16 resolved metadata records
  and a 64-KiB file budget. Mutation IDs never come from the page. Raw forwarding
  permits completed reads only. Forgetting cannot silently remove pending intent.
- Nine journal tests cover exclusive ownership, all identity bindings, bounded
  structured data/history, private Unix paths, symlink/hard-link refusal,
  intent/result persistence failure, exact maximum revisions and process death.
  The ignored crash helper is explicitly invoked at three real subprocess exit
  points: before intent, after synced intent and after synced result. No network
  command is sent by that filesystem-only helper.
- Six coordinator tests use the real TLS host and an in-memory credential-store
  fixture: query a previously committed ID after reconstructing the controller
  without reapplying it; retain a never-accepted intent until explicit review;
  refuse dispatch when persistence fails; continue owned native work after a
  page caller is cancelled; reject a switch during persistence without sending
  to either host; and validate exact receipt/resource/revision bindings.
- Frontend tests cover immutable confirmations, missing/mismatched journals,
  unknown barriers, explicit query/review without resend, profile reply fences,
  retained drafts after deletion and hiding drafts outside current read scope.
  Two Chromium fixtures verify confirmations, cancelled confirmation, retained
  drafts, uncertain results and review in English/1100px and Spanish/390px.
  Both rendered layouts were visually inspected; narrow dialog labels fit.
- The opt-in packaged GUI test runs real Vue in Wry with the production bridge,
  native journal and a real TLS gateway over one isolated owner core. UI clicks
  confirm disable at revision 1, reload the page, recover native history and
  confirm delete at revision 2. Final tombstone revision 3 and two history records
  prove no reload reapplication. Client directories contain only profiles. The
  test uses private temporary host data/certificates and memory-only credentials,
  with Cloud disabled and no TikTok connection or production account.
- Initial GUI probing ran before frontend readiness and timed out; the driver
  now waits for the actual ready generation, including after reload. The final
  test passed. This test-driver fix does not loosen product permission checks.

## Points persistence evidence

The [points specification](LIVE-COLLABORATION-POINTS.md) records the preceding
local consistency foundation. Its remote API/command extension is verified in
the next section. The subsequent points-stream section records snapshot/replay;
the later native coordinator/journal extension is recorded separately below;
the points view/form were pending at that earlier stage; the latest extension
is recorded at the top of this document.

- The mutation gate spans cold reads through enqueue; deterministic barriers
  cover an award paused after releasing the viewer lock, overtaking awards/
  adjustments/resets and shutdown. Writer completion tests use the resolved
  watermark, not queue/executing gauges that can transiently both be zero.
- Cold reload timeout refuses pre-reset SQLite state. Invalid single-viewer
  input cannot become a full reset. Unsafe arithmetic rejects without a
  balance mutation; large valid levels saturate. A later successful batch
  cannot conceal an earlier permanently failed durable sequence.
- Viewer revisions/outbox reservations share both the batch and per-operation
  fallback transaction. Tests inject reservation failure and revision exhaustion
  and verify rollback of balances, history and versions. Reset covers cold
  viewers; migration seeds old rows once without fictitious award delivery.
  Partial-batch fallback counts each actually committed operation once.
- Delivery namespaces automation/points acknowledgements and admits independent
  bounded quotas. High-volume awards do not create the identity/audit store;
  manual operations require audit. Tests cover a failed second audit chunk,
  acknowledgement failure after the other database already acknowledged,
  stable-ID retry, redacted metadata and no business-event replay.
- Viewer API lookup now runs on the blocking pool: a cold read or pending
  writer/reset must not block a Tokio network worker.

The preceding foundation's complete core execution passed **458 tests**, with three subprocess
helpers ignored in the parent suite. It invoked the freshly built test
executable serially, with Cloud disabled and every application path in a
disposable directory. `TMPDIR` and `SQLITE_TMPDIR` were placed on the main disk:
the system `/tmp` tmpfs reached 100 percent during earlier repetitions and
produced SQLite disk-I/O failures and incomplete logs. No temporary user data
was deleted to make the test pass.

Parallel whole-core runs also hit the pre-existing
`enabled_event_subscriber_starts_with_host_runtime` shutdown-safety assertion;
that test passed alone and the complete serial run passed. The older full-core
fixtures share a process data root and construct independent owner cores,
whose ranking checkpoint ownership can fence each other. Parallel whole-suite
stability remains unconfirmed. New points/storage/delivery fixtures use private
paths and deterministic barriers. These results do not establish concurrent
controller load, the 10-percent overhead or physical two-PC acceptance.
The current extension above records the later fake-host fixture correction
and a newer passing default-parallel full-core run.

That foundation's no-persistence build passed all **14** selected points tests. The five
shared implementation crates passed all-target Clippy with warnings denied,
and the earlier API/SDK/gateway/store command passed **143** tests, superseded
by the 147-test points-command run and the latest 151-test points-stream run below. Desktop,
frontend and Wry results below remain evidence from the earlier workflow-client
stage; this points foundation does not add or claim a points controller UI.

## Points API and command evidence

The [points contract](LIVE-COLLABORATION-POINTS.md) now includes remote
`points.viewer.get`, `points.leaderboard` and additive `points.adjust`.
Command/host-instance IDs are mandatory for remote writes; CAS preconditions
reject before execution. Legacy local-owner envelopes and optimistic calls are
preserved. Reset and configuration methods remain denied remotely.

- Fresh adjustments drain earlier snapshots before the transaction, commit
  with FULL synchronous durability and foreign-key enforcement, and update the
  hot cache only after commit. The bounded mutation/drain/SQLite admission
  rejects before acceptance on overload or any earlier durability failure.
- Tests force a producer paused before enqueue, a stalled writer and permanent
  failure, concurrent duplicate callers, and automatic awards racing a command.
  Balances, history, versions and receipts remain consistent without replaying
  a business event. A required command reservation failure rolls everything back.
- A deterministic barrier holds the actual admitted SQLite transaction while
  the grant loses its write capability. Releasing it cannot accept the command
  or reserve an ID. Scope/capability/bonus-limit tests and queued egress checks
  cover viewer data, outcomes and missing-record version metadata.
  Penalties preserve the nominal delta/zero floor; clamping cannot bypass the
  requested-delta limit, and replay cannot remove a later award.
- Principal/command IDs and capacity span automation and points databases.
  Expired outcomes keep tombstones and cannot execute again. A later award,
  writer shutdown or host restart cannot make a historical replay rewind cache
  state. Process fixtures terminate before commit and after commit/before cache
  update, then verify rollback or original-outcome recovery under the same ID.
- SQL filters permission before ranking/loading bodies. Query-plan checks prove
  scoped boards use primary-key searches rather than scan hidden viewers, while
  unrestricted boards use the full ranking index. Serialized/raw byte budgets
  reject oversized visible data without returning a partial board.
- Direct and IPC SDK metadata tests now cover additive points and cross-domain
  ID rejection. An actual TLS/WSS two-session test discards the first response,
  reconnects, resolves/replays the original result once, verifies a later owner
  adjustment and scoped board, and denies the read-only client's mutation.

The API-only stage did not include the native points journal/UI. The coordinator
extension is recorded below; the points view/form were still pending at that stage. These results do
not complete the points domain's UI/load/platform or physical LIVE acceptance.
The prior serial whole-core run below is historical; this step reruns the
affected points/SQLite suites and shared implementation.

## Points stream evidence

The host now advertises `points` alongside `workflows` and `commands`. Points
subscriptions require the current points-read/viewer grant without requiring
workflow access. Commands are caller-owned and independently filtered by their
resource kind/read capability; creator-restricted sessions cannot receive global
viewer data. Discovery also hides recovery methods without a supported read
grant. Grant changes invalidate retained pages and queued updates.

- SQLite captures a complete permitted viewer set with matching versions in one
  read transaction, filtering before loading bounded profile bodies. The
  baseline is not the public leaderboard's 1,000-row window. Tests use 1,005
  rows, an oversized hidden/visible profile, explicit empty grants and total
  budgets; oversize fails without retaining a partial baseline. A first direct
  executable launch from the checkout imported copies of legacy data into
  fixture databases, causing unrelated row counts. The fixture now precreates
  empty databases before initialization, independently of the launch directory;
  original legacy files were not modified.
- Immutable points pages keep original balances/revisions while a reset commits
  and its invalidation is delivered. Notifications wait for the last page.
  Automatic awards and resets publish metadata, never replay an award.
- Mixed workflow/viewer pages can contain identical IDs. The SDK indexes
  versions by kind/scope/ID, rejects duplicate or unmatched bodies/versions,
  and validates event topics against subscribed domains and resource targets.
  Snapshot budgeting includes the optional points field and token metadata.
- Real TLS SDK tests exercise mixed multi-page baselines at a 4-KiB frame,
  principal-private command receipts, points-only sessions, replay of an
  unacknowledged update and grant-change resnapshot. Reliable invalidations
  remain separate from the pending native display's four-updates/second limit.

CLI `api events` selects points and caller-owned commands for points-only grants,
respecting advertised domains, topic filters and creator restrictions. Older
hosts cannot be coerced into an unadvertised points subscription.

Native points projection/form, high-load behavior, platform credential
stores and physical two-PC/LIVE acceptance remain required.

## Native points intent and migration evidence

The native bridge now has `controller.points.command` with
`{epoch,uniqueId,delta,currencyName}`. Rust validates the selected connection,
feature, read/adjust/viewer scope and bounded finite input, then generates and
syncs the intent before sending once. Additive commands contain no fake CAS
revision. `points.info` exposes only the permitted unit label; the host checks
optional `expectedCurrency` against its frozen configuration before accepting
a fresh adjustment. Existing unchecked payload hashes remain unchanged.

- Version 2 preserves every version-1 pending workflow ID, binding and recent
  record under the same exclusive journal lease. Subprocesses terminate before
  migration replacement, after rename and after directory sync. Invalid old
  point fields/unsupported formats fail closed without rewriting an intent.
  The journal process-death fixture covers both workflow and points intents/results.
- A separate native coordinator process connects over real TLS and terminates
  before intent persistence, after synced intent/before send, after the committed
  host reply and after synced result. The surviving owner accepts zero or one
  adjustment as appropriate; reopening never resends an unaccepted pending ID
  and recovers the original committed result after a later award. This uses a
  private fixture credential store, not a production account or OS-store proof.
- TLS native-coordinator tests cover points-only grants, page-chosen ID rejection,
  blocked raw mutation forwarding, amount/viewer validation, pre-acceptance unit
  mismatch and zero-floor penalties. A cancelled page plus a profile switch
  during disk sync cannot send the old intent to either host.
- Recovery reads the original ID after later owner awards and a unit rename,
  verifies the exact receipt/resource/unit/effective amount/positive revision,
  and stores the historical result without overwriting current host state.
  Forbidden/unknown/expired outcomes keep the pending ID and are never resent.
- The common Vue history/review panel distinguishes requested amount from the
  verified effective delta and total, including units. Native status/command
  projections hide historical host totals while disconnected/recovering or when
  an updated grant no longer allows that points-read/viewer scope; the private journal is
  preserved for authorized recovery. Projection and unknown-review data remain
  separate from the host's live points view.

At that stage, the native points live projection, adjustment form/confirmation
and actual Wry points interaction were pending. The latest extension at the top
now records their implementation and separate evidence. No physical two-PC/LIVE or real OS-store points
acceptance is claimed by these fixture tests.

## Executed checks

Local Linux checkout; changes are uncommitted. The native-points shared run
passed **161 tests**: 41 store/type/stream tests, 52 API tests, 54 SDK tests and
14 gateway tests. A later focused native-coordinator run passed 12 tests,
including its newly added real process-death parent; one child helper is
ignored and invoked explicitly at four crash boundaries. The extended live
grant narrowing/restoration/removal test also passed separately. Ignored child
fixtures are not counted as ordinary passes. The points-core run passed 47
tests with one explicitly invoked subprocess helper ignored in the parent.

The latest desktop's earlier workflow stage passed 103 unit tests, one actual
executable selection test and two separately invoked real-Wry controller tests.
The CLI's 36 unit tests passed again; the 4 earlier actual local-integration
tests remain historical for this native journal extension. Remote selection's earlier
3 subprocess tests remain historical. The SDK's lease, journal/migration and
native coordinator crash children are explicitly invoked by their parents.
The current controller-only frontend suite passed 10 tests, followed by clean
TypeScript/ESLint checks and a fresh packaged frontend build.
The latest frontend run passed 58 unit/component tests, plus two Chromium owner
fixture tests, two additional Chromium controller command tests and the prior
real owner/browser/TLS integration. A new real Vue/Wry/native journal/TLS-owner
test passes with confirmed disable, page reload/history recovery and delete.
Owner settings were exercised in Chromium; native controller tests use an
isolated in-memory credential-store fixture, not a real OS keychain.

| Command | Result |
| --- | --- |
| `cargo test -p tiktools-collaboration -p tiktools-control-api -p tiktools-client -p tiktools-remote --features tiktools-client/remote-profiles --locked --offline` with private main-disk temporary data and Cloud disabled | Passed 161 tests for the native points extension, including checked currency/read info, version-2 migration, workflow/points recovery and original gateway/API regressions; three child helpers ignored in the parent harness |
| `cargo test -p tiktools-client --features remote-profiles --locked --offline --lib remote::controller::commands::tests:: -- --test-threads=1` | Passed 12 native coordinator tests, including a separately terminated real TLS client at four intent/commit/result boundaries; one child fixture ignored in the parent harness |
| `cargo test -p tiktools-client --features remote-profiles --locked --offline --lib remote::controller::commands::tests::native_points_command_is_durable_without_workflow_access_or_javascript_command_ids -- --exact` | Passed after extending the test to hide/restore historical totals under viewer-scope narrowing/restoration and read-capability removal |
| `cargo test -p tiktools-core --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --lib points:: -- --test-threads=1` | Passed 47 tests after checked-currency extension; original unchecked commands, crashes, award ordering, scopes, ledger and projection limits remain covered; one subprocess fixture ignored in the parent |
| `cargo clippy -p tiktools-core -p tiktools-collaboration -p tiktools-control-api -p tiktools-client -p tiktools-remote -p tiktools-cli -p tiktools-desktop --all-targets --features tiktools-core/persistence,tiktools-core/native-tiktok,tiktools-core/http,tiktools-core/plugin-install,tiktools-core/cloud-auth,tiktools-client/remote-profiles,tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline --no-deps -- -D warnings` | Passed for the final native points coordinator, journal migration, typed info and test fixtures across all seven implementation packages |
| `bun x vitest run src/web/features/controller` | Passed 10 controller tests, including requested/effective/history units and unknown-review context |
| `bun x vue-tsc --noEmit`; `bun x eslint src/web/features/controller/model/controller.ts src/web/features/controller/ui/ControllerRoot.vue src/web/features/controller/ui/ControllerRoot.test.ts --max-warnings=0`; `bun scripts/task.ts frontend-build` | Passed after journal-history integration; no points adjustment form is claimed |
| `cargo test -p tiktools-desktop --bin tiktools-desktop controller::smoke::commands::packaged_controller_confirms_tls_commands_and_keeps_history_after_reload --locked --offline -- --ignored --exact --nocapture` | Passed again with the version-2 journal and current packaged frontend: real Vue/Wry/native bridge/TLS workflow disable, page reload/history and delete. This does not establish the still-pending points form |
| `cargo test -p tiktools-cli --features remote-control,local-web --bin tiktools --locked --offline`; `cargo test -p tiktools-core --no-default-features --locked --offline --lib services::points::tests -- --test-threads=1` | Passed 36 CLI unit tests and 14 core points tests without persistence after the native points/typed-parameter extension |
| `cargo build -p tiktools-desktop -p tiktools-cli --bin tiktools-desktop --bin tiktools --features tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline` in the normal environment | Passed for the final native points journal/checked-currency implementation; the packaged frontend was rebuilt separately. No test-only Cloud-disable override was embedded in these final binaries |
| Previous foundation: fresh core test executable built with `persistence,native-tiktok,http,plugin-install,cloud-auth`, invoked with `--test-threads=1` and the isolated environment described above | Previously passed 458 tests with 3 subprocess helpers ignored in the parent harness; not a new whole-core run for the additive-command step |
| `cargo test -p tiktools-core --no-default-features --locked --offline --lib services::points::tests -- --test-threads=1` with `TMPDIR` on the main disk | Passed 14 points tests without persistence |
| `cargo clippy -p tiktools-core -p tiktools-control-api -p tiktools-collaboration -p tiktools-client -p tiktools-remote --all-targets --features tiktools-core/persistence,tiktools-core/native-tiktok,tiktools-core/http,tiktools-core/plugin-install,tiktools-core/cloud-auth,tiktools-client/remote-profiles --locked --offline --no-deps -- -D warnings` | Passed for the final points foundation, including test targets |
| `cargo test -p tiktools-cloud-sync --locked` | Previously passed 12 tests, including unresolved-conflict regressions; unchanged since that run |
| `cargo test -p tiktools-collaboration -p tiktools-control-api -p tiktools-client -p tiktools-remote --features tiktools-client/remote-profiles --locked --offline` with `TMPDIR` on the main disk and Cloud disabled | Passed all 151 tests, including points/mixed-domain snapshot/replay at 4-KiB frames, points-only reconnection/grant changes, principal-private command events, additive API, direct/IPC metadata, TLS lost-ack recovery and existing workflow/native-controller regressions |
| `cargo test -p tiktools-core --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --lib points:: -- --test-threads=1` with private temporary data on the main disk | Passed 45 tests, including points process-death recovery, actual SQLite-admission revocation, racing awards, penalties, scope/bonus limits and query plans; one subprocess helper ignored in the parent harness |
| `cargo test -p tiktools-core --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --lib db::sqlite:: -- --test-threads=1` with private temporary data on the main disk | Passed 47 tests and two subprocess helpers ignored in the parent harness; includes generic ledger, workflow regression, delivery, points rollback and migration coverage; overlaps the points suite |
| `cargo test -p tiktools-core --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --lib control::streams:: -- --test-threads=1` with private main-disk temporary data | Passed 5 stream tests, including immutable points pages during reset and existing workflow snapshot/publication/grant regressions |
| `cargo test -p tiktools-core --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --lib db::sqlite::points::tests:: -- --test-threads=1` | Passed all 7 SQLite points tests, including the complete 1,005-viewer baseline, scoped/visible profile budgets and legacy persistence/migration/query-plan regressions |
| `cargo clippy -p tiktools-core -p tiktools-collaboration -p tiktools-control-api -p tiktools-client -p tiktools-remote -p tiktools-cli -p tiktools-desktop --all-targets --features tiktools-core/persistence,tiktools-core/native-tiktok,tiktools-core/http,tiktools-core/plugin-install,tiktools-core/cloud-auth,tiktools-client/remote-profiles,tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline --no-deps -- -D warnings` | Passed all seven implementation packages and their test targets after points snapshot/replay and the actual CLI additive test extension; the subsequent CLI points-event change has its own final Clippy run |
| `cargo build -p tiktools-desktop -p tiktools-cli --bin tiktools-desktop --bin tiktools --features tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline` | Passed after points snapshot/replay; the final CLI event selector is rebuilt separately below |
| `cargo clippy -p tiktools-cli --all-targets --features remote-control,local-web --locked --offline --no-deps -- -D warnings` | Passed for the final CLI points-only event selector and all CLI test targets |
| `cargo build -p tiktools-cli --bin tiktools --features remote-control,local-web --locked --offline` | Passed for the final points event selector in the normal build environment, without the test-only Cloud-disable override |
| `cargo test -p tiktools-client --features remote-profiles --locked --offline --lib remote::recovery::tests::` | Passed the additional SDK contract test for identical IDs across kinds, subscription domains and command targets |
| `cargo test -p tiktools-cli --features remote-control,local-web --bin tiktools --locked --offline` with `TMPDIR` on the main disk | Passed 36 unit tests, including additive ID/instance flags without a CAS revision, points-only event selection, older-host domain checks and creator-restricted denial |
| `cargo test -p tiktools-cli --features remote-control,local-web --test collaboration_owner_browser --locked -- --ignored --nocapture` | Passed 1 real owner UI/local adapter/IPC/TLS test with two independent clients and one host; requires built `dist/web`, Bun and Playwright Chromium |
| `TIKTOOLS_WEB_PORT=34819 bun x playwright test tests/e2e/collaboration.spec.ts --reporter=line` | Passed 2 Chromium owner fixture tests: grants/approval/revocation/local QR and Spanish 390px configuration/disable |
| `bun x vitest run src/web/entities/collaboration src/web/features/manage-collaboration src/web/pages/settings/ui/SettingsPage.test.ts src/web/features/controller src/web/shared/api/control-client.test.ts src/web/shared/api/browser-transport.test.ts` | Passed 58 unit/component tests |
| `cargo clippy -p tiktools-collaboration -p tiktools-control-api -p tiktools-client -p tiktools-remote -p tiktools-cli -p tiktools-desktop --all-targets --features tiktools-client/remote-profiles,tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline --no-deps -- -D warnings` | Passed after remote workflow disable/delete and owner UI/API additions |
| `cargo test -p tiktools-desktop --bin tiktools-desktop --locked --offline` | Passed 103 tests; both real-controller GUI tests are ignored in this ordinary suite and invoked separately below. Existing opt-in GUI fixtures do not establish their real-display acceptance through this run |
| `cargo test -p tiktools-desktop --test controller_selection --locked --offline` | Passed the actual binary test: invalid mixed options/secret arguments fail before logs, GUI, locks or host data |
| `cargo test -p tiktools-desktop --bin tiktools-desktop controller::smoke::packaged_controller_reloads_without_starting_a_local_host --locked --offline -- --ignored --exact --nocapture` | Passed on the local Linux display, with built `dist/web`: packaged Vue controller root/native IPC and three distinct page generations; isolated profile home creates no local-host domain directories |
| `cargo test -p tiktools-desktop --bin tiktools-desktop controller::smoke::commands::packaged_controller_confirms_tls_commands_and_keeps_history_after_reload --locked --offline -- --ignored --exact --nocapture` | Passed actual Vue/Wry/bridge/native-journal/TLS host integration: confirmed disable, page reload with history, confirmed delete; final tombstone 3, two records, no client host-data directories; memory-only test credentials |
| `TIKTOOLS_WEB_PORT=34819 bun x playwright test tests/e2e/controller-commands.spec.ts tests/e2e/collaboration.spec.ts --reporter=line` | Passed 4 Chromium fixture tests: owner access management plus controller confirmations/unknown review and retained drafts in English and Spanish/390px; rendered layouts inspected |
| `cargo clippy -p tiktools-client -p tiktools-desktop -p tiktools-cli --all-targets --features tiktools-client/remote-profiles,tiktools-cli/remote-control,tiktools-cli/local-web --locked --offline --no-deps -- -D warnings` | Passed with the new desktop/native projection and shared private invitation parser |
| `bun x vitest run src/web/features/controller src/web/shared/api/control-client.test.ts src/web/shared/api/browser-transport.test.ts` | Passed 31 tests, including scoped drafts, late-page rejection and private invitation UI behavior |
| `cargo test -p tiktools-control-api --test command_outcomes --locked` | Passed again after explicit desktop runtime startup; entered-runtime API startup, durable retry and owner audit delivery remain integrated |
| `cargo clippy -p tiktools-collaboration -p tiktools-control-api -p tiktools-client -p tiktools-remote -p tiktools-cli --features tiktools-client/remote-profiles,tiktools-cli/remote-control,tiktools-cli/local-web --all-targets --locked --no-deps -- -D warnings` | Passed for implementation crates, native profiles, CLI host integration and local-browser forwarding |
| `cargo test -p tiktools-core --features persistence db::sqlite:: --locked` | Passed 35 persistence/coordination/delivery tests, including queued revocation under core and SQLite writer admission; two subprocess-only fixtures are ignored in the parent suite and explicitly invoked at two crash points each by their parent tests |
| `cargo test -p tiktools-core --features persistence control::streams:: --locked` | Passed 4 tests: concurrent writer/capture/publication watermarks, filtered immutable pages, superseded pages/grant changes and bounded reads |
| `cargo clippy -p tiktools-core --lib --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --no-deps -- -D warnings` | Passed for the modified core library |
| `cargo clippy -p tiktools-core --all-targets --features persistence,native-tiktok,http,plugin-install,cloud-auth --locked --offline --no-deps -- -D warnings` | Passed including the new remote admission/rollback test and existing persistence/stream tests; two test-only style diagnostics were corrected |
| `bun x vitest run src/web/shared/api/control-client.test.ts src/web/shared/api/browser-transport.test.ts` | Previously passed 28 tests, including metadata, command receipt and conflict regressions; frontend unchanged by workflow delivery |
| `cargo test -p tiktools-cli --features remote-control,local-web --test client_integration --locked --offline -- --test-threads=1` with a short private `TMPDIR` on the main disk | Passed 4 actual child-host/direct/IPC concurrency, event and command-metadata tests, including additive points receipt/replay/query and rejection of wrong/missing CAS metadata |
| `cargo test -p tiktools-cli --features remote-control,local-web --bin tiktools --locked` | Passed 34 unit tests, including remote selection, private invitation input and command metadata parsing |
| `cargo test -p tiktools-cli --features remote-control,local-web --test remote_selection --locked` | Passed 3 subprocess tests: rejected local/remote combinations, missing-profile failure without local fallback, and inventory/private-input behavior without host data or secret logs |
| `bun x vue-tsc --noEmit` | Passed |
| `bun x eslint src/web/shared/api/control-client.ts src/web/shared/api/control-client.test.ts --max-warnings=0` | Passed |
| `cargo check -p tiktools-cli --features remote-control --offline` | Passed during gateway integration; latest Clippy run also checked this feature |
| `cargo check -p tiktools-client --features remote --no-default-features --locked --offline` | Passed with native transport but no persisted-profile backend |
| `cargo check -p tiktools-client --features remote-profiles --target aarch64-linux-android --locked --offline` | Passed with NDK r27 Clang and target sysroot; Android selects the explicit unavailable OS-store backend. Android-only QuickJS bindgen was enabled after the first attempt found no bundled bindings; no Android execution is claimed |
| `cargo check -p tiktools-cli --no-default-features --locked --offline` | Passed without native remote/profile support |
| `cargo check -p tiktools-cli --no-default-features --features remote-client --target aarch64-linux-android --locked --offline` | Passed with NDK r27 Clang and target sysroot; compile-only evidence for CLI/profile error paths without Cloud account support |
| `cargo check -p tiktools-cli --target aarch64-linux-android --locked --offline` | Did not pass: the existing default Cloud account WebAuthn dependency requires target OpenSSL, unavailable in this cross toolchain. This is separate from the native remote transport; no full default Android build is claimed |
| `cargo check -p tiktools-desktop --locked --offline` | Prior SDK/desktop build check; newer controller startup/Wry evidence is listed separately above. No physical-device acceptance is implied |
| `cargo test -p tiktools-control-api --no-default-features --test collaboration --locked` | Passed again after stream integration; management does not create a collaboration DB without persistence |
| `node --check crates/tiktools-remote/src/join.js` | Passed syntax check; not browser UI acceptance |
| `git diff --check` | Passed |

The network fixtures use real HTTP/HTTPS and WS/WSS on loopback, temporary
certificates and isolated app-data/plugin directories. They exercise:

1. Certificate validation, host fingerprint hint, negotiation, filtered
   discovery, allowed reads, denied owner calls, grant updates, refresh reuse
   and independent revocation against one core.
2. Browser cookie separation, CSRF, Origin, one-use socket tickets and logout.
3. Host/query rejection, mixed credentials and actual per-principal/host
   connection admission limits.
4. Configured lower limits, parameter rejection and frame isolation.
5. Local-only startup without a collaboration DB.
6. Creator restrictions without exposing another creator or unfiltered history.
7. Supervisor restore, reconfiguration, disable/re-enable and host shutdown.
8. TLS replacement at the same file paths, rejection by the old trust root,
   stable host ID and continued validity of approved sessions.
9. Scoped workflow state/events, contiguous filtered sequence, detached replay
   after native token refresh, wrong-principal denial and grant-change closure.
10. Immutable multi-page snapshots under a 4-KiB frame limit during owner edits,
    and rejection of resume after an incomplete baseline disconnect.

Store tests include concurrent one-use redemption, expiration, refresh reuse,
independent revocation, atomic grant revisions, audit rollback, process ownership,
durable configuration generations, pagination through 1,050 historical
identities/sessions/invitations, and byte-budget pagination of large grants.
Workflow persistence tests cover concurrent saves with one winning revision,
local toggles invalidating stale saves, delete/recreate fencing, old-schema
migration, revision exhaustion, and rollback when invalidation reservation
fails. Command tests also terminate child processes immediately before and
immediately after SQLite commit, bypassing destructors, then inspect recovered
business data, receipts and both outboxes. Duplicate races, semantic hash
changes, host restart, expiry tombstones, quota rejection, no-op deletes,
required audit-write rollback, grant/session changes and shutdown drain are
covered. API, direct/IPC SDK, CLI and frontend tests verify envelope compatibility,
precondition forwarding, original retained receipts, current versions on
conflict, full JSON error data, and rejection of unsupported command metadata
without executing the operation. Delivery tests also terminate child processes
after audit import and after projection publication, bypassing destructors;
recovery preserves IDs, imports one audit record and never reapplies the command.
Destination import failure, source acknowledgement failure, linked-queue cleanup
in either order, 100-entry fair batches, original-timestamp retention and
host-worker shutdown are covered. The owner API exposes imported workflow
metadata while remote callers remain denied access to owner audit listing.
Workflow recovery tests also cover concurrent writer/capture/publication
watermarks, immutable multi-page reads during edits, resource filtering before
parsing hidden malformed data, stale-page/queued-event denial after grant or
snapshot changes, replay after native refresh, wrong-principal denial,
4-KiB-frame paging and incomplete-disconnect rejection over real TLS/WSS.
Unit fixtures exercise journal count/byte/age bounds, detached admission caps,
page expiry, total snapshot metadata budgets and sequence preservation beyond
JavaScript's safe integer range.

Native SDK fixtures pair against a real TLS host and test mandatory hello,
typed calls and metadata, denied owner/resource access, certificate trust,
expiry/name/usage rejection, admission and parameter limits, refresh/logout,
immutable baseline acknowledgement and connection lifetime. Actor tests cover
lost acknowledgements without resend, malformed correlation, secret reflection
redaction and a silent peer's heartbeat deadline. Profile fixtures explicitly
inject a credential store: they test nonsecret metadata, session binding and
duplicate-session rejection, unavailable-store failure, lease ownership,
uncertain refresh fencing, saved-generation repair, cancellation during
refresh, unacknowledged-event replay and new-grant baselines. A parent test
starts and kills a second process to prove the lease is released after process
death; its helper is intentionally ignored in the ordinary suite. Linux file
permissions and symlink rejection are covered. Refresh crash boundaries are
simulated durable states; actual power loss and production OS keychains have
not been exercised. CLI subprocess checks are failure/inventory checks, not
successful remote CLI sessions against a production keychain.

Four controller fixtures use actual TLS with an explicit in-memory credential
store. They cover owner approval/pin confirmation, initial filtered workflow
state, owner edits/deletion, refused owner RPCs, disconnect/recovery,
unavailable-profile clearing and permission removal/restoration (including a
connection with no domain stream). Browser-model tests retain scoped drafts
and discard an earlier host's delayed page after selection changes. A Vue
component test clears the invitation input and makes only controller RPCs.
The separately invoked Linux Wry fixture loads the actual built Vue assets,
checks the isolated controller root/private-input form and native IPC through
three page generations, then checks its profile home for host directories.
That initial root/reload fixture uses an empty profile inventory. The separate
workflow command fixture later connects the packaged Vue/Wry controller to a
real TLS host through its native journal, including page reload. It injects
credentials and does not establish a real OS-store backend. The extended
workflow/points fixture now also looks up Alice, confirms a -3 Points adjustment,
verifies the host balance of 2, and reloads with all three command records.
Two complete desktop clients connected to one LIVE remain unverified.

These checks do not prove recovery for every external-effect kind,
all-domain recovery, physical two-PC acceptance or the complete remote
browser/controller UI. The tested owner browser panel is local administration.
The default-feature build emits existing optional-path warnings in core;
warning cleanup was not included. No production account, LIVE or backend
service was used by these fixtures.

## Remaining acceptance

All end-to-end COL acceptance IDs in
[the specification](LIVE-COLLABORATION-SPEC.md) remain open. These tests provide
partial evidence for identity, permissions, transport and Cloud conflict
handling; they do not establish complete COL-01 through COL-30 acceptance.

Required work: transactional resource revisions/ledgers for the remaining domains;
remaining plugin/native-input/TTS external-effect recovery, delivery for other domains
and remote replay; delegated workflow/script effect policy;
authorized streams/snapshot recovery for the remaining domains, scoped feed history;
owner-administration command deduplication and complete denied-attempt auditing;
real native OS credential-store, refresh power-loss and cross-platform acceptance;
complete desktop domain UI and native trust/profile management;
browser controller UI and confined plugin
assets; domain permissions and mutations; host output behavior; metrics/load
and fault tests; Windows private ACLs; Windows/macOS checks; and two physical
PCs managing a real LIVE. Existing feed events carry creator metadata, which
must be used by the pending scoped history/recovery projection.

The version-1 baseline uses a directly reachable host. The optional
RemakeBackend relay remains outside that baseline. No relay, automatic NAT
traversal, production backend route, RemakeBackend database migration or
deployment has been implemented or validated. No physical two-PC or full browser controller result
is claimed by this record.
