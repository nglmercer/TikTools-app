---
title: "Frequently asked questions"
docId: "user.preferences.faq"
translationKey: "user.preferences.faq"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
translationRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
order: 40
---

# Frequently asked questions {#frequently-asked-questions}

## Goal and prerequisites {#goal-and-prerequisites}

Resolve common workflow confusion before changing data or credentials. You need the running desktop app for the checks below; [requirements](../requirements.md) describes supported packages. The local features discussed here are available unless an experimental or service-dependent limit is stated.

## Does Cloud sign-in connect TikTok? {#does-cloud-sign-in-connect-tiktok}

No. Open [Connections](../live/connections.md), choose a creator and start LIVE separately. Cloud identity supports account-based services. A TikTok cookie belongs only in its intended masked connection field.

## Why is a connected feed empty? {#why-is-a-connected-feed-empty}

A quiet room may have no new events. Choose all events, clear search and resume scrolling, then inspect connection status. Pausing scrolling leaves ingestion and automations running. Clearing the feed does not reset points or persisted analytics.

## Are statistics only for this session? {#are-statistics-only-for-this-session}

The current [Analytics page](../points/statistics.md) queries persisted aggregates for the selected creator and date range. Widen the dates and refresh to inspect recorded history. It is not a complete chat transcript. Point balances use different rules and can include manual or automation adjustments.

## Where are templates, variables and rankings? {#where-are-templates-variables-and-rankings}

Templates and profiles live in [Automations](../automations/templates.md); Plugins Explore requests plugins. Runtime values live in Automations Variables. The experimental Rankings workspace still has `#/rankings`, but is hidden from main navigation and startup restoration. Ordinary point leaderboards and feed ranking panels remain separate views.

## Does saving synchronize everything? {#does-saving-synchronize-everything}

No. Events/Actions Changes compares a local reference. Variables saving stays local, with separate reviewed send/adopt/withdraw operations when supported. Secret comparisons are masked. Cloud overlays use create/revise and library operations; this version has no shared overlay Changes adapter. Read [Changes](../cloud/changes.md) before choosing a direction.

## Are tests harmless, and do imports run immediately? {#are-tests-harmless-and-do-imports-run-immediately}

Tests can execute effects, including audio, network requests and point changes. Begin with [core.log](../automations/first-rule.md). Imported definitions remain disabled; review and explicitly activate them. Installing a plugin also requires its service configuration before its actions can work.

## Expected result and recovery {#expected-result-and-recovery}

These checks should identify the responsible page without a reset. Automatic update checks are optional; installation is experimental and disabled by current release gates. Full remote collaboration and production nickname search still depend on deployment and acceptance. If uncertainty remains, preserve your data through [backups](backups.md) and collect a [diagnostic report](troubleshooting.md) with version, time and reproduction.
