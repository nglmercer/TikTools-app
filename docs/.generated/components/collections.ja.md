
### BaseList.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/list/BaseList.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `items` | `readonly T[]` | 必須 | `—` |
| `itemKey` | `(item: T) => K` | 必須 | `—` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `activate`: `[item: T]`

モデル:

- `selected`: `K &#124; null`, `{ default: null }`

Slots: `default { item: item, selected: index === selectedIndex }`, `empty`.

### BaseTable.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/table/BaseTable.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `columns` | `readonly TableColumn[]` | 必須 | `—` |
| `rows` | `readonly T[]` | 必須 | `—` |
| `rowKey` | `(row: T) => K` | 必須 | `—` |
| `selectable` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `activate`: `[row: T]`

モデル:

- `selected`: `K &#124; null`, `{ default: null }`
- `sort`: `TableSort &#124; null`, `{ default: null }`

Slots: `cell-${column.key} { row: row, value: cellValue(row, column.key) }`, `empty`.
