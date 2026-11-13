---
title: "Plugin permissions and isolated UI"
docId: "developer.plugins.permissions-ui"
translationKey: "developer.plugins.permissions-ui"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "240142875d416f523b6daee5f53bf37c16b4b15f409561b38bd5357668239f50"
translationRevision: "240142875d416f523b6daee5f53bf37c16b4b15f409561b38bd5357668239f50"
order: 270
---

# Plugin permissions and isolated UI {#plugin-permissions-and-isolated-ui}

## Goal and prerequisites {#goal-and-prerequisites}

Expose configuration and intentional host effects without importing plugin script into the main application. Use a validated manifest, the matching plugin SDK/UI package and a private test host. Read [runtime trust](runtimes.md): a manifest capability is not an OS sandbox for native libraries or process executables.

## Declare a narrow contribution {#declare-a-narrow-contribution}

Describe action fields/settings through manifest JSON schemas and UI hints. Simple declarative pages render trusted host components. `events.publish` gates event intents; host audio additionally requires the `audio.play` capability and `audio.output` permission. Check the canonical [manifest schema](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json) and Rust parser rather than copying an exhaustive permission list into documentation.

Processors require their declared enrichment contribution and return side-effect-free annotations/views/logs. Domain observers require `events.subscribe` and receive a stable envelope through bounded delivery, not the internal Rust event enum or arbitrary transport handles.

## Build an isolated custom page {#build-an-isolated-custom-page}

A manifest `ui` with `apiVersion:1` and `mode:"webview"` points at compiled `ui/dist/` assets. The main application hosts an opaque-origin script-enabled frame; optional pop-out windows use a restricted native broker. Custom script never executes inside the main Vue runtime.

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
window.addEventListener('pagehide', stop, { once: true });
```

The host supplies plugin identity. The client cannot choose another plugin ID. Broker methods are limited to its own settings/actions/options/events plus locale/theme. A page opened without a broker fails explicitly. Executing an action from a page is a live effect, even if the button says test.

## Expected result and status {#expected-result-and-status}

Declarative rendering, inline frames and native pop-outs have implemented contracts and tests. CORS/CSP, source-window binding and native broker allowlists jointly enforce the boundary; Shadow DOM does not. WebKitGTK's non-configurable IPC property does not need deletion because the native handler itself is restricted.

## Recover and verify {#recover-and-verify}

For a blank frame, inspect asset paths, CSP and opaque-origin CORS before loosening policy. For `broker unavailable`, verify the hosting context. Run `bun scripts/task.ts test bun plugin-ui` and `bun scripts/task.ts test web plugin-webview-host`. The [broker package](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-ui/src/broker.ts) owns envelope/version behavior and teardown semantics.

## Operational details {#operational-details}

Processors need both `processorTypes` and the `events.enrich` capability; declarative packages are skipped because they execute no processor code. Enable/install the package and verify processor readiness before testing its event sample. Start from TextIntel’s manifest/SDK `Plugin::enrich`, not a declarative-only manifest. See [the processor operation recipe](../automations.md#operational-details).

Promise-returning broker calls fail without a broker, but `subscribe` on a top-level page returns an empty unsubscribe function and iframe subscription errors are ignored. Check `broker.transport()` plus a call such as `getSettings()`, and report its rejected promise; a subscription is not a connection probe. Verify isolation with the exact command below.

```sh
bun scripts/task.ts test web plugin-webview-host
```

```ts
const broker = createBrokerClient();
console.log(broker.transport());
await broker.getSettings();
```

```json
{"capabilities":["events.enrich"],"processorTypes":[{"id":"acme.analyze","title":{"default":"Analyze"},"eventTypes":["tiktok.chat"],"stage":"pre-filter","timeoutMs":250,"failureMode":"pass-through","inputs":[{"path":"event.data.comment","role":"message"}]}]}
```
