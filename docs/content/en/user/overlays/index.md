---
title: "Prepare overlays for OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3141f778f3bf0e5c9e01415122e7863e9342c266a57bee9811d7a6b75b2c842b"
translationRevision: "3141f778f3bf0e5c9e01415122e7863e9342c266a57bee9811d7a6b75b2c842b"
order: 10
---

# Prepare overlays for OBS {#prepare-overlays-for-obs}

## Goal and prerequisites {#goal-and-prerequisites}

Design a readable widget, inspect its preview, and connect its browser source to OBS. You need TikTools running, the overlay gateway capability and OBS on the same computer for the default loopback address. Live output also needs the relevant incoming event or configured ranking source.

<DocCallout title="Preview the design first" tone="tip">

Use the sample editor to learn the controls before configuring a broadcast. Its fictional preview helps assess appearance; it does not establish real LIVE delivery or OBS reception.

</DocCallout>

## Design and preview {#design-and-preview}

<DocSteps>

1. **Choose a widget.** Open **Overlays**, read the gateway status and choose follow, gift, chat, share, subscribe, likes or ranking.
2. **Make it readable.** Edit layers, visible text, colors, spacing and avatar options in the design inspector. Check contrast and text size at your actual broadcast resolution.
3. **Inspect the preview.** Replay the preview and review the layout. Use Undo/Redo for local edits, then **Save**. Switching widget kind with unsaved changes asks you to discard them.

</DocSteps>

<DemoFrame scenario="overlays" locale="en" panel="overlays" title="Try the overlay design controls" />

<DocSteps>

1. **Copy the source.** In the preview/source panel, use the copy control to obtain the usable source URL. The displayed URL masks its credential; keep the copied URL private.
2. **Add it to OBS.** Create an OBS Browser Source and paste the copied URL. Keep TikTools running and validate one relevant event before relying on it during a broadcast.

</DocSteps>

The default gateway is `127.0.0.1:17452`. A loopback URL refers to the computer running the browser source, so copying it to another computer does not reach this host automatically. Ranking widgets also need a selected ranking ID; see [ranking status](../points/rankings.md).

## Use the Cloud library {#use-the-cloud-library}

When the host exposes Cloud overlay capabilities, open the library from Overlays. Browse local designs or **My overlays**, use the editor to create a Cloud overlay, or open an existing ID and revise it. The Cloud editor supports text, live fields, artwork and canvas dimensions; image uploads use their separate flow. Local design saving and Cloud create/revise are distinct operations.

## Expected result and status {#expected-result-and-status}

Local designs, gateway controls and capability-gated Cloud editor/library flows are available. A shared overlay **Changes** adapter is not available in this version. Validate the design in OBS and confirm real LIVE output on the intended operating system.

## Recovery {#recovery}

For missing output, check gateway state, assets, credentials, source URL and selected event kind. For a Cloud revision conflict, reload the current revision before editing again. Report gateway errors through [diagnostics](../preferences/troubleshooting.md).

<DocCallout title="The source URL contains a credential" tone="warning">

Never publish its token in screenshots or support requests. Use the widget-scoped credential from the generated source URL, and verify reception with a harmless event on the intended OBS source.

</DocCallout>

## Operational details {#operational-details}

Install the separate Event Gateway plugin, enable it and confirm it is running before adding live OBS sources. Widget status guides recovery: missing → install; disabled → enable/check availability; stopped → start; credentialUnavailable → restart the gateway once; assetsMissing → repair/rebuild its packaged widget assets; starting → retry briefly; unreachable → inspect loopback port/service. A ready status or local preview proves neither a real LIVE event nor OBS reception.
