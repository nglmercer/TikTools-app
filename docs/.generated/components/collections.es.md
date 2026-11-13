
### BaseList.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/list/BaseList.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `items` | `readonly T[]` | Obligatoria | `—` |
| `itemKey` | `(item: T) => K` | Obligatoria | `—` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `activate`: `[item: T]`

Modelos:

- `selected`: `K &#124; null`, `{ default: null }`

Slots: `default`, `empty`.

### BaseTable.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/table/BaseTable.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `columns` | `readonly TableColumn[]` | Obligatoria | `—` |
| `rows` | `readonly T[]` | Obligatoria | `—` |
| `rowKey` | `(row: T) => K` | Obligatoria | `—` |
| `selectable` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `activate`: `[row: T]`

Modelos:

- `selected`: `K &#124; null`, `{ default: null }`
- `sort`: `TableSort &#124; null`, `{ default: null }`

Slots: `cell-${column.key}`, `empty`.
