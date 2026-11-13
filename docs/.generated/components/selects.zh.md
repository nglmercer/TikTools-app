
### BaseComboBox.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/combo-box/BaseComboBox.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `options` | `readonly SelectOption<T>[]` | 必需 | `—` |
| `placeholder` | `string` | 可选 | `''` |
| `size` | `ControlSize` | 可选 | `'md'` |
| `disabled` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `T`, ``

Slots: `prefix`.

### BaseSearchSelect.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/search-select/BaseSearchSelect.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `options` | `readonly SearchOption[]` | 必需 | `—` |
| `placeholder` | `string` | 可选 | `undefined` |
| `allowCustom` | `boolean` | 可选 | `false` |
| `disabled` | `boolean` | 可选 | `false` |
| `invalid` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

- `search`: `[query: string]`

模型:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`.
