
### BaseTabs.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tabs/BaseTabs.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `tabs` | `readonly TabOption<T>[]` | 必需 | `—` |
| `label` | `string` | 可选 | `undefined` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `T`, `{ required: true }`

Slots: `bar`, `default`.

### BaseToolbar.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/toolbar/BaseToolbar.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | 可选 | `'horizontal'` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `default`.
