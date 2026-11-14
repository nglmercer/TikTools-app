
### BaseFloatingPanel.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/floating-panel/BaseFloatingPanel.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `title` | `string` | 必需 | `—` |
| `moveLabel` | `string` | 必需 | `—` |
| `closeLabel` | `string` | 必需 | `—` |
| `minimizeLabel` | `string` | 必需 | `—` |
| `restoreLabel` | `string` | 必需 | `—` |
| `resetLabel` | `string` | 必需 | `—` |
| `width` | `number` | 可选 | `620` |
| `activationKey` | `number` | 可选 | `0` |

组件自身的事件（原生控件属性按组件约定透传）:

- `viewportChange`: `[width: number]`

模型:

- `open`: `boolean`, `{ default: false }`

Slots: `default { minimized: minimized }`.
