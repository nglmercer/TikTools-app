
### BaseButton.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseButton.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `variant` | `'default' &#124; 'primary' &#124; 'danger' &#124; 'ghost'` | 可选 | `'default'` |
| `size` | `ControlSize` | 可选 | `'md'` |
| `type` | `'button' &#124; 'submit' &#124; 'reset'` | 可选 | `'button'` |
| `block` | `boolean` | 可选 | `false` |
| `disabled` | `boolean` | 可选 | `false` |
| `pressed` | `boolean` | 可选 | `undefined` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `icon`, `default`.

### BaseIconButton.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseIconButton.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `label` | `string` | 必需 | `—` |
| `hint` | `string` | 可选 | `undefined` |
| `variant` | `'default' &#124; 'ghost' &#124; 'primary' &#124; 'danger'` | 可选 | `'ghost'` |
| `size` | `ControlSize` | 可选 | `'md'` |
| `disabled` | `boolean` | 可选 | `false` |
| `pressed` | `boolean` | 可选 | `undefined` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `default`.
