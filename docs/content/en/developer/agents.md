---
title: "Agents and the typed Rust SDK"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3ce4fe9492e939fffca83d1844d176dc2869735af3aee806934f01fe706a1195"
translationRevision: "3ce4fe9492e939fffca83d1844d176dc2869735af3aee806934f01fe706a1195"
order: 80
---

# Agents and the typed Rust SDK

## Goal and prerequisites

Build scripts or agents against the same host used by the desktop. Have a running isolated host, a matching `tiktools` CLI and access to [discovery](control-api.md). The SDK is `crates/tiktools-client`; it exposes direct, connected local and optional remote backends without introducing another business core on ordinary client paths.

## Start with read-only discovery

```sh
target/debug/tiktools --json system info
target/debug/tiktools api discover
target/debug/tiktools api schema live.status
target/debug/tiktools api call live.status '{}' --format json
```

Resolve supported capabilities and parameter schemas before generating an action. Do not infer TikTok connection, OBS availability or permission to mutate from `system.info` alone. `system.doctor` distinguishes runtime probes and skipped checks from build capabilities.

A Rust async function can attach to the existing owner:

```rust
async fn inspect() -> Result<(), tiktools_client::ClientError> {
    let client = tiktools_client::TikToolsClient::connect().await?;
    let info = client.system_info().await?;
    println!("{info}");
    Ok(())
}
```

The caller supplies its Tokio runtime. Typed methods delegate to registered RPC methods, and the SDK parity integration test guards coverage. Use `call_with_meta` or `call_value_with_meta` when a workflow, automation or owner journal needs versions and command receipts; domain-only helpers intentionally keep their simpler return shape.

## Submit intentional changes

For a durable operation, retain method, canonical parameters, original command ID, host instance and the read revision required by discovery. Save the user's candidate before submission. A remote automation draft must remain explicitly disabled and author-owned; it is pending owner review, not authorized execution. Owner adoption and activation have separate local contracts.

## Expected result, status and recovery

An agent receives the same validation, events and failures as other clients. Agent access is implemented; authority depends on the selected backend and current grants. Unknown acknowledgement requires original-outcome lookup, not automatic resubmission under a new ID. Closed remote summaries cannot reconstruct a full definition, so retain the submitted source privately.

Run `bun scripts/task.ts rust test -p tiktools-client --test parity --locked` after adding a method. The [SDK source](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs) and canonical registry own signatures; historical agent examples can lag new owner recovery coordinators.
