---
title: "Read and inspect the LIVE feed"
docId: "user.live.feed"
translationKey: "user.live.feed"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "8589ccb9d2f5cc646ae0bf9ac5164b90ba7cc73e4bcc819535fc59e0752fe30d"
translationRevision: "8589ccb9d2f5cc646ae0bf9ac5164b90ba7cc73e4bcc819535fc59e0752fe30d"
order: 20
---

# Read and inspect the LIVE feed {#read-and-inspect-the-live-feed}

## Goal and prerequisites {#goal-and-prerequisites}

Find an event, inspect its fields and distinguish a paused display from a disconnected room. Start with a [LIVE connection](connections.md), or use events already present in the current feed. The feed is a bounded working view, not a complete recording of a broadcast.

## Inspect activity {#inspect-activity}

1. Open **Live feed** and confirm the active creator in the connection status.
2. Choose an event filter: all events, chat, gifts, likes, follows, joins or other social activity. Combine it with search to find an author, message, gift name or exact gift ID.
3. Select **Pause** to stop automatic scrolling while you read. Events can continue arriving and the unread count can increase. This control does not disconnect TikTok or suspend automations.
4. Select an event to open its inspector. Read the event type and available details before choosing automation fields. Not every event carries chat text, gift information or a viewer identity.
5. Select a viewer's name or avatar to inspect their tracked profile. Use the inspector toggle if the details panel is hidden.
6. Select **Resume** or the unread-event jump button to return to incoming activity. Clear the search and select all events when checking whether activity is arriving.

Use the labeled controls with keyboard focus where possible. Pausing scrolling helps when movement makes reading difficult. A missing avatar has an initials fallback; a picture is not needed to identify a viewer.

## Expected result and status {#expected-result-and-status}

Filtering and inspection are available. During heavy traffic, the interface can show a limited rendering window and report dropped UI events. These indicators describe the display; do not use its visible row count as an accounting total. [Analytics](../points/statistics.md) queries persisted aggregates separately.

## Recovery {#recovery}

If the feed is empty, remove filters, resume scrolling and check connection status. A quiet room may be healthy. Reconnecting to the same creator retains context; switching creator changes it. **Clear** empties the working feed and does not reset points or analytics. Investigate loss indicators through [diagnostics](../preferences/troubleshooting.md), without assuming all missing display rows are missing upstream events.

<DemoFrame scenario="feed" locale="en" title="Demo: feed" />
