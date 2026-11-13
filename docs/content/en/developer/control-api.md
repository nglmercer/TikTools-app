---
title: "Control API contracts and discovery"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
translationRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
order: 70
---

# Control API contracts and discovery {#control-api-contracts-and-discovery}

## Goal and prerequisites {#goal-and-prerequisites}

Call the host's published contract without inventing method names or response schemas. Use a running private host and the matching CLI. The canonical [Rust registry](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs) describes methods; registration modules provide their schemas, capabilities, desktop requirements and concurrency flags. Discover the actual build before choosing an operation.

## Inspect, then call {#inspect-then-call}

```sh
target/debug/tiktools api discover
target/debug/tiktools api schema system.info
target/debug/tiktools api call system.info '{}' --format json
target/debug/tiktools api verify --sandbox
```

The last command uses an isolated verification runtime. It does not validate a real LIVE connection. For local stdio/IPC the wire is line-delimited JSON; remote WSS carries one message per frame and first requires `collaboration.hello`.

```json
{"jsonrpc":"2.0","id":"info-1","method":"system.info","params":{}}
```

A response echoes the ID and contains `result` or `error`. Errors use a string `code`, a message and optional safe `data`; do not assume the numeric error codes of an unrelated JSON-RPC server. Notifications use `method:"event"` and `params:{topic,data}`. Keep notifications separate from pending response resolution.

## Preserve metadata {#preserve-metadata}

Versioned calls can carry top-level `meta.expectedRevision`. Durable calls also bind a command ID and host instance according to discovery. Read `meta.resourceVersions`, `meta.command` and `meta.hostInstanceId` alongside the domain result. Revisions are decimal strings, not JavaScript numbers. The frontend's `callWithMeta` helper in `src/web/shared/api/control-contracts.ts` preserves this envelope.

## Expected result and status {#expected-result-and-status}

A discovered method is supported by that host build; remote permission and resource restrictions still apply. Local-owner methods are not implicitly remote methods. Generic requests retain the same operation path as typed SDK wrappers. Shared technical references should derive from registry/schema sources rather than a second manually maintained method table.

## Recovery {#recovery}

On `method_not_found`, compare discovery and client/host versions. On `revision_conflict`, retain the candidate and compare current versions. A transport timeout may leave an accepted mutation running; query its original retained command ID where supported. Never create a fresh ID merely to bypass uncertainty. Continue with [agents](agents.md) or [remote recovery](remote-client.md).

## Versioned method reference {#method-reference}

Generated from the checked-in discovery snapshot; the running host remains authoritative.

<!--@include: ../../../.generated/control-api.en.md-->

## Operational details {#operational-details}

Use a disposable `TIKTOOLS_HOME`, clear inherited path overrides as described in [storage](storage.md), start `tiktools host --ipc`, and use the same home in the controlling terminal. The recipe assumes an existing disposable workflow `docs-example`; create the disabled sample in [automations](automations.md#operational-details) first. Capture the read’s `meta.resourceVersions` revision and `meta.hostInstanceId`; replace `REVISION`/`HOST_INSTANCE` below. Keep one stable command ID with its exact payload. On timeout, query the original ID, then refresh the resource; `accepted`, `running` and `unknown` do not authorize a new effect. A revision conflict requires a fresh read and review, not automatic overwrite. `points.adjust` is additive: it accepts a command ID but no expected revision.

`--include-meta` preserves the domain result and metadata; default output extracts the result. `--dry-run --explain` validates/plans without executing; explanations go to stderr and JSON results to stdout. Destructive calls require `--confirm`. Default output redacts recognized secrets; `--secrets-visible` only changes CLI rendering and cannot recover values already hidden by the host. Review output manually. `api events` is bounded by timeout/count and uses topic filters. Live discovery remains authoritative for features and permissions; generated contract descriptions below retain the registry’s exact text.

```sh
tiktools api discover --json
tiktools api schema workflows.disable --json
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --explain --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --json
tiktools api call collaboration.commands.get '{"commandId":"docs-disable-001"}' --json
tiktools api call points.adjust '{"uniqueId":"docs-viewer","delta":1}' --command-id docs-points-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --json
tiktools api events --topics live.status --timeout 5 --max-events 2
```

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.disable","params":{"id":"docs-example"},"meta":{"expectedRevision":"1","commandId":"docs-disable-001","hostInstanceId":"HOST_INSTANCE"}}
```

```json
{"jsonrpc":"2.0","id":1,"result":{"id":"docs-example","enabled":false},"meta":{"hostInstanceId":"HOST_INSTANCE","resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}],"command":{"commandId":"docs-disable-001","state":"succeeded"}}}
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":"revision_conflict","message":"Resource revision changed","data":{"resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}]}}}
```
