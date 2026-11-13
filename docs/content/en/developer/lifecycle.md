---
title: "Lifecycle, ownership and cleanup"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3e866271067cf2c4d374aaccd9a6cdf6babf981df4deeff5313947cbf39b5cc7"
translationRevision: "3e866271067cf2c4d374aaccd9a6cdf6babf981df4deeff5313947cbf39b5cc7"
order: 50
---

# Lifecycle, ownership and cleanup {#lifecycle-ownership-and-cleanup}

## Goal and prerequisites {#goal-and-prerequisites}

Make mounting, reconnecting and shutdown release the resources they own. Know the [architecture](architecture.md) and use an isolated `TIKTOOLS_HOME` for native lifecycle experiments. A frontend unsubscribe and a host shutdown solve different ownership problems.

## Follow the desktop lifetime {#follow-the-desktop-lifetime}

`crates/tiktools-desktop/src/app.rs` implements Winit's application handler. Window and WebView creation happen on the event-loop thread. The window begins hidden and appears after Vue sends `frontend-ready`. Closing hides the window when a tray exists; without a tray it initiates shutdown. Quit asks core to stop its services before the event loop exits.

`crates/tiktools-core/src/shutdown.rs` closes plugin start admission early, stops background coordination and monitoring, disconnects LIVE and drains owned work. Repeated shutdown callers wait for the same completion. Dropping a future cannot stop arbitrary native code or undo an external effect.

## Own frontend resources explicitly {#own-frontend-resources-explicitly}

Use the pattern already followed by `use-app-runtime.ts`:

```ts
import { onUnmounted } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';

const control = useControlClient();
const stop = control.onTopic('points.changed', () => {
  // Refresh the owning store's committed presentation here.
});
onUnmounted(stop);
```

The callback body belongs to the entity that owns the view. A feature should dispose its subscriptions, timers, animation frames and request scopes, while app bootstrap owns attaching and detaching the shared transport. Widget template changes must dispose the previous controller rather than leaving its alert timers running.

## Expected result and verification {#expected-result-and-verification}

Navigating away and back does not multiply callbacks. Hidden presentation stops unnecessary rendering without pausing authoritative accounting. Reconnect reports gaps and reloads snapshots; it does not replay business effects. Run `bun scripts/task.ts test web cloud-lifecycle` and `bun scripts/task.ts test web request-scope` for focused examples.

## Recovery and limits {#recovery-and-limits}

A timed-out QuickJS startup can remain uncertain; the manager retains its worker identity and blocks replacement instead of pretending stop succeeded. Some synchronous plugin lifecycle control paths can still block their caller. Treat these as implementation limits, especially during updater handoff. Stop the owning host cleanly rather than deleting its live lock. The [shutdown source](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs) and runtime tests are stronger evidence than a historical lifecycle diagram.

## Operational details {#operational-details}

Shutdown first stops LIVE admission and drains ingress, then cancels observers/disconnects and stops plugin polling. It flushes analytics, resolves points writes (30 s deadline), checkpoints SQLite (5 s), and finishes native shutdown. A plugin poll timeout, analytics failure, unresolved points write or checkpoint failure marks update shutdown unsafe. `AppCore::update_shutdown_safe()` requires both completed shutdown and the safety flag; completion alone is insufficient. The updater consumes this native result—there is no user switch that can certify uncertain work. Preserve logs/pending recovery data and resolve uncertainty before replacing binaries.
