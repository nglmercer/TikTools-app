
### BaseList.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/list/BaseList.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `items` | `readonly T[]` | Obrigatória | `—` |
| `itemKey` | `(item: T) => K` | Obrigatória | `—` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `activate`: `[item: T]`

Modelos:

- `selected`: `K &#124; null`, `{ default: null }`

Slots: `default { item: item, selected: index === selectedIndex }`, `empty`.

### BaseTable.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/table/BaseTable.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `columns` | `readonly TableColumn[]` | Obrigatória | `—` |
| `rows` | `readonly T[]` | Obrigatória | `—` |
| `rowKey` | `(row: T) => K` | Obrigatória | `—` |
| `selectable` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `activate`: `[row: T]`

Modelos:

- `selected`: `K &#124; null`, `{ default: null }`
- `sort`: `TableSort &#124; null`, `{ default: null }`

Slots: `cell-${column.key} { row: row, value: cellValue(row, column.key) }`, `empty`.
