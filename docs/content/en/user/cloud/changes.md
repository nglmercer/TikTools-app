---
title: "Review changes by area"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "5d7fbfc359cdf7c8c0b545b961361cf3d77ef535627f85a4ea31ed74d60e1155"
translationRevision: "5d7fbfc359cdf7c8c0b545b961361cf3d77ef535627f85a4ea31ed74d60e1155"
order: 20
---

# Review changes by area

## Goal and prerequisites

Understand what a **Changes** panel compares before resetting, sending or adopting data. You need saved local content. Cloud operations additionally need a supported host, the relevant synchronization mode and a [signed-in account](account.md). Panels with the same name do not all perform the same transport.

## Review Events and Actions locally

1. Open **Automations**, select Events or Actions and open **Changes**.
2. Read the notice: this area compares current definitions with a reference retained on this device. The first loaded visit seeds that reference, so it can legitimately show no changes.
3. Select an added, modified or deleted record and inspect the before/after values.
4. Choose **Mark as reviewed** only when you want the current section to become the new local reference. This does not upload definitions or approve delegated execution.
5. Use **Reset** only when you intend to restore the reference. For a newly added record, reset removes it; for a modified/deleted record, it restores the retained definition. Save an export first if needed.

The Events/Actions adapter has no Cloud collection for individual definitions in this version. A clean comparison is neither a Cloud backup nor evidence that another computer received your edits.

## Review Variables and Plugins explicitly

Variables have their own native reviewed sharing: save locally, fetch candidates, inspect the comparison and explicitly send, apply/adopt or withdraw. Secret or blocked values remain masked and cannot be sent through that path. Local save never uploads. See [Variables](../automations/variables.md).

Plugin Changes uses its supported synchronization workspace for plugin selections and staged settings. Publication is section based rather than a separate send for every row. Settings show changed key names without exposing values; conflicts allow choosing the local or Cloud version, not arbitrary field merging. Read the section action and selected synchronization mode before publishing.

Cloud overlays use the [library/editor](../overlays/index.md) create/revise flow. A shared overlay Changes adapter is not available in this base version.

## Expected result, status and recovery

Local automation comparison and capability-gated native sharing are available in their respective areas. Operations with remote side effects depend on the service; production acceptance is not implied. If a review becomes stale, refresh and compare again. If the outcome is unconfirmed, inspect status/journal information before creating another operation. Never treat a timeout as proof that nothing changed.

<DemoFrame scenario="cloud" locale="en" tour="changes" title="Demo: cloud" />
