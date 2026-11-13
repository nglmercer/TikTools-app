# Two-PC WebRTC collaboration acceptance

PC A owns LIVE and executes effects. PC B opens the native controller or a
remote CLI profile; it does not create another local LIVE runtime. Cloud
workspace sync shares configuration, while collaboration uses the existing
host's ControlApi, local grants and durable receipts.

## Current procedure

1. Deploy the existing RemakeBackend API with migrations
   `20261112000001_peer_directory.sql`, `20261112000002_rtc_signaling.sql` and
   `20261112000003_relay_device_binding.sql` and
   `20261112000004_native_relay_tunnels.sql`. Use its HTTPS/WSS origin on both
   PCs (`TIKTOOLS_CLOUD_ORIGIN` for a custom instance). No separate Iroh relay
   or PC-specific HTTPS certificate is part of this transport.
2. On PC A, sign in, create/select the team and activate this PC as its host.
   Keep one host runtime running. The normal Cloud supervisor registers the
   device endpoint, confirms understood policies and receives WebRTC offers.
3. On PC B, run `tiktools-desktop --controller`, sign in through its Cloud panel
   and join using the private invitation. Approve the role on PC A and wait for
   host confirmation. The CLI equivalent is
   `tiktools remote cloud pair --profile moderator`; enter invitations privately.
   Initial login still requires the browser callback. An agent can request
   `--kind agent`; the owner must explicitly approve that native client kind.
4. Open the approved profile. New profiles use version 3 and pin the host
   device/key without cached addresses or SDP. Version 2 migrates only after
   admission to the same prior key and RPC identity. The actual selected ICE
   pair supplies the Direct/Relay indicator.
   `tiktools --remote-profile moderator live status` must reach PC A, subject
   to its grant, without constructing a local host on PC B.
   Run `tiktools --json remote check moderator` to capture the selected
   `direct`/`relay` route, connection time and admitted host/session IDs. This
   read-only probe omits credentials, endpoint addresses and permission data.
5. Verify a permitted points adjustment and receipt, retry the same command ID
   and observe one effect. Check rejection for a viewer/resource outside scope.
   First physical acceptance should use the same LAN. The backend operator can
   configure `RTC_STUN_URLS` once; both native clients obtain these URLs through
   authenticated `/api/v3/remote/rtc-config`. The default list is empty. Physical
   Internet NAT traversal between physical PCs remains unverified. The SDK tries
   WebRTC for eight seconds, then opens an end-to-end encrypted WSS relay through
   the same backend. Set `RTC_RELAY_ONLY=true` on the API to force that path for
   diagnostics; return it to false for normal P2P preference.

| Scenario | Required observation |
| --- | --- |
| Cloud unavailable after direct connection | Reads, permitted changes and receipts continue under the existing local lease. |
| Cloud becomes available again | Signaling and metadata recover without duplicating a mutation. |
| Direct route lost | The Desktop coordinator reconnects the same profile with backoff. SDK/one-shot CLI callers reopen it explicitly; recover pending receipts before deciding whether a command is safe to submit. |
| Cloud unavailable during relay | The relay closes. Desktop retries when Cloud and the host return; SDK/one-shot CLI callers reopen the profile. No mutation is replayed during reconnection. |
| Owner revokes locally | Following commands are rejected immediately. |
| Owner revokes through Cloud | Host reconciliation revokes the local grant and closes affected RPC sessions. |
| Calendar corrected or host suspended | The boot-clock offline deadline remains authoritative. |
| Opening a new connection with Cloud unavailable | Opening fails without local-host fallback; an established channel remains independent. |
| OS reboots with Cloud unavailable | Cached authorization cannot be renewed until fresh Cloud validation. |
| Real LIVE running | Both controllers observe PC A and approved effects execute only there. |

## Verified automated evidence

`python3 scripts/collaboration-cloud-lab.py --backend-suite` passes all 26 v3
backend tests against isolated PostgreSQL, including the actual native Cloud
onboarding and normal supervisor/SDK path. Coverage includes version-2 profile
migration, reopening, scoped points/receipts, 22 seconds with HTTPS/WSS
unreachable, recovery, revocation, logout and unsupported policy isolation.
Signaling tests also exercise two backend nodes and a device rotation while an
old socket lease is still present. The backend API already prevents enrolling
two active host devices in the same team.

