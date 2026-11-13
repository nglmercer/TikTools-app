---
title: "Manage LIVE connections"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: "en"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "642878a380846ff5bf87c9769f203e7223972ccba4152bcfeaa0ab4fd0a443df"
translationRevision: "642878a380846ff5bf87c9769f203e7223972ccba4152bcfeaa0ab4fd0a443df"
order: 10
---

# Manage LIVE connections

## Goal and prerequisites

Connect to a specific creator, understand reconnection, and configure plugin servers independently. You need a working [desktop installation](../installation.md), network access, and the creator's handle. Check [requirements](../requirements.md) before diagnosing operating system problems.

## Connect deliberately

1. Open **Connections**. Enter the handle, with or without `@`; a display name is not the same identifier.
2. Start with **Use Cookie** off. If TikTok requires an authorized session, enable it and enter the cookie only in the masked field. Cookies remain in memory; do not paste them into diagnostics or Cloud settings.
3. Leave **Monitor and auto-connect** off for a direct attempt, then select **Connect**. Read the status and any error before retrying.
4. Check [Live feed](feed.md) for the correct creator and a new event. A connected but quiet room may show no activity.
5. Use **Reconnect** for the same creator or **Disconnect** before changing creators. During a busy attempt, **Cancel** stops that attempt.

After a successful direct connection, enable monitoring to wait for a creator to go live. Waiting and checking are distinct from being connected. Stop monitoring before editing a locked handle. Selecting a recent creator starts a connection; it is more than filling the input.

The explorer offers discovered rooms, but results can become stale. Refresh before assuming a room is still available. Plugin server connections in the lower panel configure their own services; a connected plugin does not establish TikTok or Cloud authentication.

## Expected result and status

Direct connections, monitoring and plugin connection controls are available. Reconnecting to the same creator preserves the current feed context; changing creator starts another context. Actual TikTok access depends on the upstream room and session, and is not guaranteed by a successful local test.

## Recovery

For offline rooms, verify that the creator is live. For session or rate-limit errors, follow the reported retry condition. Repeated reconnect clicks do not refresh authentication. Use [troubleshooting](../preferences/troubleshooting.md) to collect the failing component and time without exposing credentials.
