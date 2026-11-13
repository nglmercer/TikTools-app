
### BaseComboBox.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/combo-box/BaseComboBox.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `options` | `readonly SelectOption<T>[]` | Required | `—` |
| `placeholder` | `string` | Optional | `''` |
| `size` | `ControlSize` | Optional | `'md'` |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `T`, ``

Slots: `prefix`.

### BaseSearchSelect.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/search-select/BaseSearchSelect.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `options` | `readonly SearchOption[]` | Required | `—` |
| `placeholder` | `string` | Optional | `undefined` |
| `allowCustom` | `boolean` | Optional | `false` |
| `disabled` | `boolean` | Optional | `false` |
| `invalid` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

- `search`: `[query: string]`

Models:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`.
