
### BaseList.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/list/BaseList.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `items` | `readonly T[]` | 必需 | `—` |
| `itemKey` | `(item: T) => K` | 必需 | `—` |

组件自身的事件（原生控件属性按组件约定透传）:

- `activate`: `[item: T]`

模型:

- `selected`: `K &#124; null`, `{ default: null }`

Slots: `default { item: item, selected: index === selectedIndex }`, `empty`.

### BaseTable.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/table/BaseTable.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `columns` | `readonly TableColumn[]` | 必需 | `—` |
| `rows` | `readonly T[]` | 必需 | `—` |
| `rowKey` | `(row: T) => K` | 必需 | `—` |
| `selectable` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

- `activate`: `[row: T]`

模型:

- `selected`: `K &#124; null`, `{ default: null }`
- `sort`: `TableSort &#124; null`, `{ default: null }`

Slots: `cell-${column.key} { row: row, value: cellValue(row, column.key) }`, `empty`.
