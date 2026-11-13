---
title: "Browser demo architecture and limits"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "en"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e2311798c9114ae8540db974017d7b5a571f626ce02f31a39fd6b6e64410491c"
translationRevision: "e2311798c9114ae8540db974017d7b5a571f626ce02f31a39fd6b6e64410491c"
order: 140
---

# Browser demo architecture and limits

## Goal and prerequisites

Explore the real application UI with synthetic data in a browser. Use Bun 1.4.4 and the repository's locked install. This guide describes the intended docs/demo integration: the demo runtime and its acceptance checks must remain consistent with the final bootstrap implementation.

## Build and inspect

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

Run a development or preview server only when local browser work is wanted. `docs` owns VitePress and `demo` owns the browser application bundle; `site build` assembles them for one static site. The docs workspace keeps its VitePress dependency separate from the application's Vite 8 toolchain. Commands must not silently install dependencies or invoke Cargo.

## Preserve the application boundary

`src/web/app/mount-workspace.ts` provides the shared app/Pinia/router factory; importing it does not mount or choose a transport. Explicit demo bootstrap supplies `src/web/demo/runtime/transport.ts` at the shared `ControlTransport`/control-client boundary. Pure seed fixtures live in `demo/fixtures/base-host.ts`, independently of Playwright. `shared/lib/app-storage.ts` and `demo/storage.ts` provide isolated storage adapters so resets cannot clear native preferences or another demo instance. Pages, widgets and features should use their existing entity stores and `src/web/shared/api` calls. Do not create a second imitation application, inject fake native APIs into ordinary native startup or let demo requests fall through to a real host.

Scenarios should seed explicit synthetic records and deterministic event streams. A reset must restore its initial state, terminate previous timers/subscriptions and clear transient drafts or errors. Unsupported operations need a clear simulated/unsupported outcome instead of a fabricated success for cloud login, plugin execution, native audio or an external integration.

The shared entry `startTour(id, {router, capabilities, taskState})` and exported `tourRegistry` live in `src/web/features/guided-help/index.ts`. Seven versioned tours use stable page targets and separate EN/ES/PT/FR catalogs. They load Driver.js only on request, constrain targets to the workspace, respect reduced motion, restore focus and cancel bounded waits on navigation. Dismissal never completes a task or executes an application action.

## Expected result and status

The demo demonstrates navigation, local editing, event presentation and recovery states supported by its scenario model. It does not validate real TikTok sessions, OBS Browser Source, Wry/native behavior, a production backend or multiple PCs. Rankings stays hidden; Automations Changes is local-only, reviewed Variables is real native product work, Overlay Changes remains an unmerged follow-up, and updater auto-install gates are experimental.

## Recovery and verification

Reset a scenario after a deliberately simulated gap or failure. If it contacts an external service, stop and repair the transport boundary. Check content, demo build and site assembly independently; a docs build cannot prove native correctness. [Testing](testing.md) distinguishes fake-host, browser, Rust and manual acceptance evidence.
