---
title: "Prepare collaboration and team access"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "en"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4a97f64b6bdc818ebb8d16eec1e67d95a2d0eae0b75f7cfd5c1493bf27244684"
translationRevision: "4a97f64b6bdc818ebb8d16eec1e67d95a2d0eae0b75f7cfd5c1493bf27244684"
order: 30
---

# Prepare collaboration and team access {#prepare-collaboration-and-team-access}

## Goal and prerequisites {#goal-and-prerequisites}

Grant another operator a defined role while retaining owner control. You need a capable desktop host and a deliberate choice between local/native collaboration and account-based Teams. Teams also needs [Cloud sign-in](account.md) and matching deployed service support. Start with read-only access and a nonproduction setup.

## Configure owner access {#configure-owner-access}

1. Open **Settings → Collaboration**. Inspect connection status and enable/configure the intended listener or gateway through its controls.
2. Review allowed origins and the access matrix before creating an invitation. Choose only the areas, operations and resources the operator needs.
3. Create an invitation and deliver its private link/token through a trusted channel. An invitation is not the same as an approved active session.
4. Inspect pending requests and the requested grant. Approve only the expected operator, then confirm their visible identity and connection.
5. Use the access inspector to review later grant changes. Revoke invitations, identities or sessions when their access is no longer needed.

Delegated definition editing, adopting a definition as owner and approving delegated execution are distinct decisions. Read the saved source, referenced actions and permissions in the specific review. A definition preview alone does not authorize an effect.

## Prepare an account-based team {#prepare-an-account-based-team}

When **Settings → Teams** is offered, create a named team and select it. Enroll **this PC** as a host with a recognizable device label, then use **Start serving** and inspect its link state. Creation and local-host enrollment are wired in the frontend; they are separate from inviting a teammate. A controller device has its own enrollment role.

Choose a recipient, role and invitation lifetime, then review incoming requests, members and devices. The nickname search flow requires a separately deployed backend endpoint and may be unavailable on the current service. Do not assume a failed search means the person has no account, and do not substitute an arbitrary ID to bypass selection.

## Expected result and status {#expected-result-and-status}

Owner access controls and frontend team creation/enrollment exist. The complete remote collaboration workflow is experimental: real two-PC LIVE operation, production nickname lookup, NAT/relay behavior and every platform/effect recovery path are not established here. A ready link alone does not prove end-to-end permission enforcement for your intended action.

## Recovery {#recovery}

For stale grants or host revisions, reload and review again. For unconfirmed commands, inspect the recorded outcome before retrying. Stop serving or revoke access when ending a session. Keep invitation tokens and credentials out of public [diagnostic reports](../preferences/troubleshooting.md).

<DemoFrame scenario="collaboration" locale="en" title="Demo: collaboration" />

## Operational details {#operational-details}

After joining or approving Teams access, the host must confirm the current native permission revision. `pendingHost` means this is still pending; an active transport or team membership alone grants no native access. Reload after a permission change and recheck host/device status. Observer is read-oriented; moderator adds selected operational controls, automationEditor adds selected editing, and custom requires an explicit reviewed policy. None grants unrestricted scripts or external effects. See [the role and confirmation contract](../../developer/collaboration.md#operational-details) if available controls do not explain a denial.
