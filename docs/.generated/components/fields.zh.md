
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
