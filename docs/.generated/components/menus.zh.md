
### BaseMenu.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | 必需 | `—` |
| `anchor` | `AnchorTarget &#124; null` | 必需 | `—` |
| `placement` | `Placement` | 可选 | `'bottom-start'` |
| `focusFirst` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

模型:

- `open`: `boolean`, `{ default: false }`

Slots: —.
