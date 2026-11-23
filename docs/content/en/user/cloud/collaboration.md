---
title: "Prepare collaboration and team access"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "en"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
translationRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
order: 30
---

# Prepare collaboration and team access {#prepare-collaboration-and-team-access}

## Goal and prerequisites {#goal-and-prerequisites}

Grant another operator a defined role while retaining owner control. Remote access is provided by the installable **Remote Control** plugin, not by a Settings tab: you need [Cloud sign-in](account.md), a matching backend and the plugin installed for this host. Start with read-only access and a nonproduction setup.

## Configure remote access {#configure-remote-access}

1. Install and enable `tiktools.remote-control`, then open its **Remote control** page.
2. Create a team and enroll **this PC** as its host with a recognizable device label; start the relay link and wait for its status to reach `ready`.
3. Create an invitation and deliver its one-time token through a trusted channel. An invitation is not an approved active session.
4. Review the incoming request and requested role. Approve only the expected operator; the grant stays `pendingHost` until the host link confirms it.
5. Inspect grants and devices regularly. Revoke a grant when its access is no longer needed.

Delegated definition editing, adopting a definition as owner and approving delegated execution are distinct decisions. Read the saved source, referenced actions and permissions in the specific review. A definition preview alone does not authorize an effect.

## Prepare a controller device {#prepare-a-controller-device}

On the controlling device, sign in with its own account, enroll it as a controller and redeem the invitation token. A pending request then appears for the owner. Nickname lookup needs a separately deployed backend endpoint and may be unavailable on the current service. Do not assume a failed lookup means the person has no account, and do not substitute an arbitrary ID to bypass selection.

## Expected result and status {#expected-result-and-status}

The two-account flow (team, host link, invitation, approval, read command, revoke) has been verified against a local backend. The Settings collaboration/team tabs were removed; the native collaboration service has no web UI in this build. Production relay/NAT behavior, nickname lookup and two-PC LIVE operation are not established here. A ready link alone does not prove end-to-end permission enforcement for your intended action.

## Recovery {#recovery}

For stale grants, revoke access and review it again after the host link refreshes. For unconfirmed commands, inspect the recorded outcome before retrying. Stop the relay link or revoke access when ending a session. Keep invitation tokens and credentials out of public [diagnostic reports](../preferences/troubleshooting.md).

## Operational details {#operational-details}

Every remote command crosses the host's capability gate (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`); the plugin never receives a Control API or a full bearer token. An active transport or team membership alone grants no native access. Observer is read-oriented; moderator adds selected operational controls, automationEditor adds selected editing, and custom requires an explicit reviewed policy. None grants unrestricted scripts or external effects. See [the role and confirmation contract](../../developer/collaboration.md#operational-details) and [transports](../../developer/transports.md) for the current boundary.
