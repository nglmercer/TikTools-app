---
title: "Collaboration authority and authentication"
docId: "developer.collaboration"
translationKey: "developer.collaboration"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "cb0c8fc55416a0ade232a0272051d244ed5558d4b1f3aec87d37a7500af59475"
translationRevision: "cb0c8fc55416a0ade232a0272051d244ed5558d4b1f3aec87d37a7500af59475"
order: 190
---

# Collaboration authority and authentication {#collaboration-authority-and-authentication}

## Goal and prerequisites {#goal-and-prerequisites}

Allow an approved device to inspect or perform a narrowly scoped operation on an existing host. Use a test owner profile, a build with remote support and a configured transport. Account login, Cloud team membership, local ownership and a direct delegated session are distinct authorities.

## Establish the trust chain {#establish-the-trust-chain}

Local-owner `collaboration.*` administration owns configuration, invitations, pending requests, principals, grants, sessions and audit. Discover schemas before configuring a listener or creating credentials:

```sh
tiktools api schema collaboration.configure
tiktools api schema collaboration.invites.create
tiktools api call collaboration.status '{}' --format json
```

An invitation initiates pairing; it does not itself grant business authority. Owner approval creates the appropriate session/grants. Direct HTTPS requires validated certificates or explicitly pinned native trust. Native bearer and browser cookie boundaries differ. The initial `collaboration.hello` binds protocol, host instance, principal, session, grant revision, capabilities and negotiated limits.

## Authorize each operation {#authorize-each-operation}

A supported remote method must pass its allowlist, capability, resource restriction and current-session checks. Workflow disable/delete use durable compare-and-set metadata. Points adjustments are additive and have their own grant limits. Remote automation reads are closed summaries; disabled author-owned drafts cannot acquire executable owner authority merely by being saved.

Owner review/adoption and checked approval bind current definitions, dependencies and provenance. A preview manifest reports inspection data, not an execution token. Revalidate grants and resource bindings at execution boundaries rather than treating transport identity as permanent permission.

## Expected result and status {#expected-result-and-status}

Direct native collaboration has implemented scoped reads, selected durable mutations and bounded recovery. Cloud teams/relay additionally have their own adapters; the documented Cloud relay read path must not be presented as full moderation or draft execution. Remaining delegated effects, domain writes and multi-PC acceptance stay experimental. See [remote clients](remote-client.md) and [Cloud](cloud.md) for the selected path.

## Recover and verify {#recover-and-verify}

On revocation or identity change, stop subscriptions and discard private projections. Use `collaboration.subscribe`, `snapshot` and `resume` according to negotiated domains and cursor validity. Resolve the original command ID after an uncertain acknowledgement. Do not retry a delete with a new ID or restore authority from an old receipt. The [authorization source](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-collaboration/src) and registry policy define the boundary; passing TLS alone is not an authorization test.
