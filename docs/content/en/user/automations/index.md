---
title: "Organize automations"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ed0812cc38caebe0861f2732f625347dea351ed14cc7d0c985e2b8cf1dbaf7bd"
translationRevision: "ed0812cc38caebe0861f2732f625347dea351ed14cc7d0c985e2b8cf1dbaf7bd"
order: 10
---

# Organize automations

## Goal and prerequisites

Build a small, understandable library that responds to LIVE events. You need a working desktop host. A real [LIVE connection](../live/connections.md) is needed to validate upstream events; plugin actions also require their plugin and service to be ready.

## Understand the library

An **Action** is a reusable configured operation, such as writing a log line or playing a sound. An **Event** chooses a trigger, filters, cooldown and linked actions. A profile groups definitions for a workflow; only one profile pack is active at a time. Runtime [Variables](variables.md) supply values that several actions can reuse.

1. Open **Automations** and confirm the active profile before editing. Search Events and Actions separately so you know which kind of record you are selecting.
2. Begin with the [first logging rule](first-rule.md). Create its action, then an event that references it.
3. Choose a trigger supported by the current host. Use the field picker to add filters for that trigger. Multiple conditions narrow matching; they do not change the incoming payload.
4. Set a cooldown and its scope. A global cooldown limits the rule across viewers; a per-user cooldown tracks viewers separately.
5. Link the intended actions in order. The normal mode runs the chosen list; random mode selects an action from it. A delay action postpones later actions rather than slowing LIVE ingestion.
6. Save, review any displayed definition changes, then explicitly enable the records you intend to run. Check the run history for the action name, outcome and error.

Tests can execute the configured effects. Read the selected action list before pressing **Test** or firing an event, especially for audio, HTTP, points and plugin operations. Use logging while learning.

## Expected result and status

Local editing, execution, templates, profiles and history are available. Plugin triggers appear only while their supporting plugin is installed and enabled. Imported definitions remain disabled until explicit activation. The Events/Actions **Changes** panel is a [local reference comparison](../cloud/changes.md), not automatic Cloud upload.

## Recovery

For no run, check active profile, enabled states, trigger, filters, cooldown and missing actions in that order. For a revision conflict, read the current saved definition and review again. Do not overwrite another person's update blindly. Export a known-good [profile](templates.md) before broad changes.
