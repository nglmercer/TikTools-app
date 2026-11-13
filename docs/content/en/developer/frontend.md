---
title: "Frontend layers and data flow"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d4e475fe98a36afd316ddf03877384a3b0a376c8ab93b4892d64f66a7671d746"
translationRevision: "d4e475fe98a36afd316ddf03877384a3b0a376c8ab93b4892d64f66a7671d746"
order: 40
---

# Frontend layers and data flow

## Goal and prerequisites

Extend the Vue application without coupling a page to native IPC. The frontend uses Vue 3.5, Pinia 4, Vue Router 5 and vue-i18n 11 from the installed workspace ranges. Use the locked dependencies and [setup](setup.md) commands rather than a new frontend scaffold.

## Find the owning layer

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

## Implement a focused change

1. Locate the route in `app/router/routes.ts`, then follow its page to the feature and entity store.
2. Use an existing shared component for fields, status or dialogs. Preserve its labels, focus behavior and event contract.
3. Keep editable candidates separate from saved host records. Use `shared/lib/request-scope.ts` to ignore obsolete asynchronous answers; invalidation does not cancel an accepted host mutation.
4. Add locale keys through `shared/i18n` and dispose subscriptions at their owning lifetime.

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## Expected result, status and recovery

The route renders current host data and retains unsaved edits on conflicts. Rankings has source and a route but remains hidden from normal navigation; do not infer product availability from a component's presence. Native, local-browser and demo bootstrap paths can provide different transports while using the same feature code.

If a stale reply replaces a newer query, check its generation before changing the backend. If a route loses its plugin page, return to the installed plugins view as the runtime already does. The [application runtime](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts) is the reference for attachment, resynchronization and teardown.
