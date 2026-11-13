---
title: "Browser demo architecture and limits"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e7f0ab271cce7a8c6e78d9d73c38ae0da022cefe2db104b1001a8f45cf8754c"
translationRevision: "2e7f0ab271cce7a8c6e78d9d73c38ae0da022cefe2db104b1001a8f45cf8754c"
order: 140
---

# Browser demo architecture and limits {#browser-demo-architecture-and-limits}

## Goal and prerequisites {#goal-and-prerequisites}

Explore the real application UI with synthetic data in a browser. Use Bun 1.4.4 and the repository's locked install. The explicit demo entry runs independently of Rust, accounts and a running host. Its runtime and browser acceptance checks are part of this repository.

## Build and inspect {#build-and-inspect}

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

For an uncommitted local preview, use `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build`. Its manifest is marked dirty and publication rejects it. Final assembly requires both bundles to report the same clean Git SHA and base; rebuild after committing. Run a development or preview server only when local browser work is wanted. `docs` owns VitePress and `demo` owns the browser application bundle; `site build` assembles them for one static site. The docs workspace keeps its VitePress dependency separate from the application's Vite 8 toolchain. Commands must not silently install dependencies or invoke Cargo.

## Preserve the application boundary {#preserve-the-application-boundary}

`src/web/app/mount-workspace.ts` provides the shared app/Pinia/router factory; importing it does not mount or choose a transport. Explicit demo bootstrap supplies `src/web/demo/runtime/transport.ts` at the shared `ControlTransport`/control-client boundary. Pure seed fixtures live in `demo/fixtures/base-host.ts`, independently of Playwright. `shared/lib/app-storage.ts` and `demo/storage.ts` provide isolated storage adapters so resets cannot clear native preferences or another demo instance. Pages, widgets and features should use their existing entity stores and `src/web/shared/api` calls. Do not create a second imitation application, inject fake native APIs into ordinary native startup or let demo requests fall through to a real host.

`demo/scenarios.ts` validates 11 scenario IDs and the allowed routes/locales/tours. Focused examples additionally accept `embed=connection|feed|automation|points|overlays`. Each embedded panel requires its matching route and cannot start a tour; invalid combinations fail validation. The embedded entry omits the workspace navigation and full demo toolbar while retaining the real page components, stores and simulated runtime. `runtime/host.ts` registers explicit method handlers; discovery lists those handlers, request IDs are echoed in `rpc-response`, and unsupported effects return an error immediately. The host seeds fictional records and advances a fixed clock one tick per Next event. Play and Next event first connect the fictional creator through the simulated RPC when needed, then deliver an immediate event. Play continues every two seconds; Pause stops production. The toolbar displays playback state and the event count. The stream stops at 300 ticks and retains at most 60 feed events and 60 runs. Response queues and stored records have separate limits.

Reset clears only the current `tiktools.demo.<instance>.*` prefix and reloads the frame. Reloading resets module singletons, journals, i18n and diagnostics. Different embedded frames receive distinct instance IDs. Page exit unmounts Vue, disposes stores, detaches the client/transport and clears both timers. The demo does not install a `window.ipc` bridge or perform arbitrary HTTP, shell, RCON, visitor-script execution or native update installation. Its CSP rejects network connections. Log templates and filter simulation use pure shared functions; these are examples of the frontend contract, not a replacement for Rust execution.

The shared entry `startTour(id, {router, capabilities, taskState})` and exported `tourRegistry` live in `src/web/features/guided-help/index.ts`. Seven versioned tours use stable page targets and separate catalogs for each supported language. They load Driver.js only on request, constrain targets to the workspace, respect reduced motion, restore focus and cancel bounded waits on navigation. Dismissal never completes a task or executes an application action. Tutorial selection names the complete task, prepares its matching scenario and pauses playback before starting. Driver’s close control dismisses the guide; the toolbar reports the outcome. Reset clears this instance’s versioned progress.

## Expected result and status {#expected-result-and-status}

The demo demonstrates navigation, local editing, event presentation and recovery states supported by its scenario model. It does not validate real TikTok sessions, OBS Browser Source, Wry/native behavior, a production backend or multiple PCs. Rankings stays hidden; Automations Changes is local-only, reviewed Variables is real native product work, Overlay Changes remains an unmerged follow-up, and updater auto-install gates are experimental.

## Components and Markdown {#components-and-markdown}

`docs/examples/*.vue` supplies both the rendered catalog and source snippets. Small buttons render directly in Markdown without desktop global styles. Stateful dialogs, layout examples and application pages use `DemoFrame`, which builds base-aware, validated URLs and loads an iframe only on request. Place focused examples next to the instructions they demonstrate:

```vue
<DemoFrame scenario="connection" locale="en" panel="connection" title="Try the LIVE connection panel" />
```

The `panel` prop selects the focused `embed` query and route. Before activation, the component shows a lightweight preview; clicking loads the real interface. Each frame receives an isolated instance ID, and Reset clears only that instance. The full demo link removes the embed mode and opens the complete workspace. Keep explanatory steps and expected results in Markdown so the article remains useful before loading the example. The public `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` option exposes the component catalog without enabling all desktop development tools.

## Recovery and verification {#recovery-and-verification}

Reset a scenario after a deliberately simulated gap or failure. If it contacts an external service, stop and repair the transport boundary. Check content, demo build and site assembly independently; a docs build cannot prove native correctness. [Testing](testing.md) distinguishes fake-host, browser, Rust and manual acceptance evidence.
