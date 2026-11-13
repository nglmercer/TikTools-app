# Owner collaboration settings

Status: implemented owner administration UI, updated 2026-11-08. Scoped
workflow/event deactivation/deletion, points adjustments, native automation status
and API/SDK/CLI disabled author-only behavior drafts are available. Enabled
remote definitions, complete domain UI and physical two-PC acceptance remain
pending. Native Owner profile/template transactions and compound CLI imports
are implemented contracts under validation; executed evidence stays in the
validation record.
See the [product plan](LIVE-COLLABORATION.md),
[transport contract](LIVE-COLLABORATION-TRANSPORT.md),
[controller guide](LIVE-COLLABORATION-CLIENT.md) and
[validation record](LIVE-COLLABORATION-VALIDATION.md).

## Enable a host

Open **Settings → Collaboration** in the local owner's app. The section uses
the trusted local control channel; it is absent from the isolated remote
controller. Opening it reads status and redacted administration pages. Remote
access is disabled by default and requires an explicit configuration change.
The section is mounted only when selected; opening General settings does not
initialize collaboration storage.

The owner app must include `remote-control` (enabled by default for Desktop).
For a headless host and its local browser administration UI:

```bash
cargo build -p tiktools-cli --features remote-control,local-web --locked
tiktools host --ipc
# In a separate terminal, using the same TIKTOOLS_HOME:
tiktools web --assets /absolute/path/to/dist/web --port 5174
```

Use the local browser adapter's pairing code to open its owner UI. That local
adapter remains distinct from the remotely reachable collaboration listener.
Run one execution host per workspace.

Select **Allow remote controllers** and configure:

| Field | Meaning |
| --- | --- |
| Listen address and port | Host interface/port accepting remote connections |
| Public HTTPS address | Exact HTTPS origin controllers will connect to; no credentials, path, query or fragment |
| Certificate chain / private key file | Absolute paths on the host, for direct TLS; files are read by the host, not uploaded from the controller |
| TLS proxy | Explicit loopback TLS-termination mode; supply the external HTTPS origin and configure the proxy separately |
| Additional browser origins | Exact allowed HTTPS origins, one per line; not wildcard hosts |
| Advanced limits | Bounded connection, RPC, rate, frame and retained-command admission limits |

Select **Apply configuration**. A successful save precedes listener startup;
wait until status shows the current configuration applied and the listener
running. Errors are shown in the owner panel. For direct TLS, compare the active
listener's SHA-256 certificate fingerprint with the controller through an
independent trusted channel. With a TLS proxy, verify the proxy certificate
separately; the host does not claim its internal certificate is the public one.
Reachability, DNS, firewall rules and certificates are managed separately.

The panel checks status every two seconds and shows the last successful check,
host/instance IDs, active sessions and connected sockets. A session can have
more than one socket; an approved session with zero sockets is offline.
Counts from a retired listener cannot overwrite a replacement listener's counts.

## Invite and approve

1. In **Invite a controller**, choose Viewer, Workflow reviewer or Agent reviewer.
   Review exact capabilities and resource restrictions before creating an invite.
   Currently implemented remote capability groups are `live.read`,
   `workflows.read`, `commands.read`, `workflows.toggle` (disable only),
   `workflows.edit` (delete only), `points.read/adjust`, `automation.read`
   (status summaries), `automation.toggle` (action/event deactivation and
   activation under the applicable reusable delegation) and
   `automation.edit` (whole-event deletion plus disabled author-only drafts through
   API/SDK/CLI or the separate native proposal editor). The grant editor adds
   the read and command-recovery capabilities needed by selected write permissions.
   Reviewer presets grant reads only; choose write permissions
   explicitly. Other stored capabilities are marked as
   unavailable; selecting one does not enable an unsupported operation.
2. Each resource kind can allow all IDs, only the entered exact IDs, or no IDs.
   An empty restricted list means no resources. IDs are one per line and may
   not contain wildcards or control characters. Automation IDs require the
   family prefix `event:<id>` or `action:<id>`. Permitted actions can be
   disabled and eligible definitions activated from the native status board;
   the native proposal editor and API/SDK/CLI may submit disabled action drafts under the
   authoring restrictions below. Bare old automation restrictions are rejected
   rather than broadened.
   A LIVE creator restriction prevents access to global automation state.
   Advanced grant JSON preserves
   action/setting/points restrictions and is validated before submission.
3. Select **Create private invitation**. The one-use secret is returned once.
   The join link puts it in the URL fragment, not a query string. Reveal/copy
   and QR display require explicit actions. QR generation runs locally on a
   canvas and does not call a third-party service.
4. Deliver the invitation privately. On the controller, pair using that link
   and verify the displayed request ID against the owner's pending request.
   A claimed device label is descriptive; it does not prove identity.
