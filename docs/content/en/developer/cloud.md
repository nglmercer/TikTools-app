---
title: "Cloud contracts, synchronization and review"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a8e695f0431f934e989dca5460a6338a3a24e04acf0d71b66b7c9050bd4ce839"
translationRevision: "a8e695f0431f934e989dca5460a6338a3a24e04acf0d71b66b7c9050bd4ce839"
order: 180
---

# Cloud contracts, synchronization and review

## Goal and prerequisites

Develop account configuration and catalog features against the provisioned Cloud instance. Local-only operation remains valid. For authenticated work, use a dedicated test account/backend and the native session handoff; this documentation does not establish production bearer or two-device acceptance.

The verified contract owner is [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend), with [v2 configuration contracts](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md) and additive [v3 surfaces](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md). Backend-owned portable types and client transport are represented locally in `crates/cloud-contracts` and `crates/cloud-client`. Desktop durability and authentication remain host responsibilities.

## Gate and stage operations

1. Discover native methods with `rpc.discover`; an older host can retain legacy views.
2. Let the host check the instance's `/api/v2/capabilities` before using its gated feature endpoint. Never hardcode an advertised server capability into a browser-only flag.
3. Use the operator-provisioned origin. Release builds do not accept arbitrary runtime origin replacements.
4. Preview migration and conflicts, retain frozen bases and uncertain writes, and submit only the reviewed operation.

```sh
tiktools api discover
tiktools api schema sync.v2.migrate
tiktools api schema globals.typed.replace
```

These inspect schemas; they do not migrate an account or save variable values.

## Current implementation

V2 has native state/store/coordinator, guarded v1 migration, conflict review, rebuild and installed-plugin reconciliation paths. Adoption is explicit and account scoped; unmarked legacy installs remain on their legacy path. Full two-Desktop acceptance and some product adapters remain incomplete. The account-wide write-epoch setup has irreversible server consequences explained by its migration review.

V3 adds catalog, teams, integrations, assets, overlays and package surfaces behind capabilities. Catalog metadata does not replace signed plugin verification. Template/profile import uses the owner library journal and remains disabled. Package application records interrupted steps rather than replaying them automatically. Cloud relay read access must not be confused with direct collaboration mutation authority.

## Expected result and recovery

Configuration changes retain identity, revision and review boundaries. Automations Changes is local-only; reviewed Variables uses real typed native methods and does not sync raw values. Overlay Changes is not merged. On conflict, compare base/local/fresh Cloud records; on unknown acknowledgement, resolve the retained intent. A retryable HTTP error never authorizes repeating a mutation. [Collaboration](collaboration.md) covers running-host authority separately.
