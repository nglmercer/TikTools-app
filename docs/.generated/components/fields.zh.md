
### BaseTextField.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | 可选 | `'text'` |
| `placeholder` | `string` | 可选 | `undefined` |
| `size` | `ControlSize` | 可选 | `'md'` |
| `disabled` | `boolean` | 可选 | `false` |
| `readonly` | `boolean` | 可选 | `false` |
| `invalid` | `boolean` | 可选 | `false` |
| `revealable` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

### BaseSpinBox.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `min` | `number` | 可选 | `undefined` |
| `max` | `number` | 可选 | `undefined` |
| `step` | `number` | 可选 | `1` |
| `precision` | `number` | 可选 | `undefined` |
| `suffix` | `string` | 可选 | `undefined` |
| `size` | `ControlSize` | 可选 | `'md'` |
| `orientation` | `'vertical' &#124; 'horizontal'` | 可选 | `'vertical'` |
| `disabled` | `boolean` | 可选 | `false` |
| `readonly` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.

### BaseTextArea.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-area/BaseTextArea.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `rows` | `number` | 可选 | `4` |
| `placeholder` | `string` | 可选 | `undefined` |
| `maxLength` | `number` | 可选 | `undefined` |
| `monospace` | `boolean` | 可选 | `false` |
| `disabled` | `boolean` | 可选 | `false` |
| `readonly` | `boolean` | 可选 | `false` |
| `invalid` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseSlider.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/slider/BaseSlider.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `min` | `number` | 可选 | `0` |
| `max` | `number` | 可选 | `100` |
| `step` | `number` | 可选 | `1` |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `number`, `{ default: 0 }`

Slots: —.

### BaseRadio.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/radio/BaseRadio.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `value` | `T` | 必需 | `—` |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `T`, ``

Slots: `default`.

### BaseTagsInput.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tags-input/BaseTagsInput.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `placeholder` | `string` | 可选 | `undefined` |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `string[]`, `{ default: () => [] }`

Slots: —.

### BaseDateField.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/date-field/BaseDateField.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `type` | `'date' &#124; 'time' &#124; 'datetime-local'` | 可选 | `'date'` |
| `min` | `string` | 可选 | `undefined` |
| `max` | `string` | 可选 | `undefined` |
| `size` | `ControlSize` | 可选 | `'md'` |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseColorField.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/color-field/BaseColorField.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `string`, `{ default: '#000000' }`

Slots: —.

### BaseFileUpload.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/file-upload/BaseFileUpload.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `label` | `string` | 必需 | `—` |
| `hint` | `string` | 必需 | `—` |
| `accept` | `string` | 必需 | `—` |
| `maxBytes` | `number` | 必需 | `—` |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

- `error`: `[message: string]`

模型:

- `modelValue`: `File &#124; null`, `{ default: null }`

Slots: —.
