---
title: "First LIVE connection"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
translationRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
order: 30
---

# First LIVE connection {#first-live-connection}

## Goal and prerequisites {#goal-and-prerequisites}

Connect one creator, receive an event, and inspect its details before enabling automations. For a real LIVE you need a working desktop installation, internet access, and a creator who is currently broadcasting.

<DocCallout title="Practice before your broadcast" tone="tip">

The examples below use the real controls with fictional data. You can connect `demo_creator` without a TikTok account. A successful demo does not validate a real TikTok session. Cloud sign-in is a separate connection and does not authenticate TikTok.

</DocCallout>

## Connect {#connect}

<DocSteps>

1. **Choose the creator.** Open **Connections**, or use **Connect** in the bottom status bar. Enter the creator's handle; the leading `@` is optional. Use the identifier rather than the display name.
2. **Choose the connection options.** Leave **Use a session cookie** off when guest access is possible. If the room requires your authorized session, enable the control and enter the cookie only in its masked field. Leave **Monitor and auto-connect** off for this first direct connection.
3. **Make the connection.** Press **Connect** and wait for the status to settle. Try the panel below with `demo_creator` and watch the connection indicator.

</DocSteps>

<DemoFrame scenario="connection" locale="en" panel="connection" title="Try the LIVE connection panel" />

## Expected result {#expected-result}

The status changes to connected. Open **Live feed** and wait for an event or ask for a harmless chat message. Check the creator and timestamp, then select the event to inspect its details. Select a viewer's name or avatar to inspect their tracked profile.

<DemoFrame scenario="feed" locale="en" panel="feed" title="Inspect fictional LIVE events" />

An idle room can legitimately produce no new events. Points depend on enabled point rules, and automation results depend on configured actions and events. Receiving an event is the first check; it does not enable those features automatically.

## Monitoring and reconnecting {#monitoring-and-reconnecting}

After a successful direct connection, **Monitor and auto-connect** can wait for the creator to go live. Its status distinguishes waiting, checking and connection activity. Stop monitoring to edit a locked creator field. Reconnection to the same creator retains the current feed; changing creator starts another feed context.

## Recovery {#recovery}

For an offline creator, confirm the room is live before retrying. For authentication or rate-limit errors, inspect **Diagnostics** and follow the reported retry condition; repeated clicks do not repair an expired upstream session.

<DocCallout title="Keep session cookies private" tone="warning">

If a cookie is needed, obtain a fresh authorized session and keep its value out of screenshots, logs and support requests. It remains in memory and is not a reusable Cloud credential. Disconnect before testing another creator.

</DocCallout>
