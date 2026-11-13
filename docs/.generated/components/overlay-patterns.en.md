
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

### BaseSpinBox.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `min` | `number` | Optional | `undefined` |
| `max` | `number` | Optional | `undefined` |
| `step` | `number` | Optional | `1` |
| `precision` | `number` | Optional | `undefined` |
| `suffix` | `string` | Optional | `undefined` |
| `size` | `ControlSize` | Optional | `'md'` |
| `orientation` | `'vertical' &#124; 'horizontal'` | Optional | `'vertical'` |
| `disabled` | `boolean` | Optional | `undefined / Vue boolean false` |
| `readonly` | `boolean` | Optional | `undefined / Vue boolean false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.
