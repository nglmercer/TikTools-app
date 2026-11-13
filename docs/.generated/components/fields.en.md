
### BaseTextField.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | Optional | `'text'` |
| `placeholder` | `string` | Optional | `undefined` |
| `size` | `ControlSize` | Optional | `'md'` |
| `disabled` | `boolean` | Optional | `false` |
| `readonly` | `boolean` | Optional | `false` |
| `invalid` | `boolean` | Optional | `false` |
| `revealable` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

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
| `disabled` | `boolean` | Optional | `false` |
| `readonly` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.
