
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

### BaseTextArea.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-area/BaseTextArea.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `rows` | `number` | Optional | `4` |
| `placeholder` | `string` | Optional | `undefined` |
| `maxLength` | `number` | Optional | `undefined` |
| `monospace` | `boolean` | Optional | `false` |
| `disabled` | `boolean` | Optional | `false` |
| `readonly` | `boolean` | Optional | `false` |
| `invalid` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseSlider.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/slider/BaseSlider.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `min` | `number` | Optional | `0` |
| `max` | `number` | Optional | `100` |
| `step` | `number` | Optional | `1` |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `number`, `{ default: 0 }`

Slots: —.

### BaseRadio.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/radio/BaseRadio.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `value` | `T` | Required | `—` |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `T`, ``

Slots: `default`.

### BaseTagsInput.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tags-input/BaseTagsInput.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `placeholder` | `string` | Optional | `undefined` |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `string[]`, `{ default: () => [] }`

Slots: —.

### BaseDateField.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/date-field/BaseDateField.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `type` | `'date' &#124; 'time' &#124; 'datetime-local'` | Optional | `'date'` |
| `min` | `string` | Optional | `undefined` |
| `max` | `string` | Optional | `undefined` |
| `size` | `ControlSize` | Optional | `'md'` |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseColorField.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/color-field/BaseColorField.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `string`, `{ default: '#000000' }`

Slots: —.

### BaseFileUpload.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/file-upload/BaseFileUpload.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `label` | `string` | Required | `—` |
| `hint` | `string` | Required | `—` |
| `accept` | `string` | Required | `—` |
| `maxBytes` | `number` | Required | `—` |
| `disabled` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

- `error`: `[message: string]`

Models:

- `modelValue`: `File &#124; null`, `{ default: null }`

Slots: —.
