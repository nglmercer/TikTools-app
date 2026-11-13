# @tiktools/plugin-sdk

Types and helpers for TypeScript plugins that run in the TikTools `napi-vm`
runtime, plus the JSON Schema for `plugin.json`.

```ts
import { actionConfig, actionResult, definePlugin, emit } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    const message = String(actionConfig(request).message ?? 'Hello');
    return actionResult(message, { intents: [emit('acme.greeted', { message })] });
  },
});
```

Build to a single ES module and point the manifest `entry` at it:

```bash
bun build src/index.ts --outdir dist --target browser --format esm
```

The JavaScript guest runs on QuickJS (`napi-vm` in the manifest) with
deny-by-default host capabilities. Its compatibility environment has no full
Node `process`, `fs` or network API; brokered effects are intents validated
against the manifest's `capabilities` and `permissions`.

Trusted, allowlisted `.node` addons are an explicit exception: they execute
native code inside the host process with its OS privileges, outside the
JavaScript capability boundary. See the [runtime trust limits](../../docs/content/en/developer/plugins/runtimes.md)
before loading native dependencies. A process plugin has crash isolation,
but its executable also retains ordinary user OS permissions.

| Helper | Wire result |
| --- | --- |
| `actionResult(summary, extras?)` | `{ summary, logs?, intents?, events? }` |
| `emit(type, data)` | `{ type: 'emit', data: { eventType, data } }` (needs `events.publish`) |
| `audioPlay(path, options?)` | `{ type: 'audio-play', data: { fileRef, volume?, overlap? } }` (needs `audio.play` and `audio.output`) |
| `pollResult(events, logs?)` | `{ events, logs? }`, at most 16 events |
| `actionConfig(request)` / `actionType(request)` | the action's config object and type id |

## Manifest schema

Add the schema to `plugin.json` for editor completion and validation:

```json
{ "$schema": "./node_modules/@tiktools/plugin-sdk/plugin.schema.json" }
```

`tiktools-plugin validate --publish` runs the host's own validation plus the
registry publishing rules; the schema is a convenience, not the authority.
