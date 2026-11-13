---
title: "Diagnose a failed operation"
docId: "user.preferences.troubleshooting"
translationKey: "user.preferences.troubleshooting"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
translationRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
order: 30
---

# Diagnose a failed operation {#diagnose-a-failed-operation}

## Goal and prerequisites {#goal-and-prerequisites}

Identify the failing component and collect a useful report without repeatedly triggering effects. You need the app version, the approximate failure time and the operation you attempted. Keep a [backup](backups.md) before resets or data replacement.

## Inspect the nearest evidence {#inspect-the-nearest-evidence}

1. Read the inline error where the failure occurred. Use its diagnostic status action, or open **Diagnostics**, to inspect related entries.
2. Filter by source, level or text. Useful sources include LIVE, monitoring, plugins, account, synchronization, updates, transport, catalog, installation and collaboration.
3. Select the relevant entry and retain its time, code, request/entity identifiers and short description. Expand details when available.
4. Copy the filtered diagnostics or export the text report. If clipboard/export fails, use the displayed fallback text. Review it before sharing; remove private tokens, cookies, invitation links and personal content.
5. Include a short reproduction: page, selected creator/profile/plugin, action taken, expected result and actual result. Explain whether it happened before or after an update.

## Isolate common failures {#isolate-common-failures}

For an empty feed, clear search and filters, resume scrolling and confirm the [connection](../live/connections.md). Paused scrolling is not disconnection. A display loss indicator does not prove points or analytics lost the same event.

For an automation with no run, check active profile, event/action enablement, trigger, filter field paths and cooldown. Inspect run history before changing effects. **Test** can execute audio, HTTP, points and plugin actions.

For a plugin failure, read the installation job or connection error. For Cloud, check account state and the affected area rather than replacing a TikTok cookie. For overlays, inspect gateway status and source configuration. For updates, use [manual update guidance](../updates.md) when installation is unavailable.

## Expected result and status {#expected-result-and-status}

Diagnostics filtering, copying and text export are available. A packaged app also needs its web assets and platform runtime. On Linux, check the supported GTK/WebKitGTK environment and X11/XWayland session; on Windows, check WebView2. See [requirements](../requirements.md) for installation scope.

## Recovery {#recovery}

Retry after correcting the specific cause. For revision conflicts, reload and review the current saved state. For an unconfirmed remote command, inspect its journal/status before sending another: a timeout can follow a successful effect. If startup fails, report the package/version and startup log rather than deleting the data directory.

<DemoFrame scenario="diagnostics" locale="en" tour="diagnostics" title="Demo: diagnostics" />