5. Review the proposed grant, request ID and client details, then explicitly
   approve or reject. Approval binds that request to a host-local principal and
   an independent device session. A second device needs its own invitation and
   approval. Cloud roles do not authorize control of this host.

The displayed secret is held only in the mounted panel, cleared on hide,
expiry, disable or close, and excluded from redacted history. Reopening the
section cannot retrieve it. Copying deliberately places it on the system
clipboard; hiding it does not clear the clipboard or revoke the invitation.
Use **Invitations → Revoke** to invalidate an outstanding invitation.

The current remote browser page supports pairing only. Use the separate native
desktop controller for workflow, points and automation status, and confirmed
[workflow/event changes with durable recovery](LIVE-COLLABORATION-CLIENT-COMMANDS.md),
or SDK/CLI with [explicit command metadata](LIVE-COLLABORATION-CLIENT.md#remote-workflow-commands).
The owner UI does not imply that the public endpoint serves the complete app.

## Review delegated definitions

`automation.edit` now permits API/SDK/CLI `automation.create/update` for the
caller's own explicitly disabled behavior drafts. It does not permit replacing
owner or another principal's definitions, claiming their retained deleted IDs,
or enabling/executing a draft. Action drafts cannot change dependencies of owner
or foreign events, including disabled events and missing referenced action IDs.
Uninspectable dependencies or trusted authority fail closed without exposing
hidden event IDs. The controller's separate private proposal editor can submit
disabled definitions for owner review; it cannot adopt or activate them.

Authority lives in a host-owned database sidecar, not record JSON. Ordinary
owner edits, bulk toggles, deletion/recreation and reference cleanup preserve a
delegated origin. A stored enabled flag alone does not make a quarantined draft
executable. Delegated records and references stay blocked from the owner's LIVE
and manual fire/test paths until an explicit authority transfer or a valid
closed delegated approval admits their exact saved closure.

For an intentional owner transfer, open **Automations**, select the saved event
or action and choose **Review saved authority**. Review displays the saved body,
trusted principal/session and every referenced action, independently of any
unsaved editor text. Confirm the explicit owner-authority acknowledgement, then
choose **Adopt with owner authority**. This does not save the open editor draft
or enable automatic execution. Changed source, dependencies or host identity
require a fresh review; the host checks the complete reviewed versions atomically.

The host stores the command identity before dispatch. **Adoption history** stays
available without a selection and after reload. A pending result blocks fresh
adoptions in this flow across owner windows. **Check original outcome** only
queries the existing ID. Explicit uncertainty archival never repeats or undoes
the transfer; the host checks for a newly available result first. Neither source
nor intent IDs are stored in browser storage. Generic advanced API/CLI adoption
continues to have its separate command/audit contract.

The equivalent trusted local control-channel flow is:

1. Read `automation.review {kind,id}` for the exact body, trusted authority,
   generation, dependencies and complete envelope revisions. If any changed,
   reread and review the source;
   do not adopt an unseen newer definition by taking only its latest revision.
2. Submit owner-only `automation.adopt` with the reviewed `kind`/`id`,
   `acknowledgeAuthority:true`, and durable command metadata using that exact
   `expectedRevision`, current host instance and complete `reviewedVersions`
   array. Resolve its original command ID
   after an uncertain response rather than repeating the transfer with a new ID.
3. A Delegated-to-Owner transfer atomically advances the definition's
   version/generation and leaves it **disabled**. Review its dependencies and
   separately enable it through a checked owner action for automatic operation.
   Explicit owner fire/test retains its usual rules after authority transfer.
   Adopting an already Owner definition is a checked no-op and may leave it enabled.

Remote callers cannot read these governance details or perform adoption. Grant
editing, account sign-in and Cloud admin/moderator roles do not adopt a draft.
The separate approval flow below now admits supported core effects while
preserving delegated authority. Broader effect policies remain unfinished. The existing Cloud sync
service remains independent of host behavior authority.

## Approve delegated execution

In the Owner's **Automations** page, select a saved action or event and choose
**Review delegated execution**. This reads the saved definition and every
referenced action together with current permissions and the effect manifest.
It preserves delegated authority; **Review saved authority** remains the
separate adoption/transfer flow.

Review every dependency, including disabled alternatives, the author grants,
rendered fields and effects. Choose explicit enabled states only for the
delegated dependencies you intend to change. Omitted actions keep their saved
state; Owner actions require their ordinary separate controls and a fresh
review. The primary will be enabled. Acknowledge the complete plan, then choose
**Approve and activate delegated execution**.

The current delegated policy supports log, delay and points actions. Unsupported
effects, missing references, unavailable authors or insufficient grants cannot
be overridden by acknowledgement. Existing approvals can still be revoked when
an author is unavailable or the primary has been deleted.

**Approval history** restores the native pending operation and metadata receipts
after reload. **Resolve original outcome** queries the existing command and
never repeats it. Review unknown outcomes explicitly before archiving them;
then read the saved state again and provide new consent. Approval, adoption,
single-definition changes and library transitions share a pending gate across
Owner windows. Saved source, IDs and credentials do
not enter browser storage. See [the detailed contract](LIVE-COLLABORATION-APPROVAL.md).

## Manage profiles and templates

Owner profile creation/switch/deletion, template application, profile import,
record assignment and custom gallery import/deletion now use a native library
coordinator. Review the current host's catalog and full saved graph before
submitting. The host checks both the catalog revision and complete definition
digest; an unrelated new/deleted definition can require a fresh review rather
than silently changing the switch/delete universe.

Template/profile imports stage their exact action/event IDs once. The graph,
pack registration and optional saved gallery documents commit together, including
compound CLI imports. A collision rejects the whole transition. Gallery import
replaces an existing matching ID only under the reviewed catalog version;
the UI can explicitly stage fresh IDs for copies. Importing a replacement pack
leaves its old graph available in Default unless another pack owns it.

Switching rewrites the complete live universe, including already matching
enabled states. Deleting a pack coalesces reference cleanup and prunes its IDs
from other packs, including already absent IDs. Creating a fresh graph removes
its IDs from stale pack memberships before honoring the requested pack or
Default. Existing record assignment never implicitly transfers it from another
pack. These operations preserve trusted delegated authority and invalidate old
approvals when definitions change; they do not authorize execution.

Library history contains one pending operation and bounded metadata receipts.
**Resolve original outcome** only checks that original command; reload and
uncertainty review never resubmit source or regenerate IDs. A pending operation
in any of the four Owner journals blocks fresh native changes. After resolution
or explicit unknown archival, review current state and provide fresh consent.
A manual definition save and subsequent membership assignment are distinct:
if the save is confirmed and assignment fails, keep the record saved and stage
assignment explicitly rather than saving again.

Existing profile/gallery app-state values are copied once from PointsDB into
the AutomationDB catalog without deleting the old values. Legacy reads use the
canonical projection. Typed compatibility writes acknowledge only a committed
catalog/version/outbox change; separate legacy key writes are not a single
transaction. Corrupt legacy data fails visibly without silently dropping packs
or templates. Large workspaces are admitted by byte budgets, with no 512-record
cap on their existing universe; 512 bounds only newly staged graph targets.
See [the exact library/API and recovery contract](LIVE-COLLABORATION-AUTOMATION.md#atomic-owner-library-transitions).

## Manage access

**Approved devices** shows principals and current grants. **Device sessions**
shows the device label, session ID, activity and current connection count.
Edit permissions from either list: the panel fetches that exact principal by
ID, including when it is outside the current principal page. Revisions prevent
an older editor from overwriting a newer grant. Reload the latest grant after
a conflict and make a deliberate choice.

Revocation confirmations identify the scope:

- Session: disconnect and revoke only that session. Other sessions keep access.
- Principal: revoke its sessions and outstanding delegated access.
- Invitation: prevent that invitation's further use or unapproved redemption.
- Disable remote access: stop the remote listener and revoke remote sessions,
  outstanding invitations and pending exchanges. The owner runtime/local LIVE
  remains available. Re-enabling requires new controller approvals.

Configuration changes carry the exact `configurationRevision` loaded with the
draft as `expectedRevision`. The host checks it in the same transaction as the
change. A stale disable therefore cannot revoke access. Status polling never
replaces an edited form. **Load latest configuration** explicitly resets it.
Legacy SDK configuration calls without a precondition remain supported.

## Failures and history

Administration mutations are sent once. If the connection fails after a request,
the panel cannot infer whether the host applied it. It blocks further mutations
until the owner reviews current status/history and explicitly acknowledges the
uncertain outcome. Reads can be refreshed; there is no automatic mutation retry.
Durable administration command deduplication remains a separate pending feature.

Lists use bounded cursor pages. Audit entries contain redacted metadata rather
than private links, bearer tokens, raw settings, scripts or chat. A short audit
page may reflect a response byte budget; pagination continues until an empty
page. Unsafe numeric audit cursors are rejected instead of silently rounded.
If host identity changes, reopen the panel before managing that host.

No settings-panel operation starts another TikTok runtime, publishes a Cloud
record, configures an Internet relay or deploys RemakeBackend. The v1 connection
uses the configured directly reachable host. Complete remote writes, other
domain subscriptions, platform acceptance and real LIVE acceptance are tracked
in the [specification](LIVE-COLLABORATION-SPEC.md).
