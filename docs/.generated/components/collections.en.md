
### BaseList.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/list/BaseList.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `items` | `readonly T[]` | Required | `—` |
| `itemKey` | `(item: T) => K` | Required | `—` |

Own events (native control attributes are forwarded according to the component):

- `activate`: `[item: T]`

Models:

- `selected`: `K &#124; null`, `{ default: null }`

Slots: `default { item: item, selected: index === selectedIndex }`, `empty`.

### BaseTable.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/table/BaseTable.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `columns` | `readonly TableColumn[]` | Required | `—` |
| `rows` | `readonly T[]` | Required | `—` |
| `rowKey` | `(row: T) => K` | Required | `—` |
| `selectable` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

- `activate`: `[row: T]`

Models:

- `selected`: `K &#124; null`, `{ default: null }`
- `sort`: `TableSort &#124; null`, `{ default: null }`

Slots: `cell-${column.key} { row: row, value: cellValue(row, column.key) }`, `empty`.
