
### BaseJsonDiff.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `before` | `unknown` | Optional | `undefined` |
| `after` | `unknown` | Optional | `undefined` |
| `beforeLabel` | `string` | Optional | `undefined` |
| `afterLabel` | `string` | Optional | `undefined` |
| `label` | `string` | Optional | `undefined` |
| `switchable` | `boolean` | Optional | `undefined / Vue boolean false` |
| `context` | `number` | Optional | `3` |
| `maxHeight` | `string &#124; number` | Optional | `undefined` |
| `maxLines` | `number` | Optional | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | Optional | `undefined / Vue boolean false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.

### BasePanel.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `title` | `string` | Required | `—` |
| `subtitle` | `string` | Optional | `undefined / Vue boolean false` |
| `flush` | `boolean` | Optional | `undefined / Vue boolean false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Optional | `undefined / Vue boolean false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `leading`, `status`, `actions`, `default`, `footer`.
