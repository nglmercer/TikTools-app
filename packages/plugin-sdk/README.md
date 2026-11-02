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

The guest runs inside napi-vm with deny-by-default capabilities: no `process`,
no `fs`, no network. Every effect is an intent the host validates against the
manifest's `capabilities` and `permissions`.

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
