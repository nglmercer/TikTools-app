---
title: "Widget SDK, gateway and overlay boundaries"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "99d3351859ddb3a3304133f062f68b8e3130416947fc40433dc9c6bffaf85f6a"
translationRevision: "99d3351859ddb3a3304133f062f68b8e3130416947fc40433dc9c6bffaf85f6a"
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

## Operational details {#operational-details}

The independent widget unit suite uses fixtures; the real-gateway Chromium suite needs built widget assets and the external plugin checkout selected by `TIKTOOLS_PLUGINS_REPO`. It skips when that source is missing: report a skip explicitly. Run `bun x --no-install playwright test --config src/widgets/playwright.gateway.config.ts` for that separate boundary. Then verify actual OBS Browser Source, widget credential, event delivery and recovery with real LIVE input; headless Chromium does not establish OBS/native acceptance.
