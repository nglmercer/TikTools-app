---
title: "Review changes by area"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f875a22e66dd822d7ea1344188d8b7b087a23a777e072240a4a3c7c57969f995"
translationRevision: "f875a22e66dd822d7ea1344188d8b7b087a23a777e072240a4a3c7c57969f995"
order: 20
---

# Review changes by area {#review-changes-by-area}

## Goal and prerequisites {#goal-and-prerequisites}

Understand what a **Changes** panel compares before resetting, sending or adopting data. You need saved local content. Cloud operations additionally need a supported host, the relevant synchronization mode and a [signed-in account](account.md). Panels with the same name do not all perform the same transport.

## Review Events and Actions locally {#review-events-and-actions-locally}

1. Open **Automations**, select Events or Actions and open **Changes**.
2. Read the notice: this area compares current definitions with a reference retained on this device. The first loaded visit seeds that reference, so it can legitimately show no changes.
3. Select an added, modified or deleted record and inspect the before/after values.
4. Choose **Mark as reviewed** only when you want the current section to become the new local reference. This does not upload definitions or approve delegated execution.
5. Use **Reset** only when you intend to restore the reference. For a newly added record, reset removes it; for a modified/deleted record, it restores the retained definition. Save an export first if needed.

The Events/Actions adapter has no Cloud collection for individual definitions in this version. A clean comparison is neither a Cloud backup nor evidence that another computer received your edits.

## Review Variables and Plugins explicitly {#review-variables-and-plugins-explicitly}

Variables have their own native reviewed sharing: save locally, fetch candidates, inspect the comparison and explicitly send, apply/adopt or withdraw. Secret or blocked values remain masked and cannot be sent through that path. Local save never uploads. See [Variables](../automations/variables.md).

Plugin Changes uses its supported synchronization workspace for plugin selections and staged settings. Publication is section based rather than a separate send for every row. Settings show changed key names without exposing values; conflicts allow choosing the local or Cloud version, not arbitrary field merging. Read the section action and selected synchronization mode before publishing.

Cloud overlays use the [library/editor](../overlays/index.md) create/revise flow. A shared overlay Changes adapter is not available in this base version.

## Expected result, status and recovery {#expected-result-status-and-recovery}

Local automation comparison and capability-gated native sharing are available in their respective areas. Operations with remote side effects depend on the service; production acceptance is not implied. If a review becomes stale, refresh and compare again. If the outcome is unconfirmed, inspect status/journal information before creating another operation. Never treat a timeout as proof that nothing changed.

<DemoFrame scenario="cloud" locale="en" tour="changes" title="Demo: cloud" />

## Operational details {#operational-details}

Before v1 → v2 preparation/migration, back up local data and coordinate compatible versions on every device. In setup, prepare, check compatibility and preview; inspect selection, pending, conflict and v1-only counts rather than treating all as synchronized. Enforcing the write-epoch header is irreversible account-wide for v2 writes and can reject old devices. A barrier timeout is ambiguous; repeating a barrier advances it again. Refresh migration/epoch state and the current preview before any further action. Reading the epoch does not prove older writes completed.

Recovery depends on mode: Events/Actions Changes compares local references, so it does not send Cloud writes. For reviewed Variables/Plugins, reload stale comparisons and review current revisions, account and inventories. If plugins are missing, use the setup inventory review/repair and generate a new preview. For an uncertain retained intent, preserve its ID/evidence and inspect current history/state; technical recovery requires a fresh recovery review of frozen entries and Cloud revision. Resuming that reviewed intent does not itself send a remote write. Stop if state cannot be reconciled; do not create a replacement mutation to bypass uncertainty.
