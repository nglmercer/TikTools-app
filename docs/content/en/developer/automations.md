---
title: "Automation execution and script contracts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "53beca9878c74b282c8ffc0354132e854357ba73b462514f3b635ded16a71708"
translationRevision: "53beca9878c74b282c8ffc0354132e854357ba73b462514f3b635ded16a71708"
order: 150
---

# Automation execution and script contracts {#automation-execution-and-script-contracts}

## Goal and prerequisites {#goal-and-prerequisites}

Implement an automation using host descriptors and generated event contracts. Work in a private profile with external effects disabled until intentionally tested. Understand [event normalization](events.md) and [control metadata](control-api.md); the Vue editor configures execution owned by Rust.

## Build a minimal rule {#build-a-minimal-rule}

A behavior action stores a configured action type. A behavior event selects a trigger, filters, cooldown and action IDs. Workflow graphs instead compose versioned nodes and ports. Built-in descriptors come from core; plugin descriptors come from installed manifests. Never hardcode a second action catalog into the editor.

1. Select an emitted trigger from `src/automation/contracts/generated/event-registry.generated.ts`.
2. Use its sample to choose a real dotted field such as `event.user.uniqueId` or `event.data.giftName`.
3. Start with logging or a disabled action, then test the filter and cooldown against a sanitized event.
4. Save through the owning native coordinator, retaining the editor candidate and reviewed versions when required.

```sh
bun scripts/task.ts contracts
bun scripts/task.ts contracts --check
bun scripts/task.ts test bun automation
```

## Verify from the CLI {#cli-verification}

With a local host running, the CLI talks to it over IPC. An agent can check a rule without touching a LIVE chat.

```sh
target/debug/tiktools automation list --kind event
target/debug/tiktools automation get <id>
target/debug/tiktools automation context --event-type tiktok.chat
```

A dry run evaluates filters and reports what each action would do; it executes nothing. The response includes `eventSource` (`live`, `sample`, `custom`), `eventData` (the payload actually used) and `pinned` (values copied from filters into a sample).

Filters are the contract in `crates/tiktools-core/src/services/automation/filters.rs`, mirrored for tests by `src/automation/behavior/filter-simulation.ts` and pinned case-by-case by `tests/fixtures/automation-filters/filter-corpus.json`. Every value operator accepts `values: string[]` plus `quantifier` (`any` default, `all`; `neq` lists normalize to `all` = «none of»), and an optional `as` alias. A matched record's winning candidates are published to action rendering as `{{ match.<alias> }}` / `{{ match.<index> }}` — bounded, sanitized, injected after the cooldown claim and never into delegated projections beyond the explicit `match.*` render-field allowlist. Records may also carry `anyOf`: up to 4 flat groups of up to 12 filters that OR against each other while still AND-ing with `filters`; only the first winning group contributes captures.

```sh
target/debug/tiktools --json automation test --id <id>
target/debug/tiktools --json automation test --id <id> --event '{"type":"tiktok.chat","data":{"comment":"!play Daft Punk"}}'
```

A real fire publishes the event through the live pipeline and **executes the actions**. Use it only when the operator asks for it; check the dry run first.

```sh
target/debug/tiktools --json automation fire --trigger tiktok.chat --event '{"type":"tiktok.chat","data":{"comment":"!play Daft Punk"}}'
target/debug/tiktools --json automation fire --trigger tiktok.chat --event '{"type":"tiktok.chat","data":{"comment":"!play Daft Punk"}}' --record '<editor draft json>'
```

`api call automation.fire '<params>' --dry-run` validates parameters without firing; `api discover` and `api schema <method>` show the current contract.

## Script boundary {#script-boundary}

`core.code` and workflow script nodes run bounded QuickJS. They receive JSON `event`, `inputs` and `data` (the inputs alias). There is no Node loader, filesystem, network, WebView or database object. Keep transforms deterministic and JSON serializable; use configured host actions for HTTP, points, audio or plugin effects. Source/result limits and execution budgets belong to the host, not a browser timeout.

## Expected result and status {#expected-result-and-status}

Saved rules execute through the core runtime and its capability broker. Processors can add `event.intel` evidence without replacing raw fields. Automations Changes is a local review surface, not account-wide configuration synchronization. Reviewed Variables has real native implementation; Overlay Changes is a separate unmerged follow-up.

## Recovery {#recovery}

On conflict, compare current records without losing the candidate. A remote draft remains disabled pending owner review and authority transfer; preview/manifest inspection alone does not authorize execution. Unknown external-effect outcomes require the retained effect inspection/recovery flow. Replaying UI history must never fire the automation again. Use [templates](templates.md) for controlled library import and [testing](testing.md) for boundary-specific evidence.

## Operational details {#operational-details}

The graph below uses catalog v1 trigger → transform → log ports and is disabled. Save it as `docs-graph.json` and validate node versions/ports against `automation.nodes.list`. `workflow save --record` persists the graph; `workflow get`/metadata-aware enable/disable calls verify saved state. Replace versions from fresh reads. Persistence/enablement is not proof of a complete graph executor or visual editor on every build: this checkout’s normal runtime executes behavior Events/Actions. Use the disabled log template in [templates](templates.md#operational-details), inspect it, intentionally enable both records and test a sanitized chat for an observable harmless runtime result. Do not claim LIVE execution merely because a graph saved.

For enrichment, install/enable TextIntel with `events.enrich`, inspect Processors status/metrics and run its sample test. Inspect the actual `event.intel.providers.textintel.comment.moderation.blocked` field before creating an is-true filter linked to `core.log`; missing enrichment is not false evidence. Circuit-open or saturated processors can be skipped. A processor sample test is not end-to-end LIVE acceptance.

For uncertain owner work, inspect `automation.library.get`, `automation.definition.get`, `automation.adoption.get` or `automation.approval.get` as appropriate. Retain pending.commandId, host identity and candidate. Resolve that journal’s original operation (`*.resolve`); if unknown, review actual current state and only then use its `*.review_unknown` with the original ID and explicit acknowledgement. Adoption needs a fresh authority/dependency review, not just inspect/preview. `automation.effects.get {effectId}` inspects a retained external-effect result; inspection authorizes no dispatch and offers no blind retry. Preserve unknown outcomes for operator reconciliation.

```json
{"schemaVersion":1,"id":"docs-example","name":"Docs","enabled":false,"nodes":[{"id":"start","type":"trigger.event","version":1,"position":{"x":0,"y":0},"config":{"eventType":"tiktok.chat"}},{"id":"text","type":"transform.template","version":1,"position":{"x":200,"y":0},"config":{"template":"docs-example"}},{"id":"log","type":"action.log","version":1,"position":{"x":400,"y":0},"config":{"message":"docs-example"}}],"edges":[{"id":"a","kind":"flow","source":"start","sourcePort":"flow","target":"text","targetPort":"flow"},{"id":"b","kind":"flow","source":"text","sourcePort":"flow","target":"log","targetPort":"flow"}]}
```

```sh
tiktools api call automation.nodes.list '{}' --json
tiktools workflow save --record "$(cat docs-graph.json)"
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.enable '{"id":"docs-example"}' --expected-revision REVISION --dry-run --json
tiktools api call automation.library.get '{}' --json
tiktools api call automation.adoption.get '{}' --json
```
