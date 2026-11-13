---
title: "Use TikTools"
docId: "user.overview"
translationKey: "user.overview"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6b17801afa3c9d70e222ab513770144e48b85c13ac26ccb82382b3c1ee8a12fa"
translationRevision: "6b17801afa3c9d70e222ab513770144e48b85c13ac26ccb82382b3c1ee8a12fa"
order: 0
---

# Use TikTools

TikTools connects a Windows or Linux desktop to a TikTok LIVE, displays its events, keeps viewer points and runs rules on your computer. The desktop interface and the optional headless host share the same Rust core. A Cloud account is separate from the TikTok connection.

## Choose your first task

1. Check the [requirements](requirements.md) and [install a released binary](installation.md). You do not need Rust, Cargo or Bun to use the desktop release.
2. [Connect to a LIVE](first-live.md), starting without a TikTok cookie when the room permits guest access.
3. Use **Live feed** to inspect events. **Points** keeps viewer totals; **Analytics** shows statistics. **Automations** connects triggers to configured actions.
4. Install optional integrations from **Plugins**. External plugin packages are distributed separately from the application.
5. Use **Overlays** to prepare a Browser Source for OBS. The gateway integration is required for live widget delivery.

## Available and experimental features

The main navigation exposes Live feed, Points, Analytics, Connections, Automations, Plugins, Overlays and Settings. Rankings and competitions have a native engine and a direct route, but their management page is still hidden from normal navigation. Do not assume every prototype described in an older design document is a released feature.

Cloud plugin changes and reviewed Variables use explicit section controls. Automations currently compares against a local reference copy; that button does not publish profiles to Cloud. Collaboration transports and some installer handoffs remain experimental. A disabled control usually indicates a missing host capability or an unmet prerequisite.

## When something fails

Open **Diagnostics**, keep the error source and request identifier, and record the installed app version. Avoid including cookies, invitation tokens or private variable values. Start with the error recovery in each guide, then use the support guides. The interactive demo uses fictional data and cannot establish whether your TikTok account, OBS or network works.

For source development, use the separate [developer documentation](../developer/index.md).
