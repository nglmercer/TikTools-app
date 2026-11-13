---
title: "Prepare overlays for OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "732d9bfcadaba7b08c8a5c0504eea8f23dc7e9b2d63ed9f07287506ac2dad1e8"
translationRevision: "732d9bfcadaba7b08c8a5c0504eea8f23dc7e9b2d63ed9f07287506ac2dad1e8"
order: 10
---

# Prepare overlays for OBS {#prepare-overlays-for-obs}

## Goal and prerequisites {#goal-and-prerequisites}

Design a readable widget and connect its local browser source to OBS. You need TikTools running, the overlay gateway capability and OBS on the same computer for the default loopback address. Live output also needs the relevant incoming event or configured ranking source.

## Design and preview {#design-and-preview}

1. Open **Overlays** and read the gateway status. Choose follow, gift, chat, share, subscribe, likes or ranking.
2. Edit layers, visible text, colors, spacing and avatar options in the design inspector. Use enough contrast and a readable size at your actual broadcast resolution.
3. Replay the preview to inspect layout. Preview content helps assess design; it does not establish LIVE delivery.
4. Use the Undo/Redo controls for local edits, then **Save**. Switching widget kind with unsaved changes asks you to discard them.
5. In the preview/source panel, copy the source URL. The displayed URL masks its credential; use the copy control to obtain the usable URL. Keep that URL private.
6. Add an OBS Browser Source and paste the copied URL. Keep TikTools running and validate one relevant event before relying on it during a broadcast.

The default gateway is `127.0.0.1:17452`. A loopback URL refers to the computer running the browser source, so copying it to another computer does not reach this host automatically. Ranking widgets also need a selected ranking ID; see [ranking status](../points/rankings.md).

## Use the Cloud library {#use-the-cloud-library}

When the host exposes Cloud overlay capabilities, open the library from Overlays. Browse local designs or **My overlays**, use the editor to create a Cloud overlay, or open an existing ID and revise it. The Cloud editor supports text, live fields, artwork and canvas dimensions; image uploads use their separate flow. Local design saving and Cloud create/revise are distinct operations.

## Expected result and status {#expected-result-and-status}

Local designs, gateway controls and capability-gated Cloud editor/library flows are available. A shared overlay **Changes** adapter is not available in this version. OBS, real LIVE and every operating system still require acceptance in your environment.

## Recovery {#recovery}

For missing output, check gateway state, assets, credentials, source URL and selected event kind. For a Cloud revision conflict, reload the current revision before editing again. Never publish the source token in screenshots; report the gateway error through [diagnostics](../preferences/troubleshooting.md).

<DemoFrame scenario="overlays" locale="en" tour="overlay" title="Demo: overlays" />
