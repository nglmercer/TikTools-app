---
title: "Control API contracts and discovery"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
translationRevision: "36ce27660edc95aea83a48cd408dbc683affaeefd2c686ee7229aeecc22d1ac8"
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
