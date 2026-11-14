---
title: "Remote clients and recovery"
docId: "developer.remote-client"
translationKey: "developer.remote-client"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3fa6b05b6c19548fa64d45b3114bfd8080f3f0ed510f835c4d18297d9ccd6cae"
translationRevision: "3fa6b05b6c19548fa64d45b3114bfd8080f3f0ed510f835c4d18297d9ccd6cae"
order: 90
---

# Remote clients and recovery {#remote-clients-and-recovery}

## Goal and prerequisites {#goal-and-prerequisites}

Connect a controller to a host without creating a local LIVE core. You need a host with the relevant remote features enabled, an owner-approved invitation, a reachable HTTPS origin and a supported secure credential store. Read [collaboration](collaboration.md) for grants and [transports](transports.md) for native route choices.

## Pair and inspect {#pair-and-inspect}

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

## Preserve command identity {#preserve-command-identity}

Read a resource with metadata before a supported mutation. Retain its base revision, host instance and durable command ID. If the connection disappears after submission, query `collaboration.commands.get` with that original ID. A receipt is historical; refresh the resource separately. Workflow disable/delete, points adjustments and disabled author-owned automation drafts have different contracts and grants.

## Expected result and status {#expected-result-and-status}

The native desktop has an isolated `tiktools-desktop --controller` entry and profile-bound journals. Controller coverage remains experimental. Pairing, transport connection or a broad feature flag does not authorize arbitrary edits. Remote automation reads expose closed summaries, never editable source; remote enable/fire and local owner adoption remain separate.

## Recover {#recover}

On a sequence gap, use the negotiated subscription snapshot/resume path and clear private projections when identity changes. Do not replay business mutations automatically after reconnect. A changed certificate requires explicit trust review. The [profile implementation](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/profiles/mod.rs) documents profile leases and refresh uncertainty; real multi-PC and OS-store acceptance must be reported separately from loopback tests.

## Operational details {#operational-details}

Apply a reliable event to the matching snapshot/projection before calling `acknowledge`; acknowledging first loses recovery evidence. On reconnect, resume only with the same host instance, subscription, session and grant context. A gap, changed host/session/grant, rejected cursor or browser gap closure requires fresh authorized snapshots, then a new subscription. Do not replay mutations from event history. Use the [metadata and original-command recipe](control-api.md#operational-details) for uncertain writes.

Local `event` envelopes contain `{topic,data?}`; unit variants can omit data. Remote `event.params` is a ReliableEvent with string sequence, event ID, resource version and actor. `event.gap` is a separate method. Example envelopes below are illustrative, not a durable replay of missing effects.

```json
{"jsonrpc":"2.0","method":"event","params":{"topic":"live.status","data":{}}}
{"jsonrpc":"2.0","method":"event","params":{"topic":"workflow.changed","data":{},"hostInstanceId":"h1","subscriptionId":"s1","sequence":"7","eventId":"e7","resource":{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"},"actor":{"principalId":"owner"}}}
{"jsonrpc":"2.0","method":"event.gap","params":{"lost":3,"resync":true}}
```

The SDK integration sketch below requires remote-profiles. The projection callbacks are application-owned; they must finish before acknowledgement. See [RecoveryStream](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/remote/recovery.rs) for typed baseline, cursor and resumption contracts.

```rust
use tiktools_client::remote::{RecoveryMode, RecoveryStream, StreamDomain};
use std::collections::BTreeSet;

let mut stream = RecoveryStream::start(remote, BTreeSet::from([StreamDomain::Workflows])).await?;
replace_snapshot(stream.baseline().expect("complete authorized baseline"))?;
loop {
    match stream.next_event().await {
        Ok(event) => {
            apply_to_projection(&event)?;
            stream.acknowledge(&event)?;
        }
        Err(_) => {
            if stream.reconnect().await? == RecoveryMode::Snapshot {
                replace_snapshot(stream.baseline().expect("fresh authorized baseline"))?;
            }
        }
    }
}
```
