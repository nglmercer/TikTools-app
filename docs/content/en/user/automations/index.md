---
title: "Organize automations"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "33d5d9aa4db49a70a3b416ea817479822f06d7cb028699ab9bd344c14cd79892"
translationRevision: "33d5d9aa4db49a70a3b416ea817479822f06d7cb028699ab9bd344c14cd79892"
order: 10
---

# Organize automations {#organize-automations}

## Goal and prerequisites {#goal-and-prerequisites}

Build a small, understandable library that responds to LIVE events. You need a working desktop host. A real [LIVE connection](../live/connections.md) is needed to validate upstream events; plugin actions also require their plugin and service to be ready.

## Understand the library {#understand-the-library}

An **Action** is a reusable configured operation, such as writing a log line or playing a sound. An **Event** chooses a trigger, filters, cooldown and linked actions. A profile groups definitions for a workflow; only one profile pack is active at a time. Runtime [Variables](variables.md) supply values that several actions can reuse.

1. Open **Automations** and confirm the active profile before editing. Search Events and Actions separately so you know which kind of record you are selecting.
2. Begin with the [first logging rule](first-rule.md). Create its action, then an event that references it.
3. Choose a trigger supported by the current host. Use the field picker to add filters for that trigger. Multiple conditions narrow matching; they do not change the incoming payload.
4. Set a cooldown and its scope. A global cooldown limits the rule across viewers; a per-user cooldown tracks viewers separately.
5. Link the intended actions in order. The normal mode runs the chosen list; random mode selects an action from it. A delay action postpones later actions rather than slowing LIVE ingestion.
6. Save, review any displayed definition changes, then explicitly enable the records you intend to run. Check the run history for the action name, outcome and error.

Tests can execute the configured effects. Read the selected action list before pressing **Test** or firing an event, especially for audio, HTTP, points and plugin operations. Use logging while learning.

## Expected result and status {#expected-result-and-status}

Local editing, execution, templates, profiles and history are available. Plugin triggers appear only while their supporting plugin is installed and enabled. Activation depends on the import surface and reviewed enabled values; inspect both events and actions before confirming. The Events/Actions **Changes** panel is a [local reference comparison](../cloud/changes.md), not automatic Cloud upload.

## Recovery {#recovery}

For no run, check active profile, enabled states, trigger, filters, cooldown and missing actions in that order. For a revision conflict, read the current saved definition and review again. Do not overwrite another person's update blindly. Export a known-good [profile](templates.md) before broad changes.

## Operational details {#operational-details}

Before changing profiles or importing, read the [activation rules by surface](templates.md#operational-details). Profile switching can enable members that you had individually disabled. Cooldown state is in memory, capped at 24 hours; missing viewer uniqueId shares one anonymous scope. The table clears when it reaches 8,192 retained keys, so overload or restart can allow extra runs. It is not a durable quota or replay guarantee.
