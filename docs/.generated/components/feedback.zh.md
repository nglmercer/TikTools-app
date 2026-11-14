
### BaseAlert.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/alert/BaseAlert.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | 可选 | `'error'` |
| `title` | `string` | 必需 | `—` |
| `compact` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `default`.

### BaseEmptyState.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/empty-state/BaseEmptyState.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `title` | `string` | 必需 | `—` |
| `description` | `string` | 可选 | `undefined` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `icon`, `default`.

### BaseStatus.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/status/BaseStatus.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `label` | `string` | 必需 | `—` |
| `tone` | `StatusTone` | 可选 | `'neutral'` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: —.

### BaseNotification.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/notification/BaseNotification.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `message` | `string` | 必需 | `—` |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | 可选 | `'info'` |
| `actionLabel` | `string` | 可选 | `undefined` |

组件自身的事件（原生控件属性按组件约定透传）:

- `action`: `[]`
- `dismiss`: `[]`

模型:

没有组件自身的 v-model。

Slots: —.
