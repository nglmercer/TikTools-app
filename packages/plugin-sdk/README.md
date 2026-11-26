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

## Translations

Descriptors reference messages with `{ "default": "English text", "i18key": "<plugin-id>.some.key" }`.
Ship the translations themselves in the manifest's `translations` block, keyed
by locale; every key must start with the plugin id so catalogs never collide:

```json
"translations": {
  "es": { "acme.greeter.action.title": "Saludar" }
}
```

The host merges the catalog into the behavior snapshot while the plugin is
installed, enabled and available; the UI resolves `i18key` references against
it and falls back to `default` when a key or locale is missing.

## Declarative guided tours

Schema v3 plugins may contribute bounded `guides` for their own automation
action types. A guide is data, not executable tour logic: targets are limited
to the host-owned action editor regions (`editor`, `settings`, `permissions`)
or a `field` key declared by the same action type. The host validates ownership,
namespaces the contribution, renders the catalog entry, opens either the
existing action or an unsaved draft according to `mode`, and never saves the
record automatically.

```json
"guides": [{
  "id": "speak-action",
  "kind": "automation-action",
  "actionType": "acme.greeter.speak",
  "mode": "both",
  "title": { "default": "Create a voice action", "i18key": "acme.greeter.guide.title" },
  "description": { "default": "Configure speech.", "i18key": "acme.greeter.guide.description" },
  "steps": [{
    "id": "text",
    "target": { "kind": "field", "key": "text" },
    "title": { "default": "Text", "i18key": "acme.greeter.guide.text.title" },
    "description": { "default": "Message template.", "i18key": "acme.greeter.guide.text.description" }
  }]
}]
```

Translation values may document automation placeholders such as
`{{ event.data.comment }}` verbatim. They are data rather than interpolation;
if vue-i18n cannot compile those braces as a message, the host falls back to
the raw translation in the active locale before using the descriptor default.
