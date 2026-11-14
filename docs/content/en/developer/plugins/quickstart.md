---
title: "Build a first plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "17f8e12522f04bad46b814f437e80c3f480b43331ed1278e49597878e2cee4f8"
translationRevision: "17f8e12522f04bad46b814f437e80c3f480b43331ed1278e49597878e2cee4f8"
order: 250
---

# Build a first plugin {#build-a-first-plugin}

## Goal and prerequisites {#goal-and-prerequisites}

Produce a validated package that returns a harmless action summary. Use the repository's matching `tiktools-plugin` devkit and Bun 1.4.4 for TypeScript authoring. Create this sample in a separate development directory; it is an authoring example, not an application dependency change.

## Scaffold and validate {#scaffold-and-validate}

From the TikTools checkout, build the author CLI through the task runner:

```sh
bun scripts/task.ts rust build -p tiktools-plugin-devkit --locked
```

Put that build's `target/debug/tiktools-plugin` on your tool path, then in your chosen authoring directory:

```sh
tiktools-plugin new acme.hello --template napi-vm-ts --dir acme-hello
cd acme-hello
tiktools-plugin validate
tiktools-plugin validate --publish
```

The scaffold creates a schema-v3 manifest, stable publisher-prefixed ID, action descriptor, SDK import and build configuration. Keep runtime `napi-vm`, `apiVersion:1` and the built module entry aligned. The name is historical; current execution uses QuickJS.

## Write and build the guest {#write-and-build-the-guest}

The generated action can stay entirely inside the typed result boundary:

```ts
import { actionConfig, actionResult, definePlugin } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    return actionResult(String(actionConfig(request).message ?? 'Hello'));
  },
});
```

Install the generated package's declared dependencies in that separate package, then run `tiktools-plugin build --release`. It compiles JavaScript ahead of time and packs an archive. There is no runtime TypeScript transpiler or Node process. The [TypeScript SDK](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/src/plugin.ts) owns helper signatures; the manifest schema is editor assistance while the host parser remains authoritative.

## Expected result and status {#expected-result-and-status}

`validate` reports compatibility and publication issues; a successful build creates a `.plugin` package with its manifest, entry and integrity metadata. A summary-only action needs no extra host effects. If you add event emission, HTTP or audio, update declarations and review the requested authority before testing.

## Recovery and verification {#recovery-and-verification}

If the entry is missing, build before packaging and check its manifest-relative path. Do not point the process runtime at uncompiled JavaScript. For local staging, use `tiktools-plugin dev stage --into /absolute/path/dev-plugins` and a private test host configured for that root. Development staging is separate from installation; it does not prove signed registry trust. Continue with [runtime limits](runtimes.md) and [signing](packaging-registry.md) before distributing the sample.

## Operational details {#operational-details}

Read `hostCompatibility`, `publishReady` and `warnings` in `validate` output. Missing built entry is a warning; `--publish` fails on publication issues, not every compatibility warning. Revalidate after building, inspect the packaged entry/target, and test the destination host. Exit success before compilation does not prove a usable package.

For Rust, scaffold `process-rust` or `native-rust` in a separate author directory as below. Both implement the SDK `Plugin` trait and return `ActionResult::summary`; the process template uses `tiktools_process_plugin!`, while native uses `tiktools_export_native_plugin!` and a `cdylib`. Native ABI/trust is target-specific and in-process. Build for a supported target using the devkit’s target syntax, retain the exact generated entry path and use a private host. Prefer a process template when crash isolation fits the integration.

```sh
tiktools-plugin new acme.process --template process-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-process
tiktools-plugin new acme.native --template native-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-native
tiktools-plugin build acme-process --release
tiktools-plugin validate acme-process --publish
tiktools-plugin build acme-native --release
tiktools-plugin validate acme-native --publish
```
