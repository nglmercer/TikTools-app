---
title: "Configure point rules"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "89e0189daf4193853d17ec9e576a4ff9973cdfc5df06d0b27fc59296d9ee66de"
translationRevision: "89e0189daf4193853d17ec9e576a4ff9973cdfc5df06d0b27fc59296d9ee66de"
order: 10
---

# Configure point rules {#configure-point-rules}

## Goal and prerequisites {#goal-and-prerequisites}

Choose how incoming activity earns your local currency, then verify one simple award. You need TikTools running and a known creator for a later LIVE check. Point rules and automation actions are separate: a rule can award points even when you have created no automation.

<DocCallout title="Test one source at a time" tone="tip">

Start with chat only at one point per message. That makes the expected balance easy to check before adding gifts, likes or subscriber bonuses. These are local application points, not TikTok currency or a payout promise.

</DocCallout>

## Set the earning policy {#set-the-earning-policy}

<DocSteps>

1. **Name the currency.** Open **Points**, locate **Point rules** and choose a recognizable **Currency name**, such as `Stars`.
2. **Choose earning sources.** Each source has its own rate and enable switch: Gift coins, Chat messages, Likes, Shares, Follows and Joins. For the first check, enable only **Chat messages** at `1` point. Leave **Subscriber bonus** at `0`.
3. **Set level progression.** Set **Points per level**, for example `100`. The editor enforces a minimum of `10`; rates cannot be negative.
4. **Save the policy.** Select **Save rules** and wait for confirmation. **Discard** restores the saved configuration while you are still editing.

</DocSteps>

<DemoFrame scenario="points" locale="en" panel="points" title="Try point rules and viewer balances" />

Connect through [Connections](../live/connections.md), observe a new chat event in the [feed](../live/feed.md), then use **Refresh leaderboard** and search for that viewer.

Gift and like rates use received contribution quantities. For example, a like contribution of `10` at `0.1` points per like yields `1` point before any applicable bonus. A displayed feed row is not necessarily one contribution unit. Changing rates sets the policy for subsequent awards; it does not recalculate the past.

## Expected result and status {#expected-result-and-status}

Local rules, viewer balances and manual add/deduct controls are available. With the isolated chat setup, one accepted new chat should increase its viewer's balance by one. Levels follow the configured points-per-level value.

<DocCallout title="Compare the resulting balance" tone="info">

An automation that also awards points can add a second award. Verify the enabled sources and active automations if a viewer receives more than the isolated one-point chat award.

</DocCallout>

## Recovery {#recovery}

Check the source enable switch, saved rate and viewer identifier first. Reset controls are locked during active/busy LIVE connections. Disconnect before a planned reset, read its confirmation and keep a [closed-app backup](../preferences/backups.md) before clearing accumulated balances.

## Operational details {#operational-details}

<DocCallout title="Understand the reset scope" tone="warning">

The row menu resets one viewer to 0 points and level 1 and adds a reset marker. **Reset all points** affects every stored viewer in this installation, across creators: it sets all balances to 0 and levels to 1 and deletes the points transaction history. It keeps viewer records and accumulated counters, and does not clear the feed or Analytics. Disconnect LIVE, export a closed-app backup and read the scope before confirming.

</DocCallout>

Positive manual adjustments use the saved subscriber bonus: +100 with a 50% bonus awards +150 to a subscriber. A −100 adjustment does not receive that bonus; balances stop at zero. Check the resulting balance rather than assuming the entered delta is exact. The table searches only its current top-100 snapshot; a missing match does not prove that the viewer has no saved record.