The client suite passes 167 tests (eight subprocess fixtures ignored), and the
WebRTC adapter passes 29, including fourteen tests of the encrypted relay layer
and native record bridge, and a real local UDP STUN discovery test. The backend
suite now exercises opaque relay records between two nodes, including concurrent
queue bounds. The second native fixture rejects WebRTC signaling and verifies
automatic encrypted WSS fallback, scoped mutations/receipts, profile migration,
Cloud disconnect/recovery and revocation with the real SDK and host. Separate
client tests verify an unanswered eight-second ICE attempt and that forged SDP
never causes fallback. Strict Clippy passes in both repositories; Desktop,
CLI and CLI host mode compile. These checks use one machine and do not prove
physical two-PC/NAT connectivity or a real TikTok LIVE.

The relay fixture also runs the actual Desktop `Controller` coordinator. It
loses Cloud, automatically reconnects to a new connection ID and retains the
same host, principal and session without repeating the points mutation. A host
listener that has not reconnected yet is reported as temporary unavailability;
revoked grants and stale sender leases remain authorization failures. The native
transport additionally cross-compiles for `x86_64-pc-windows-gnu`; Windows runtime
acceptance still requires a real Windows PC.

The same fixture now withholds only controller-directed WebSocket responses
after identifying the outer authenticated `ready` role. Host-directed traffic
and ordinary HTTP continue. The owner observes a durable successful command
receipt while the Desktop journal still has its pending intent; Cloud is then
cut before that response can arrive. After automatic reconnection, the journal
resolves that exact ID as successful with its original points result and the
host total remains unchanged. Inner TLS/RPC content is never inspected by the
proxy, and the fault barrier exists only in the private loopback lab.

## Historical HTTPS/WSS regression checks

The remainder preserves the earlier diagnostic lab and its evidence. It is not
the current user onboarding path or acceptance of the new P2P transport.

### Verified locally on 2026-11-12

- RemakeBackend: all 14 `v3_tests` passed against a fresh disposable pgrust
  cluster. Coverage includes device proof, invitation approval, confirmed grants,
  WebSocket command routing, revocation and commands crossing two relay nodes.
  Those fixtures simulate the host's command replies; they are not an actual
  TikTools host connected through Cloud.
- Native client: `native_proposals_create_and_edit_active_definitions_with_reusable_grant`
  passed. It pairs two independent controllers over local TLS, edits active
  delegated definitions and observes the changed summaries on the other client.
- Native client: `two_native_controllers_keep_family_status_private_and_stop_delete_only_complete_events`
  passed, checking scoped visibility and durable event disable/delete.
- Built the actual `tiktools` executable with `remote-control`. Started a
  separate headless host, configured its gateway and verified HTTPS with its
  generated certificate, without disabling certificate verification.
- Paired the actual CLI with a separate local profile and the OS credential
  store. Approved its invitation through the owner's IPC. A permitted points
  command changed the host, a retry with the same command ID returned the same
  result without applying the delta twice, and `collaboration.commands.get`
  recovered its receipt. A write to another viewer was rejected as `forbidden`.

This is process-level acceptance on one computer. Physical networking, PC B's
credential store, real TikTok input, actual audio/OBS delivery and recovery during
a real LIVE remain unverified. The loopback lab used `/tmp/tiktools-two-pc-host`,
with a controller profile under `/tmp/tiktools-two-pc-controller`; that host
was stopped after verification. The prepared LAN host is running separately at
`https://192.168.18.130:18788`, using `/tmp/tiktools-two-pc-lan` and a seven-day
certificate. This address was checked from PC A only, not across the firewall
from a second machine. No TikTok LIVE is connected in this isolated lab.

Its public certificate SHA-256 is
`dab9743e6250438d564e0807ed8a62923f6744deb7a30d49d95fa38eb42d7630`.
The private invitation is retained locally; use `invite` again if it has
expired. To inspect this already prepared host, run:

```bash
python3 scripts/collaboration-lab.py --directory /tmp/tiktools-two-pc-lan status
python3 scripts/collaboration-lab.py --directory /tmp/tiktools-two-pc-lan check
```

### Legacy lab: prepare PC A

Requirements: Python 3, OpenSSL, and a native build for each PC's OS. The CLI
needs `remote-control` to serve; Desktop enables it by default. Build on PC A:

```bash
cargo build --locked -p tiktools-cli --features remote-control
# Required only if using the Desktop host/controller:
cargo build --locked -p tiktools-desktop
```

The helper uses a dedicated data home, empty plugin directories and a separate
IPC identity. It does not configure your normal application instance. Replace
`192.168.18.130` with PC A's reachable interface address. Use a new directory:

```bash
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab prepare --address 192.168.18.130 --port 8788
```

Keep the next command running in its own terminal. Choose **one** host:

```bash
# For actual TikTok LIVE acceptance, use the Desktop host:
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab desktop
# Or, for API-only acceptance, use the headless host instead:
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab host
```

From a second terminal:

