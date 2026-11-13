---
title: "Build a first plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "66a07fff758de08ce36d8c6494f7b8eb3d56aa1ad30bae5b5eaed42baa11956c"
translationRevision: "66a07fff758de08ce36d8c6494f7b8eb3d56aa1ad30bae5b5eaed42baa11956c"
order: 250
---

# Build a first plugin

## Goal and prerequisites

Produce a validated package that returns a harmless action summary. Use the repository's matching `tiktools-plugin` devkit and Bun 1.4.4 for TypeScript authoring. Create this sample in a separate development directory; it is an authoring example, not an application dependency change.

## Scaffold and validate

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

## Write and build the guest

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

## Expected result and status

`validate` reports compatibility and publication issues; a successful build creates a `.plugin` package with its manifest, entry and integrity metadata. A summary-only action needs no extra host effects. If you add event emission, HTTP or audio, update declarations and review the requested authority before testing.

## Recovery and verification

If the entry is missing, build before packaging and check its manifest-relative path. Do not point the process runtime at uncompiled JavaScript. For local staging, use `tiktools-plugin dev stage --into /absolute/path/dev-plugins` and a private test host configured for that root. Development staging is separate from installation; it does not prove signed registry trust. Continue with [runtime limits](runtimes.md) and [signing](packaging-registry.md) before distributing the sample.
