---
title: "Automation execution and script contracts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a8902b1b03921afa12cad0cea9bbc55912e6b55081154b3d511577dae3b6a400"
translationRevision: "a8902b1b03921afa12cad0cea9bbc55912e6b55081154b3d511577dae3b6a400"
order: 150
---

# Automation execution and script contracts

## Goal and prerequisites

Implement an automation using host descriptors and generated event contracts. Work in a private profile with external effects disabled until intentionally tested. Understand [event normalization](events.md) and [control metadata](control-api.md); the Vue editor configures execution owned by Rust.

## Build a minimal rule

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

## Script boundary

`core.code` and workflow script nodes run bounded QuickJS. They receive JSON `event`, `inputs` and `data` (the inputs alias). There is no Node loader, filesystem, network, WebView or database object. Keep transforms deterministic and JSON serializable; use configured host actions for HTTP, points, audio or plugin effects. Source/result limits and execution budgets belong to the host, not a browser timeout.

## Expected result and status

Saved rules execute through the core runtime and its capability broker. Processors can add `event.intel` evidence without replacing raw fields. Automations Changes is a local review surface, not account-wide configuration synchronization. Reviewed Variables has real native implementation; Overlay Changes is a separate unmerged follow-up.

## Recovery

On conflict, compare current records without losing the candidate. A remote draft remains disabled pending owner review and authority transfer; preview/manifest inspection alone does not authorize execution. Unknown external-effect outcomes require the retained effect inspection/recovery flow. Replaying UI history must never fire the automation again. Use [templates](templates.md) for controlled library import and [testing](testing.md) for boundary-specific evidence.
