
### BaseJsonDiff.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `before` | `unknown` | Optional | `undefined` |
| `after` | `unknown` | Optional | `undefined` |
| `beforeLabel` | `string` | Optional | `undefined` |
| `afterLabel` | `string` | Optional | `undefined` |
| `label` | `string` | Optional | `undefined` |
| `switchable` | `boolean` | Optional | `false` |
| `context` | `number` | Optional | `3` |
| `maxHeight` | `string &#124; number` | Optional | `undefined` |
| `maxLines` | `number` | Optional | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.

### SyncPanel.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/features/sync-changes/ui/SyncPanel.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `adapter` | `SyncAdapter` | Required | `—` |
| `preparing` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `open`: `boolean`, `{ default: false }`

Slots: `preparation`.
