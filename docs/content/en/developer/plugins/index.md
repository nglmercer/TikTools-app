---
title: "Plugin development overview"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
translationRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
order: 240
---

# Plugin development overview {#plugin-development-overview}

## Goal and prerequisites {#goal-and-prerequisites}

Choose the smallest plugin boundary that provides your integration. TikTools discovers runtime packages from built-in, user and development roots; adding a package does not require recompiling a named plugin into the host. Use the matching devkit/SDK and a private development profile before testing effects.

## Follow the authoring path {#follow-the-authoring-path}

1. Start with [quickstart](quickstart.md) to scaffold, validate and build an isolated package.
2. Choose a [runtime](runtimes.md) based on native dependencies and trust, not file extension alone.
3. Declare [permissions and UI](permissions-ui.md) so the host can review effects and render configuration.
4. Use [packaging and registry](packaging-registry.md) for integrity, publisher trust and update policy.

The low-level Rust authority is `crates/tiktools-plugin-api`: manifest validation, protocol and UI contracts. `crates/tiktools-plugin-sdk` offers the Rust author surface; `packages/plugin-sdk` contains TypeScript helpers and the [editor schema](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json). Runtime loading belongs to `crates/tiktools-plugin-loader`, not the frontend.

## Understand contributions {#understand-contributions}

An action requests a side effect through a typed result. Event sources publish triggers; observers receive stable `{topic,data}` domain envelopes with bounded delivery. Processors add side-effect-free evidence under `event.intel`, with deadlines, deterministic merging and health isolation. Declarative settings/actions let the host render ordinary configuration. A custom compiled page uses a restricted plugin broker in its own frame/window; its script never becomes part of the main Vue runtime.

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

These are focused host/SDK checks, not validation of an arbitrary external package.

## Expected result and status {#expected-result-and-status}

Native, process, declarative and QuickJS-backed `napi-vm` paths are implemented. WASM is an optional runtime slot without an active general-purpose engine in normal builds. Example integrations live in the separate [plugins-example-tiktools repository](https://github.com/nglmercer/plugins-example-tiktools); source names in old documents do not make them bundled product features.

## Recovery {#recovery}

For discovery failure, inspect manifest compatibility, root precedence, target and entry path. For a denied intent, inspect declared and granted capabilities instead of bypassing the host. An uncertain startup must be reconciled before replacement. Native/process boundaries retain OS-level trust limits; the runtime name alone never establishes a sandbox.
