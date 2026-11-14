---
title: "Agents and the typed Rust SDK"
docId: "developer.agents"
translationKey: "developer.agents"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
translationRevision: "fe3c629f94372134069e2f5316f807214666ce435d135cb45c0eba72054c5839"
order: 80
---

# Agents and the typed Rust SDK {#agents-and-the-typed-rust-sdk}

## Goal and prerequisites {#goal-and-prerequisites}

Build scripts or agents against the same host used by the desktop. Have a running isolated host, a matching `tiktools` CLI and access to [discovery](control-api.md). The SDK is `crates/tiktools-client`; it exposes direct, connected local and optional remote backends without introducing another business core on ordinary client paths.

## Start with read-only discovery {#start-with-read-only-discovery}

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

## Submit intentional changes {#submit-intentional-changes}

For a durable operation, retain method, canonical parameters, original command ID, host instance and the read revision required by discovery. Save the user's candidate before submission. A remote automation draft must remain explicitly disabled and author-owned; it is pending owner review, not authorized execution. Owner adoption and activation have separate local contracts.

## Expected result, status and recovery {#expected-result-status-and-recovery}

An agent receives the same validation, events and failures as other clients. Agent access is implemented; authority depends on the selected backend and current grants. Unknown acknowledgement requires original-outcome lookup, not automatic resubmission under a new ID. Closed remote summaries cannot reconstruct a full definition, so retain the submitted source privately.

Run `bun scripts/task.ts rust test -p tiktools-client --test parity --locked` after adding a method. The [SDK source](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-client/src/lib.rs) and canonical registry own signatures; historical agent examples can lag new owner recovery coordinators.

## Headless Teams for agents {#headless-teams}

Build the CLI with host support, or use the matching CLI included in the portable release. Run the owner as a persistent process with an absolute, isolated `TIKTOOLS_HOME`. The commands in another terminal must use that same home.

```sh
cargo build -p tiktools-cli --release --locked --features remote-control
TIKTOOLS_HOME=/absolute/test-owner target/release/tiktools host --ipc
```

`cloud login` authenticates the owner runtime. `remote cloud login` authenticates the controller context, whose account metadata and device keys remain separate. Both use the real browser handoff; the desktop window is unnecessary. `--non-interactive` never starts a login and returns `authentication_required` when credentials are unavailable.

The owner creates a team, enrolls its host device and starts serving. Use `teams people search NICKNAME` to discover a recipient ID, then create a recipient-bound invitation. The token is written once to a new private file; stdout contains metadata only. Approve the request explicitly and configure its native agent grant before connecting.

```sh
tiktools --json teams create --name "Studio" --confirm
tiktools --json teams host enroll --team TEAM_ID --label "Host" --confirm
tiktools --json teams host start --wait --timeout 30 --confirm
tiktools --json teams invitations create --team TEAM_ID --recipient RECIPIENT_ID --role observer --output-file /private/invite --confirm
```

For reusable agent access, use `remote cloud connect --host ID --profile agent --kind agent --wait --non-interactive --timeout 30` after approval. `teams list --joined` reads controller memberships; `teams list --owned` reads owner teams. `teams doctor` inspects the owner; `teams doctor --context controller` inspects controller authentication and memberships. `remote check agent` verifies an actual connection.

```sh
tiktools --json teams join --profile agent --kind agent --invite-stdin --non-interactive < /private/invite
tiktools --json teams requests list --team TEAM_ID
tiktools --json teams requests approve --team TEAM_ID --request REQUEST_ID --confirm
tiktools --json teams grants list --team TEAM_ID
tiktools --json teams grants update --team TEAM_ID --grant GRANT_ID --expected-revision REVISION --role custom --policy @agent-policy.json --confirm
tiktools remote serve --profile agent --stdio --events --timeout 30
```

`agent-policy.json`:

<<< ../../../examples/agent-policy.json

```json
{"jsonrpc":"2.0","id":"agent-1","method":"live.status","params":{}}
```

The persistent client accepts one JSON-RPC request per line and preserves IDs. Writes require `confirm: true`; durable writes also need their documented command/host metadata. EOF, termination or a closed connection releases the client. Requests have bounded sizes and timeouts; a timed-out write may still have committed.

The acceptance runner uses a compiled CLI plus a real disposable backend, synthetic accounts and separate owner/controller processes. Supply a RemakeBackend checkout containing the CLI acceptance fixture and its installed pgrust tools. Development binaries accept the isolated runtime origin. Release binaries remain pinned to their build-time origin: compile a staging release for a fixed loopback TLS port and pass `--cloud-port PORT`. Production-pinned binaries are rejected before authentication. No remote workflow is started.

```sh
bun scripts/task.ts test headless-teams --binary /absolute/path/tiktools --backend /absolute/path/RemakeBackend --report /tmp/headless-teams.json
```

New package builds also accept `./TikTools.AppImage --cli --build-info` and `./TikTools.AppImage --cli --json teams doctor`. This entry point dispatches before GUI initialization; older published AppImages do not gain it automatically.

Reusing a controller profile across CLI processes requires an available OS credential store.
