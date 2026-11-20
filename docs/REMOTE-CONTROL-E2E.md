# Remote control: two-account end-to-end test

Verified on the local backend (`cargo remake dev`, 127.0.0.1:3000).

Roles:
- **Owner (desktop):** this PC's desktop app, account A. It owns the team, approves requests and serves the team through its relay link.
- **Controller (headless):** a separate workspace, account B. It enrolls its own device key, redeems an invitation and sends commands.

## Steps

1. Owner creates the team and enrolls this PC as its host:
   `teams.hosts.create`, then `teams.devices.enroll` with `kind: host` and `hostId`.
2. Owner starts the relay link: `tiktools.remote-control.host.start` (page action) or `teams.host.start`. Status must reach `ready`.
3. Controller enrolls its device: `teams.devices.enroll` with `kind: controller`.
4. Owner creates an invitation (page action `invitation.create`). The token is shown once; read it with `--secrets-visible`.
5. Controller redeems it: `teams.invitations.redeem` with `{token}`. A pending request appears.
6. Owner approves it (page action `requests.decide`, `decision: approve`, `role: observer`). The grant is `pendingHost` and becomes `active` once the owner's host link confirms it.
7. Controller sends a read command: `teams.commands.run` with `operation: automation.list`, or the controller page action `command.run`. The owner's answer comes back with status `completed`.
8. Owner revokes the grant (page action `grants.revoke`). The controller's next command is refused.

## Expected results (verified)

- Read command completes in under a second and returns the owner's automations.
- A moderation operation (`automation.toggle`) from an observer is refused by the relay.
- After revoke, the controller is refused.

## Known issues

- After revoke, the controller sees "sign in to Cloud again to manage teams" instead of "access revoked". The relay answer is a generic 403; the message mapping should say "no active access".
- A headless workspace must start with `TIKTOOLS_DEV_PLUGINS_DIR` pointing at the staged development plugins to expose the Remote control page there.
- The core unit test `force_refresh_bypasses_the_cache_for_one_read` depends on no plugins being installed in the user's data directory; it fails when SonicBoom is installed there.
