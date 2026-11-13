
### BasePanel.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `title` | `string` | Required | `—` |
| `subtitle` | `string` | Optional | `undefined` |
| `flush` | `boolean` | Optional | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Optional | `undefined` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### BaseDialog.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/dialog/BaseDialog.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `title` | `string` | Required | `—` |
| `width` | `number` | Optional | `480` |
| `closable` | `boolean` | Optional | `true` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `open`: `boolean`, `{ default: false }`

Slots: `header-actions`, `default`, `footer-start`, `footer`.
