---
title: "Review statistics across dates"
docId: "user.points.statistics"
translationKey: "user.points.statistics"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f225b60d175147b26bdcda6972e1c4a71bd122230ee7d4c58b337162f9672bb0"
translationRevision: "f225b60d175147b26bdcda6972e1c4a71bd122230ee7d4c58b337162f9672bb0"
order: 20
---

# Review statistics across dates {#review-statistics-across-dates}

## Goal and prerequisites {#goal-and-prerequisites}

Compare activity for a creator over a useful date range and understand why totals differ from the visible feed or point balance. You need recorded activity in the local installation and the intended creator selected through [Connections](../live/connections.md). Cloud sign-in is not the source of these local analytics.

## Query the recorded period {#query-the-recorded-period}

1. Open **Analytics** and confirm the creator shown beside the date controls. The selected creator determines which stored aggregates are queried.
2. Choose **Today**, `7d`, `30d`, `90d` or a custom range. For custom dates, set the beginning and end explicitly and check their order.
3. Read the displayed date span and session count before comparing results. Local date boundaries use a timezone offset; changing the computer's timezone can change how a period is interpreted.
4. Select the metric you need: chats, gifts, likes, diamonds or peak viewers. Use the activity charts to compare periods and the contributor table to inspect the returned leading viewers.
5. Search the contributor table for a viewer already in the returned data. This is not an unrestricted search of every viewer ever observed.
6. Select **Refresh** after new activity or when returning to the page. Record the creator, dates and metric alongside any number you share.

The current page calls the native `analytics.summary` operation with a creator and date span. It reads persisted aggregates, including previously recorded sessions; it is not limited to the events still held in the current feed. The analytics database stores aggregates rather than a chat transcript.

## Expected result and status {#expected-result-and-status}

Persisted local analytics and date-range queries are available. Empty dates legitimately return no activity. Point balances can differ because [earning rates](rules.md), manual adjustments and automation awards express a different measurement. Diamonds, gift counts and points are not interchangeable units.

## Recovery {#recovery}

If data seems absent, clear the contributor search, widen the date range, confirm the creator and refresh. After a host error, retain its diagnostic code and retry once the connection recovers. Clearing the feed does not erase analytics. A fresh data directory has no earlier history; use [backups](../preferences/backups.md) when moving installations.

<DemoFrame scenario="analytics" locale="en" title="Demo: analytics" />
