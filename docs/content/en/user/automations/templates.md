---
title: "Import templates and manage profiles"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a8f48a8f385f19965c6f887a0fe90ca07dc34d65dd65147d529d356c7d896f0b"
translationRevision: "a8f48a8f385f19965c6f887a0fe90ca07dc34d65dd65147d529d356c7d896f0b"
order: 50
---

# Import templates and manage profiles

## Goal and prerequisites

Reuse a reviewed rule or a collection of rules without activating effects accidentally. You need **Automations**, a trusted JSON file or catalog entry, and any plugins required by its actions. Export the current profile before replacing an established setup.

## Choose the right surface

A `.tiktemplate.json` document describes one event and its actions. A `.tikprofile.json` document groups several templates. Open the templates dialog from Automations for built-in/custom templates, its Cloud library or manual template import. Use the profile toolbar for profile creation, selection, export and manual profile import. **Plugins → Explore** browses plugins; templates and profiles belong here.

## Review and import

1. Open the intended import dialog and choose a file or paste supported JSON. Check the staged name and source before continuing.
2. For a template, inspect the trigger, action types and parameters. For a profile, also inspect the number of entries and any common parameters.
3. Replace example destinations, media paths and viewer identifiers with values appropriate to your own setup. Do not paste credentials into a file intended for sharing.
4. Apply/import explicitly and wait for the result. Imported definitions stay disabled. Select them afterward and review each action and event before explicitly enabling the ones you want.
5. Start with a log-only test or inspect definitions first. Tests of HTTP, audio, points or plugins can execute effects.
6. Confirm the active profile before starting LIVE. Switching profiles changes the selected library context; it is not a Cloud synchronization command.

Parameters such as <code v-pre>{{ params.giftName }}</code> resolve when applying a template. Runtime references such as <code v-pre>{{ event.user.uniqueId }}</code> remain for each event, while [globals](variables.md) provide shared saved values at execution.

## Expected result and status

Local template/profile import and export surfaces are available. Cloud browsing requires the corresponding host capability and account/service. A catalog preview is not evidence that an action's external service works. Missing plugins can leave imported definitions unusable until installed and configured.

## Recovery

For invalid JSON, use the reported parser errors and the expected document kind; renaming an arbitrary JSON file does not convert it. For a revision conflict, refresh the saved library before applying again. Retain the exported known-good profile and see [backups](../preferences/backups.md) for data that profile export does not include.