```bash
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab configure
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab check
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab invite
```

`configure` waits for `gateway.running`; `check` performs a verified HTTPS
request to `/remote/v1/info`. `invite` saves a single-line private invitation
file, compatible with CLI `--invite-stdin`. It never prints its secret. Transfer
only that invitation privately to PC B. Keep PC A's private key on PC A.
The fingerprint is public and should be compared directly between the PCs.
On Windows pass `--binary C:/path/to/tiktools.exe` before the subcommand.

The initial grant allows LIVE status, scoped workflow reading/deactivation,
scoped points reading/adjustment and command receipt lookup. Workflow/viewer
permissions are restricted to `two-pc-lab`, with a maximum delta of 10. This
does not grant active automation authoring or arbitrary effect execution.
Configure those separate reusable grants explicitly in the owner UI when
testing the advanced automation scenario.

### Legacy lab: pair PC B

Use the controller UI to create a profile, or use the CLI below. The CLI and
Desktop must use the same user/data home to share the profile; choose a
dedicated `TIKTOOLS_HOME` for the lab on PC B as well. Credentials use the OS
credential store; unavailable/locked stores must be fixed rather than replaced
with plaintext tokens.

```bash
tiktools remote pair --url https://192.168.18.130:8788 --profile two-pc-lab --fingerprint SHA256_FROM_PC_A --kind desktop --invite-stdin < invitation.json
```

The redirect example uses a POSIX shell. With another shell, use the hidden
invitation prompt by omitting `--invite-stdin` and enter the one-line JSON.
While PC B waits, inspect and approve its exact request on PC A:

```bash
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab requests
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab approve REQUEST_ID
```

Then, on PC B:

```bash
tiktools --remote-profile two-pc-lab live status
tiktools --remote-profile two-pc-lab api discover
tiktools-desktop --remote-profile two-pc-lab
```

The native controller is supported by the normal Desktop build. Serving the
browser controller instead requires the separate `remote-browser-app` build
and a built web bundle; the prepared headless lab does not include it.

### Legacy lab: acceptance checklist

1. PC B reaches PC A over TLS and displays the same authenticated host ID.
   Configure network/firewall access to the chosen port as appropriate. For
   Internet testing the address must be reachable across networks; the direct
   gateway does not perform NAT traversal. Cloud relay transport exists, but
   its app-side mutation bridge still needs implementation.
2. Approving one device does not approve another. An observer cannot mutate.
3. On PC A, create a disposable workflow/viewer with ID `two-pc-lab`. PC B sees
   allowed updates. An adjustment of +3 appears once on PC A. Reusing the
   original command ID does not add another +3. Use the metadata-aware API or
   the controller form, not a raw mutation without command metadata.
4. Two controllers observe the same committed revisions. Concurrent edits
   produce a conflict or refresh rather than silently overwriting changes.
5. Disconnect PC B, modify PC A, reconnect and verify recovery. Revoke PC B's
   grant and verify that reads/writes and cached views are invalidated.
6. Connect the Desktop host to an actual LIVE. Verify activity on PC B, one
   execution of each permitted effect on PC A, and no second TikTok runtime
   or effect execution on PC B. Test each intended effect independently.
7. For active automation editing, grant the intended event/action resources,
   reusable delegation and exact effects. Test a permitted edit and then
   revoke its effect grant; the next real event must no longer execute it.

Never count the physical two-PC/LIVE steps as passed from the local fixtures.

### Legacy lab: stop and repeat

```bash
python3 scripts/collaboration-lab.py --directory /absolute/path/pc-a-lab stop
```

`stop` shuts down the dedicated host. Its data and certificate remain for
inspection; no files are removed. `prepare` refuses a nonempty directory.
Use a new empty directory when changing the advertised IP/certificate.

Focused automated checks in this app:

```bash
cargo test --locked -p tiktools-client --features remote-profiles --lib native_proposals_create_and_edit_active_definitions_with_reusable_grant
cargo test --locked -p tiktools-client --features remote-profiles --lib two_native_controllers_keep_family_status_private_and_stop_delete_only_complete_events
```

In RemakeBackend, use a disposable administrative database with `CREATEDB`:

```bash
cargo test --locked -p cloud-api v3_tests:: -- --ignored --test-threads=2
```

That command requires `CLOUD_TEST_DATABASE_URL`. Tests create/drop their own
private database clones. The local run used a fresh temporary cluster with
generated credentials and stopped it afterward. The pre-existing backend
`.env` credentials failed authentication against its existing cluster;
`devtools.toml` is overridden by `.env`'s `DATABASE_URL`. Existing credentials,
data and configuration were not rewritten to make the tests pass.
