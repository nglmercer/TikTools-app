
### BaseJsonView.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonView.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `value` | `unknown` | Optional | `undefined` |
| `source` | `string` | Optional | `undefined` |
| `label` | `string` | Optional | `undefined` |
| `lineNumbers` | `boolean` | Optional | `false` |
| `switchable` | `boolean` | Optional | `false` |
| `copyable` | `boolean` | Optional | `true` |
| `maxHeight` | `string &#124; number` | Optional | `undefined` |
| `maxLines` | `number` | Optional | `DEFAULT_MAX_LINES` |
| `maxChars` | `number` | Optional | `DEFAULT_MAX_CHARS` |
| `sortKeys` | `boolean` | Optional | `false` |
| `treeDepth` | `number` | Optional | `DEFAULT_TREE_DEPTH` |

Own events (native control attributes are forwarded according to the component):

- `copy`: `[text: string]`

Models:

- `mode`: `'code' &#124; 'tree'`, `{ default: 'code' }`
- `wrap`: `boolean`, `{ default: true }`

Slots: —.

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
