---
title: "Automation execution and script contracts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "57cc664e4ef2ff26fafb31df6458da439f075c673fbb3e06e635fa03c1032f42"
translationRevision: "57cc664e4ef2ff26fafb31df6458da439f075c673fbb3e06e635fa03c1032f42"
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

The [generated contract directory](https://github.com/nglmercer/TikTools-app/tree/remake/src/automation/contracts/generated) owns frontend event types, registry metadata and samples. Extend Rust contracts and regenerate together when exposing another field.

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
