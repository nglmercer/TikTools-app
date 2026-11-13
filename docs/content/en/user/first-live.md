---
title: "First LIVE connection"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb7101f2010cd57478f9f6b2ea9cbb8c0a6401975fe4b29b311c41b13c1a5185"
translationRevision: "fb7101f2010cd57478f9f6b2ea9cbb8c0a6401975fe4b29b311c41b13c1a5185"
order: 30
---

# First LIVE connection {#first-live-connection}

## Goal and prerequisites {#goal-and-prerequisites}

Receive events from one creator before enabling automations. You need a working desktop installation, network access and a creator who is currently live. A Cloud sign-in is a separate connection and does not authenticate TikTok.

## Connect {#connect}

1. Open **Connections** or the **Connect** action in the bottom status bar.
2. Enter the creator's handle. The leading `@` is optional. Check spelling rather than using the display name.
3. Start with **Use Cookie** disabled when guest access is possible. If the room requires your authorized session, enable that control and provide the cookie only in its masked field.
4. Leave monitoring off for this first direct connection. Press **Connect** and wait for the connection state to settle.
5. Open **Live feed**. Ask for a harmless chat message or wait for a real event. Check its creator and timestamp.
6. Select an event to inspect details. Select a viewer name or avatar to inspect their tracked profile.

## Expected result {#expected-result}

The status changes to connected and incoming events appear in the feed. An idle room can legitimately produce no new events. Points depend on enabled point rules, and automation results depend on configured actions and events.

## Monitoring and reconnecting {#monitoring-and-reconnecting}

After a successful direct connection, **Monitor and auto-connect** can wait for the creator to go live. Its status distinguishes waiting, checking and connection activity. Stop monitoring to edit a locked creator field. Reconnection to the same creator retains the current feed; changing creator starts another feed context.

## Recovery {#recovery}

For an offline creator, confirm the room is live before retrying. For authentication or rate-limit errors, inspect **Diagnostics** and follow the reported retry condition; repeated clicks do not repair an expired upstream session. If a cookie is needed, obtain a fresh authorized session and keep its value out of screenshots, logs and support requests. It remains in memory, not a reusable Cloud credential.

Disconnect before testing another creator. The demo can teach these controls using fictional events, but its success does not validate a TikTok session.

<DemoFrame scenario="connection" locale="en" tour="connection" title="Demo: connection" />
