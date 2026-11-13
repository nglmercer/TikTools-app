
### BasePanel.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `title` | `string` | 必需 | `—` |
| `subtitle` | `string` | 可选 | `undefined` |
| `flush` | `boolean` | 可选 | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | 可选 | `undefined` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### BaseDialog.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/dialog/BaseDialog.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `title` | `string` | 必需 | `—` |
| `width` | `number` | 可选 | `480` |
| `closable` | `boolean` | 可选 | `true` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `open`: `boolean`, `{ default: false }`

Slots: `header-actions`, `default`, `footer-start`, `footer`.
