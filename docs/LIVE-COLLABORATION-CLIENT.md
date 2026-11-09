# Native remote client and CLI profiles

Status: experimental native HTTPS/WSS SDK and CLI, 2026-11-08. The host
permits scoped reads, workflow/event disable/delete, disabled automation drafts
and additive points commands in
[the transport guide](LIVE-COLLABORATION-TRANSPORT.md).
The desktop has isolated workflow/points controls and a summary-only automation
board with durable whole-event disable/delete confirmation. A separate native
JSON editor retains private author copies and submits disabled proposals through
the shared durable coordinator. Complete domain UI, activation and delegated
execution remain incomplete. See the
[native proposal editor](LIVE-COLLABORATION-CLIENT-COMMANDS.md#native-automation-proposal-editor)
and the metadata-aware SDK/CLI
[automation proposal contract](LIVE-COLLABORATION-CLIENT-COMMANDS.md#automation-proposals-through-the-sdkapi).

## Build and connect

`tiktools-client` exposes optional `remote` and `remote-profiles` features.
The CLI enables `remote-client` by default; minimal builds can enable that
feature explicitly. Serving a remote host is the separate `remote-control`
feature. Selecting a remote profile never starts an `AppCore`, scans plugins,
connects to TikTok or opens host domain databases.

Prepare the owner host using [owner settings](LIVE-COLLABORATION-OWNER.md) or
[the CLI TLS setup](LIVE-COLLABORATION-TRANSPORT.md).
Obtain a one-use invitation from the owner and pair on the controller:

```bash
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
tiktools --remote-profile live-desk api events --topics workflow.changed
```

Pairing reads invitation JSON or the full join link through a hidden terminal
prompt. `--invite-stdin` accepts one bounded line from a private pipe or secret
provider. Invitation secrets and access/refresh credentials have no command-line
argument. JSON contains `inviteId`, `secret`, and optional `expiresAt`; join links
must belong to the selected HTTPS origin and use
`/remote/v1/join#inviteId=...&secret=...`. The owner approves the displayed request
ID. Client kind defaults to `agent`; `--kind desktop` is also available and
grants no additional authority.

Existing JSON/output, dry-run, precondition and destructive-confirmation rules
remain in effect. Confirmation never replaces host permission.
`--remote-profile` rejects `--standalone`, host/stdio serving, the local web
adapter, profile-management commands and the local `api verify` suite. Invalid
or missing profiles fail without IPC/direct/local-runtime fallback.

## Remote workflow commands

The host accepts `workflows.disable` under `workflows.toggle` and
`workflows.delete` under `workflows.edit`. Both also require `workflows.read`
and permission for that exact workflow ID. The owner settings grant editor
exposes these permissions explicitly; reviewer presets remain read-only.
Saving or activating a workflow is still denied, including when the grant
contains edit/toggle capabilities: delegated execution policy remains required.
The desktop controller has confirmed disable/delete controls and a native durable
command journal. See [desktop command recovery](LIVE-COLLABORATION-CLIENT-COMMANDS.md).
SDK/CLI callers manage their explicit command metadata as below.

Obtain the current `hostInstanceId` from the authenticated `collaboration.hello`
result and the exact workflow revision from a metadata-aware read. Choose a new
command ID once, save it before sending, and include all three metadata fields:

```bash
tiktools --remote-profile live-desk api call collaboration.hello '{"protocolVersion":1,"clientVersion":"cli","clientKind":"agent"}' --format json
tiktools --remote-profile live-desk api call workflows.get '{"id":"welcome"}' --include-meta --format json
tiktools --remote-profile live-desk api call workflows.disable '{"id":"welcome"}' --command-id disable-once --host-instance-id HOST_INSTANCE --expected-revision REVISION --include-meta --format json
tiktools --remote-profile live-desk api call workflows.delete '{"id":"welcome"}' --command-id delete-once --host-instance-id HOST_INSTANCE --expected-revision REVISION --include-meta --confirm --format json
tiktools --remote-profile live-desk api call collaboration.commands.get '{"commandId":"disable-once"}' --format json
```

These are examples; replace instance/revision/command IDs with values bound to
the selected host and intended operation. Ordinary typed mutation wrappers do
not attach metadata automatically; use `call_with_meta`/`call_value_with_meta`
with `RequestMetadata` in the Rust SDK. Deactivation returns the bounded remote
payload `{id,enabled:false}`, deletion `{id,deleted}`. Revisions and the durable
receipt are in response metadata. Owner deactivation keeps its legacy payload.

After a lost acknowledgement, query the existing command ID under
`commands.read`. A retry with the same semantic request/ID returns the original
outcome, even if the owner later re-enabled or recreated the resource; it does
not apply the command again. Changing the method, resource or expected revision
under that ID fails. A different principal has a different command namespace.
Expired results remain unknown and their IDs cannot execute again. There is no
automatic mutation retry or inference that a timeout means rejection.

The host rechecks current session/capability/resource access after its writer
gate and again after SQLite writer admission, including for a retained replay.
Queued work loses permission when its grant/session is revoked before that
admission. Responses and revision-conflict metadata are checked again before
network output. An already committed mutation remains committed if the reply
is later withheld because permission changed. New commands from an old host
instance fail; historical duplicates remain queryable under current authority.

Scoped points reads and additive adjustments are also available through the
generic SDK/CLI metadata API. `points.adjust` requires command/instance IDs and
rejects `expectedRevision`; point outcomes are queried by their original ID.
The SDK also recovers points and mixed-domain streams. The native desktop
journal supports workflow and typed points intents, including version-1
workflow history migration. The desktop provides a bounded points board, exact
viewer lookup and host/unit-bound adjustment confirmation. Its display coalesces
absolute committed reads independently of reliable invalidations and refreshes
after reconnect. See [points commands and recovery](LIVE-COLLABORATION-POINTS.md).

## Desktop controller

The desktop enables `remote-client` by default, independently of its host's
`remote-control` feature. Start a separate controller window:

```bash
tiktools-desktop --controller
tiktools-desktop --remote-profile live-desk
```

The owner window's **File → Connect to another PC** opens that same controller
entry point in a separate process. Starting without a controller option keeps
the existing local-owner startup. Controller selection is parsed before
logging, installer handling and the owner's single-instance lock. Invalid
mixed options fail before creating files. A controller initializes its own
logging, Tokio, Winit/Wry and the native profile engine. It constructs no
`AppCore`, local IPC listener/router, TikTok client, plugin scanner, media host,
tray or plugin asset protocol. A failed remote profile never starts a local host.

The packaged Vue entry mounts a separate controller root without the owner
router/stores. It offers saved profiles and private invitation entry, explicit
pin confirmation and owner approval, followed by a persistent host/connection
indicator, effective capabilities, permitted workflow view, points board and
closed automation status with confirmed whole-event stop/delete. Tokens and
the pairing exchange secret remain in Rust/OS storage. The invitation input
is cleared on submission; pending approval survives page reload in the native
engine. The visual proposal builder can prepare a disabled points action for
an exact literal viewer ID and a bounded nonzero adjustment. Preparing it only
replaces the open local draft after preview; it neither saves, submits nor
adjusts points. Submission and owner approval remain separate, and current
host permissions and effect limits still govern execution. Dynamic viewer
targets are not offered by this builder.

When preparing a disabled event, the builder can search the current permitted
host action summaries and copy an action ID into the local draft. It renders at
most 50 matches and permits manual IDs for actions not in the visible snapshot.
The list is informational: host access and dependencies can change, and the
owner still reviews the saved source before a separate activation.

The LIVE activity panel can filter its bounded, locally displayed
entries by kind and search author, nickname or text. Filters do not query the
host or extend its retained history, and are cleared when the connection or
grant changes or LIVE access is lost. With points access, the moderator may
look up an activity author's points directly from that entry. The shortcut is
read-only, requires the viewer grant, displays the exact lookup ID, and clears
any previous adjustment amount when the selected viewer changes. It is offered
only if the native projection retained the complete author ID unchanged and
the value is not the ambiguous `viewer` fallback. An unavailable or ambiguous
ID must be looked up manually. The shortcut does not verify a provider identity
or authorize an adjustment; adjusting points still requires a separate review
and confirmation.
A saved profile can be forgotten after explicitly confirming removal
of its local credentials, drafts and command history. The selected profile
must first be disconnected, and a pending or actively used command journal
blocks removal. The confirmed profile identity is rechecked under the native
profile lease, so an old confirmation cannot remove a replacement with the
same name. This removes local data only: direct SDK connections already opened
may remain active until they expire or the owner revokes that session on the
host. Certificate rollover still uses CLI `remote trust`.

The native `remote::controller::Controller` owns bounded workflow and automation
projections with independent automation display revisions.
It acknowledges reliable events after applying authorized current reads and
resource revisions. Desktop pages are bound to selection epoch and view
revision; profile switching discards prior host data and fences late replies.
Data is unavailable while disconnected/recovering. Stream recovery replaces
the baseline when needed; permissions are renegotiated when grants change,
including connections with no domain stream. The native bridge forwards
permitted reads under the remote identity and never invokes a local-owner
API or legacy desktop command. Mutations use the native journal coordinator;
raw forwarding cannot bypass it.

Local workflow edits remain separate from authoritative host state, keyed by
origin, host, principal and session. Unsaved edits stay in memory. **Save draft
on this PC** explicitly confirms storing the complete text, including any
sensitive values it contains, in the native private profile. Saved drafts
survive page reload and controller/profile reopen. This is a local file, not
Cloud Sync, browser localStorage or a host command. See the
[draft storage contract](LIVE-COLLABORATION-CLIENT-COMMANDS.md#local-drafts-and-conflict-comparison).

When the host revision changes, the controller shows the current host text and
retains the draft's text and original base revision. Compare and edit the local
text, then explicitly choose **Use the current revision for this local merge**
to change its base. That local merge is unsaved until separately confirmed.
**Discard draft** removes the saved copy if present and returns to the host
value; deleting only the saved copy keeps the current in-memory text. Reviewing
a saved copy never replaces a newer unsaved edit. Changed or unconfirmed local
writes require a saved-copy review before another replacement; they are not
automatically retried. Switching profiles fences earlier replies. Deleted-host
drafts remain accessible only under the current workflow read grant; grant
removal hides them without deleting their private local file. The editor still
has no host-save action: definition save/enable requires delegated-effect policy.

Linux tests exercise real TLS projection/recovery and the real packaged Vue
root through Wry across three idle page generations. An additional real
Vue/Wry/native-journal/TLS-host test confirms disable/delete with an intervening
page reload, using isolated in-memory credentials. This is not an
end-to-end two-desktop/OS-keychain/LIVE acceptance result. See
[the validation record](LIVE-COLLABORATION-VALIDATION.md). Other domain stores,
the full controller shell,
desktop trust/profile management and cross-platform UI acceptance remain open.

## Trust and host binding

Default TLS validates operating-system trusted roots. For an owner-verified
self-signed certificate, inspect its presented fingerprint:

```bash
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk --fingerprint SHA256_HEX
```

Inspection is **unverified discovery**: its verifier captures the leaf hash and
rejects the TLS handshake. It sends no HTTP request or credential and approves
nothing. Compare the fingerprint independently with the owner before explicitly
supplying it. The pin is SHA-256 of the complete leaf DER certificate. Pinned
connections also validate dates, endpoint name, server usage and TLS handshake
signatures; the approved leaf replaces CA-chain trust.

Endpoints are bare HTTPS origins without userinfo, paths, queries or fragments.
HTTP redirects, automatic HTTP retries, ambient proxies and cookies are disabled.
Native pairing/refresh use the native marker; WSS uses a sensitive Authorization
header. Public host identity is checked against the stored ID before loading
credentials; hello then verifies host, principal and session on the connection.
Public fingerprint hints never update trust automatically.

```bash
tiktools remote trust live-desk --fingerprint NEW_SHA256_HEX --confirm
```

Trust changes require explicit confirmation, a valid new certificate and the
existing host ID. A different host identity requires new pairing.

## Profile and credential persistence

```bash
tiktools remote list
tiktools remote show live-desk
tiktools remote forget live-desk
```

Profile JSON contains origin, host/profile/principal/session IDs, client kind,
trust policy and an opaque credential-store key. It never contains tokens.
Storage is `TIKTOOLS_REMOTE_PROFILES_DIR`, `<TIKTOOLS_HOME>/remote-profiles`, or
the platform's local TikTools data root. Overrides must be absolute. Linux
directories/files are private to the current user; symlink paths are rejected.
The catalog caps profiles at 32 and rejects duplicate stored copies of one
approved session, avoiding independent refresh copies of the same token.

The production store directly selects Secret Service on Linux, Keychain on
macOS and Credential Manager on Windows. No mock, memory or file fallback is
selected silently. Blocking OS operations run off Tokio/UI workers.
Android/Termux and other targets without a selected backend return an explicit
credential-store error for persisted profiles. SDK callers can explicitly keep
`NativeCredentials` in process memory for the remote transport; this
does not create a persisted CLI profile. `forget` deletes local credentials and
metadata and explicitly reports that the host session was not revoked. The
owner can revoke it through local `collaboration.sessions.revoke`; raw SDK
callers also have native HTTP logout. Other approved devices stay independent.

Opening a profile takes an exclusive process lease. A fresh access credential
is reused; one within 30 seconds of expiry is rotated once. Before refresh,
the client fsyncs a nonsecret attempt ID. After the reply it saves the new
credential generation in the OS store, then clears the attempt. A matching
saved generation repairs a crash between those steps. An uncertain prior
refresh without its new generation returns `reauthentication_required`; the
old one-use refresh token is never resent. Caller cancellation leaves the
bounded open task running to persist a known reply; process death uses the
durable intent fence. Repair may require a new invitation.

Filesystem and crash-boundary evidence is Linux-only. Tests inject an explicit
credential-store seam; real OS-store, power-loss and other-platform acceptance
remain open. Tokens are not exported into files for tests.

## SDK calls, events and recovery

`TikToolsClient::Remote(RemoteClient)` preserves the typed methods and metadata
envelopes used by direct/IPC clients. Use `ProfileManager::open` for persistence
or `RemoteClient::connect` for explicitly supplied in-memory credentials.
Both negotiate hello before ordinary RPCs. Admission, queues, frame/parameter
budgets and write/response deadlines are bounded. Concurrent replies are
correlated without converting calls into local-owner execution. Lost
acknowledgements fail once; domain RPCs are never automatically replayed.
Resolve the existing command ID before deciding whether to submit anything else.

`RecoveryStream::start(remote, domains)` subscribes locally before requesting a
server stream and all snapshot pages. It checks identity, versions, total bytes
and a stable watermark before becoming ready. Current domains are `workflows`,
`points`, `automation` and caller-owned workflow/viewer/automation `commands`;
the latter also needs
`commands.read`. Points-only sessions need no workflow capability. Mixed pages
match versions by kind/scope/ID, including when a workflow and viewer share an ID
or automation events/actions have equal raw IDs. Automation baselines retain
only validated closed summaries; they never restore definition text.
The initial `baseline()` is an immutable baseline, not a continuously updated
cache. The application owns host view state and local drafts.

Apply each `next_event()` and call `acknowledge(&event)` after application
succeeds. For invalidations, fetch the current resource through its authorized
read and apply resource versions monotonically. A newer read can subsume older
invalidations, but reliable cursors still advance through acknowledgement.
Sequences remain decimal strings, including above JavaScript's safe integer range.

`reconnect()` opens only the selected remote profile. `RecoveryMode::Resumed`
preserves application view state and replays unacknowledged events with original
IDs/sequences. `RecoveryMode::Snapshot` supplies a new baseline: replace host
view state and preserve local drafts separately. Baselines are hidden while
disconnected/recovering. Server gaps or changed permission epochs invalidate
the old baseline/cursor. Drafts are never discarded or resubmitted here.

CLI `api events` emits gap markers and reconnects with a 500-ms to 30-second
exponential delay plus bounded jitter. It never replays domain calls.
`--timeout` and `--max-events` bound the stream; gaps count as output events.
Other CLI calls make one connection/attempt and return failures. Unsupported
domains/feed are reported explicitly.

References: [native SDK](../crates/tiktools-client/src/remote/),
[CLI profile commands](../crates/tiktools-cli/src/remote.rs), and
[validation and remaining gates](LIVE-COLLABORATION-VALIDATION.md).
The verifier follows the [rustls signature contract](https://docs.rs/rustls/0.23.46/rustls/client/danger/trait.ServerCertVerifier.html);
OS storage uses [keyring 3.6.3](https://docs.rs/keyring/3.6.3/keyring/).
