---
title: "Persistence and durable recovery"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d908889440994004afab995d8dcfdef814c656b146293b70deb63c7bdb0dac7c"
translationRevision: "d908889440994004afab995d8dcfdef814c656b146293b70deb63c7bdb0dac7c"
order: 60
---

# Persistence and durable recovery

## Goal and prerequisites

Change persistent state through its owning service and preserve recoverable outcomes. Use a disposable profile, close its host before copying databases and read [control metadata](control-api.md) before editing versioned resources. `TIKTOOLS_HOME` selects the application root; `AppPaths` resolves its data and private stores.

## Identify the store

`crates/tiktools-core/src/paths.rs` defines `data/tiktok-points.db`, `data/tiktok-automation.db` and `data/tiktok-analytics.db`. SQLite service modules own their schemas. Points/viewers and automation definitions have separate business ledgers; analytics is a distinct database, which older two-database diagrams omit.

Startup can copy checkout-local legacy databases into platform app data only when a destination is absent. It never overwrites an existing destination. JSON definitions remain readable during native execution changes. Do not implement migration by deleting a user's profile or rewriting tables from Vue.

## Make a recoverable change

1. Locate the service transaction and its schema/migration tests.
2. Store the business change, revision and required journal/outbox reservation in the same transaction where that domain supports it.
3. Preserve revision tombstones on deletion. A reused record ID must not accept a pre-deletion edit revision.
4. Publish committed invalidation metadata through the existing outbox; a delivery retry must not repeat the underlying action.
5. Reopen the same private test home after interruption and resolve the original command before attempting another mutation.

Useful read-only checks against an already running test host:

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system doctor
target/debug/tiktools api discover
```

## Expected result and status

Supported durable workflow, points and owner journals retain original outcomes across restart. A receipt describes its historical command; read current state separately. Cloud workspace stores add origin/account partitions and generation checks; they do not replace the local SQLite authority. Not every external effect or domain has the same durable recovery coverage.

## Recovery

Keep SQLite files and any active WAL/SHM together when making an offline backup. A UI timeout does not prove rollback. On `revision_conflict`, preserve the draft and compare a fresh read. On uncertain external effects, inspect the retained effect record rather than replaying a feed. See the [path authority](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/paths.rs) and [Cloud recovery](cloud.md) for their distinct boundaries.
