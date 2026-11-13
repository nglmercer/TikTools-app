---
title: "Configure point rules"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b7e9f9456c4e6f5abbdbb3858192c39593e5e7393972bab1d2ca2457d63537e6"
translationRevision: "b7e9f9456c4e6f5abbdbb3858192c39593e5e7393972bab1d2ca2457d63537e6"
order: 10
---

# Configure point rules

## Goal and prerequisites

Choose how incoming activity earns your local currency, then verify one simple award. You need TikTools running and a known creator for a later LIVE check. Point rules and automation actions are separate: a rule can award points even when you have created no automation.

## Set the earning policy

1. Open **Points** and locate the rules panel. Choose a recognizable currency name, such as `Stars`.
2. Enable only the sources you want: coins from gifts, chat, likes, shares, follows or joins. Each source has its own rate and enable switch.
3. For an easy first check, enable chat at `1` point and disable the other sources. Leave the subscriber bonus at `0` while establishing the baseline.
4. Set points per level, for example `100`. The editor enforces a minimum of `10`; rates cannot be negative.
5. Select **Save** and wait for confirmation. **Discard** restores the saved configuration while you are still editing.
6. Connect through [Connections](../live/connections.md), observe a new chat event in the [feed](../live/feed.md), then refresh the leaderboard and search for that viewer.

Gift and like rates use received contribution quantities. For example, a like contribution of `10` at `0.1` points per like yields `1` point before any applicable bonus. A displayed feed row is not necessarily one contribution unit. Changing rates sets the policy for subsequent awards; do not expect it to recalculate the past.

## Expected result and status

Local rules, viewer balances and manual add/deduct controls are available. With the isolated chat setup, one accepted new chat should increase its viewer's balance by one. Levels follow the configured points-per-level value. These are local application points, not TikTok currency or a payout promise.

## Recovery

Check the source enable switch, saved rate and viewer identifier first. An automation that also awards points can add a second award. Reset controls are locked during active/busy LIVE connections; disconnect before a planned reset and read its confirmation. Keep a [closed-app backup](../preferences/backups.md) before clearing accumulated balances.

<DemoFrame scenario="points" locale="en" tour="points" title="Demo: points" />
