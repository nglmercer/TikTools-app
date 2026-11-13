---
title: "Widget SDK, gateway and overlay boundaries"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d90002c49e613ce5a797f60447aa89527e5c46faa28f3ee8686b6344fdbd4c25"
translationRevision: "d90002c49e613ce5a797f60447aa89527e5c46faa28f3ee8686b6344fdbd4c25"
order: 170
---

# Widget SDK, gateway and overlay boundaries {#widget-sdk-gateway-and-overlay-boundaries}

## Goal and prerequisites {#goal-and-prerequisites}

Build a widget that shares rendering between local preview and OBS output. Use the installed Vue toolchain and understand [events](events.md). Local preview needs no gateway or account credential. Live widget output requires the separately installed event-gateway plugin and its widget-scoped credential.

## Add a template {#add-a-template}

1. Define a renderer inside `WidgetStage`; size it relative to its parent rather than the browser viewport.
2. Use `defineWidget` from `src/widgets/sdk/template.ts`. Supply its schema, fresh sample envelopes and `create(search, clock)` factory.
3. Return a controller implementing `handleEnvelope`, `clear` and `dispose`, plus rendering. Use the supplied clock so previews can hold samples and tests can advance time.
4. Register it in `sdk/templates.ts`, then add the standalone entry, build configuration, gateway route allowlist and packaging entry. Registration alone does not expose a public URL.
5. Use the same `WidgetHost.vue` in the editor and standalone bundle; do not embed gateway HTML for a local preview.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts test bun widgets
```

Keep text tokens, design defaults and editable fields in the template schema. Designs serialize as JSON and copied widget URLs carry a validated snapshot; a later save requires copying an updated URL into OBS.

## Gateway protocol {#gateway-protocol}

`src/widgets/shared/gateway-client.ts` owns authentication, subscriptions, heartbeat, reconnect and gap reporting. Widgets consume canonical domain envelopes, not TikTok directly. The widget endpoint is `/ws/widgets`; it differs from the full external gateway client surface. Parse envelopes through the shared parser and preserve credential boundaries instead of passing a full gateway token through the main WebView.

## Expected result and status {#expected-result-and-status}

Built-in local previews and the widget SDK are implemented. Cloud overlay editing and `cloud-overlay` source code exist, but real OBS validation, artwork/live-source coverage and package application retain limits. Overlay Changes is an unmerged follow-up. Rankings remains hidden in normal application navigation even though ranking widget code exists.

## Recovery {#recovery}

For a blank local preview, inspect controller lifetime, schema defaults and stage sizing before weakening origin policy. For live output, inspect gateway health, widget credential and explicit gaps. A headless Chromium test does not prove OBS Browser Source acceptance. The [Widget SDK](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk) and shared protocol tests are the current implementation references.
