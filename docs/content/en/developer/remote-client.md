---
title: "Remote clients and recovery"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9c02df03cbaef36512591b4dd92e4713db47f3f317917dbef4191bc598bd3cdc"
translationRevision: "9c02df03cbaef36512591b4dd92e4713db47f3f317917dbef4191bc598bd3cdc"
order: 90
---

# Remote clients and recovery

## Goal and prerequisites

Connect a controller to a host without creating a local LIVE core. You need a host with the relevant remote features enabled, an owner-approved invitation, a reachable HTTPS origin and a supported secure credential store. Read [collaboration](collaboration.md) for grants and [transports](transports.md) for native route choices.

## Pair and inspect

These commands describe a manual test environment; replace the example origin with its configured host:

```sh
tiktools remote inspect --url https://host.example
tiktools remote pair --url https://host.example --profile live-desk
tiktools --remote-profile live-desk api discover
tiktools --remote-profile live-desk live status
tiktools --remote-profile live-desk workflow list
```

For a self-signed host, compare its fingerprint with the owner through an independent channel and pass `--fingerprint` during pairing. Inspection sends no authenticated business request and does not approve trust. Invitation and credential values are entered through the supported prompts, not command-line secrets.

Profile files contain identity, trust policy and an opaque secure-store reference. Linux uses Secret Service, macOS Keychain and Windows Credential Manager. An unavailable store fails explicitly; Android has no selected persistent-profile backend. The SDK can use explicitly supplied in-memory credentials. `remote forget` removes local credentials and metadata but does not revoke the host's session.

## Preserve command identity

Read a resource with metadata before a supported mutation. Retain its base revision, host instance and durable command ID. If the connection disappears after submission, query `collaboration.commands.get` with that original ID. A receipt is historical; refresh the resource separately. Workflow disable/delete, points adjustments and disabled author-owned automation drafts have different contracts and grants.

## Expected result and status

The native desktop has an isolated `tiktools-desktop --controller` entry and profile-bound journals. Controller coverage remains experimental. Pairing, transport connection or a broad feature flag does not authorize arbitrary edits. Remote automation reads expose closed summaries, never editable source; remote enable/fire and local owner adoption remain separate.

## Recover

On a sequence gap, use the negotiated subscription snapshot/resume path and clear private projections when identity changes. Do not replay business mutations automatically after reconnect. A changed certificate requires explicit trust review. The [client implementation guide](https://github.com/nglmercer/TikTools-app/blob/remake/docs/LIVE-COLLABORATION-CLIENT.md) documents profile leases and refresh uncertainty; real multi-PC and OS-store acceptance must be reported separately from loopback tests.
