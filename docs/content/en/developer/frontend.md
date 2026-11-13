---
title: "Frontend layers and data flow"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "40ff5183f34bff831db48c76fef50f9ef8d5fe40b632ed77747c10ebad708fb3"
translationRevision: "40ff5183f34bff831db48c76fef50f9ef8d5fe40b632ed77747c10ebad708fb3"
order: 40
---

# Frontend layers and data flow {#frontend-layers-and-data-flow}

## Goal and prerequisites {#goal-and-prerequisites}

Extend the Vue application without coupling a page to native IPC. The frontend uses Vue 3.5, Pinia 4, Vue Router 5 and vue-i18n 11 from the installed workspace ranges. Use the locked dependencies and [setup](setup.md) commands rather than a new frontend scaffold.

## Find the owning layer {#find-the-owning-layer}

The current Feature-Sliced Design layout is under `src/web/`:

| Layer | Responsibility | Existing example |
| --- | --- | --- |
| `app` | Bootstrap, routing, shell and runtime lifetime | `app/model/use-app-runtime.ts` |
| `pages` | Route composition | `pages/automations/ui/AutomationsPage.vue` |
| `widgets` | Substantial composed panels | `widgets/live-feed/ui/LiveFeedPanel.vue` |
| `features` | User operations and their drafts | `features/apply-rule-template/` |
| `entities` | Domain stores and reusable domain logic | `entities/rule-template/` |
| `shared` | API, UI primitives, i18n and generic utilities | `shared/api/control-client.ts` |

Compose higher layers from lower ones. Keep host calls behind the shared control client; transport selection belongs to bootstrap. The old `src/web/platform` and `src/web/views` locations in legacy articles are historical.

## Implement a focused change {#implement-a-focused-change}

1. Locate the route in `app/router/routes.ts`, then follow its page to the feature and entity store.
2. Use an existing shared component for fields, status or dialogs. Preserve its labels, focus behavior and event contract.
3. Keep editable candidates separate from saved host records. Use `shared/lib/request-scope.ts` to ignore obsolete asynchronous answers; invalidation does not cancel an accepted host mutation.
4. Add locale keys through `shared/i18n` and dispose subscriptions at their owning lifetime.

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## Expected result, status and recovery {#expected-result-status-and-recovery}

The route renders current host data and retains unsaved edits on conflicts. Rankings has source and a route but remains hidden from normal navigation; do not infer product availability from a component's presence. Native, local-browser and demo bootstrap paths can provide different transports while using the same feature code.

If a stale reply replaces a newer query, check its generation before changing the backend. If a route loses its plugin page, return to the installed plugins view as the runtime already does. The [application runtime](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts) is the reference for attachment, resynchronization and teardown.

## Operational details {#operational-details}

The bootstrap chooses a real or demo ControlClient, then `mountWorkspace({client,locale,target})` installs Pinia, i18n, router and `controlClientKey`; AppShell starts `useAppRuntime` subscriptions/gap resync and presentation. `useControlClient` throws without that provider even if Pinia/i18n exist. Reuse the mount factory for an app, or provide the client explicitly in a component test; unmount and detach when finished. Do not choose transports inside entities.

For a view query, create a request ticket and activity exit before awaiting. Apply replies only if the ticket is current; always exit activity in finally. Invalidate on context change/unmount. Each overlapping operation gets its own exit, so one response cannot clear another operation’s busy indicator. Invalidating a ticket does not cancel remote work.

```ts
import { createRequestScope, createActivityCounter } from '@/shared/lib/request-scope.ts';
const requests = createRequestScope();
const activity = createActivityCounter();
async function refresh() {
  const ticket = requests.begin();
  const leave = activity.enter();
  try {
    const result = await client.call('system.info', {});
    if (ticket.isCurrent()) info.value = result;
  } finally { leave(); }
}
// onBeforeUnmount(() => { requests.invalidate(); activity.reset(); });
```

```ts
import { mountWorkspace } from '@/app/mount-workspace.ts';
const app = await mountWorkspace({ client, locale: 'en', target: '#app' });
// At teardown:
app.unmount();
client.detach();
```
