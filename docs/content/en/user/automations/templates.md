---
title: "Import templates and manage profiles"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e9847c93a8a75fd5a58cd115b49b8e0bcb091ceda1dbdf2c226b52667eb17e23"
translationRevision: "e9847c93a8a75fd5a58cd115b49b8e0bcb091ceda1dbdf2c226b52667eb17e23"
order: 50
---

# Import templates and manage profiles {#import-templates-and-manage-profiles}

## Goal and prerequisites {#goal-and-prerequisites}

Reuse a reviewed rule or a collection of rules without activating effects accidentally. You need **Automations**, a trusted JSON file or catalog entry, and any plugins required by its actions. Export the current profile before replacing an established setup.

## Choose the right surface {#choose-the-right-surface}

A `.tiktemplate.json` document describes one event and its actions. A `.tikprofile.json` document groups several templates. Open the templates dialog from Automations for built-in/custom templates, its Cloud library or manual template import. Use the profile toolbar for profile creation, selection, export and manual profile import. **Plugins → Explore** browses plugins; templates and profiles belong here.

## Review and import {#review-and-import}

1. Open the intended import dialog and choose a file or paste supported JSON. Check the staged name and source before continuing.
2. For a template, inspect the trigger, action types and parameters. For a profile, also inspect the number of entries and any common parameters.
3. Replace example destinations, media paths and viewer identifiers with values appropriate to your own setup. Do not paste credentials into a file intended for sharing.
4. Apply/import explicitly and wait for the result. Do not assume imported definitions are disabled; inspect the reviewed enabled values. Select them afterward and review each action and event before explicitly enabling the ones you want.
5. Start with a log-only test or inspect definitions first. Tests of HTTP, audio, points or plugins can execute effects.
6. Before confirming a profile switch, review that destination members will be enabled and all nonmembers disabled, including loose records and previously disabled destination members. Effects require triggers/conditions; this is not Cloud sync.

Parameters such as <code v-pre>{{ params.giftName }}</code> resolve when applying a template. Runtime references such as <code v-pre>{{ event.user.uniqueId }}</code> remain for each event, while [globals](variables.md) provide shared saved values at execution.

## Expected result and status {#expected-result-and-status}

Local template/profile import and export surfaces are available. Cloud browsing requires the corresponding host capability and account/service. A catalog preview is not evidence that an action's external service works. Missing plugins can leave imported definitions unusable until installed and configured.

## Recovery {#recovery}

For invalid JSON, use the reported parser errors and the expected document kind; renaming an arbitrary JSON file does not convert it. For a revision conflict, refresh the saved library before applying again. Retain the exported known-good profile and see [backups](../preferences/backups.md) for data that profile export does not include.

## Operational details {#operational-details}

Manual template import first stores a catalog document; applying it creates linked actions/events. The parser defaults omitted `enabled` to true and application preserves those values. Manual profile import also prepares its entries for the owner review. Cloud catalog template import stores templates; Cloud profile import stages a profile mutation with source provenance. Neither origin nor a successful preview is a universal disabled-state guarantee. Inspect the final comparison and saved states. For CLI imports explicitly use `tiktools template import file.tiktemplate.json --disabled` or `tiktools template profile-import file.tikprofile.json --disabled`.

Switching profiles is an activation change: confirmation enables destination members and disables every nonmember action/event, including loose records. Previously disabled destination members may become enabled. Review both lists before confirming. Effects still require matching triggers, conditions and available runtime; the switch does not itself send a Cloud sync.
