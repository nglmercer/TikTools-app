
### BaseSplitter.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/splitter/BaseSplitter.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Optional | `'horizontal'` |
| `min` | `number` | Optional | `80` |
| `max` | `number` | Optional | `Infinity` |
| `step` | `number` | Optional | `16` |
| `sized` | `'start' &#124; 'end'` | Optional | `'start'` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `size`: `number`, `{ default: 240 }`

Slots: `start`, `end`.

### MasterDetailLayout.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/layout/MasterDetailLayout.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `narrow` | `boolean` | Required | `—` |
| `detailOpen` | `boolean` | Required | `—` |
| `backLabel` | `string` | Required | `—` |
| `detailTitle` | `string` | Optional | `''` |
| `min` | `number` | Optional | `240` |
| `max` | `number` | Optional | `900` |

Own events (native control attributes are forwarded according to the component):

- `back`: `[]`

Models:

- `size`: `number`, `{ default: 300 }`

Slots: `master`, `detail-actions`, `detail`.
