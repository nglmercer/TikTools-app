---
title: "Understand rankings and competitions"
docId: "user.points.rankings"
translationKey: "user.points.rankings"
locale: "en"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6ee20d94598fcfbf3cc81fc12490d77196957a3954a60d297320f7c567047162"
translationRevision: "6ee20d94598fcfbf3cc81fc12490d77196957a3954a60d297320f7c567047162"
order: 30
---

# Understand rankings and competitions

## Goal and prerequisites

Choose the appropriate ranked view and recognize the limits of the developing rankings workspace. You need incoming activity for meaningful scores. Start with the ordinary [Points](rules.md) leaderboard or the top-viewer panels in [Live feed](../live/feed.md) when you only need to inspect current viewers.

## Use the right view

1. In **Points**, search for a viewer and sort by points, level or viewer identity. Refresh before comparing a manual adjustment with an award.
2. In **Live feed**, inspect the available ranked viewer panels. These can receive ranking-engine snapshots independently of the point balance table.
3. For an overlay, open [Overlays](../overlays/index.md), select the ranking kind and inspect its source options. A configured ranking source must exist before its output can appear.

The dedicated **Rankings & competitions** workspace is experimental. Its frontend route is `#/rankings`, but it is deliberately hidden from the main navigation and is not restored as the startup tab. A missing sidebar item does not mean the route or ranking engine has been removed.

## Configure an experimental ranking

If your environment exposes that workspace, start with one creator, one room, grouping by user and a session window. Add one advertised metric, such as `tiktok.like` / `like.count`, with weight `1`. Set a modest result limit and create the definition. The form also supports rolling and daily windows, with explicit duration/event bounds or timezone offset.

Read the resulting snapshot before introducing additional sources. Pause and resume affect ranking collection; reset intentionally clears ranking state. Competition options and vote-related sources have their own grouping rules. Do not assume a points reset also resets a competition, or that every custom grouping works with every metric.

## Expected result and status

Ordinary leaderboards are available; the dedicated management page remains experimental. Scores come from the host, not from counting displayed feed rows. Real broadcast/OBS acceptance and every competition combination are not established by this guide.

## Recovery

Check creator, room, source enablement, window and pause state when scores stay empty. Remove unsupported source/group combinations rather than inventing field names. Preserve important data through [backups](../preferences/backups.md) before resets, and include definition ID and diagnostic code in a report.
