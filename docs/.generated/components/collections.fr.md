
### BaseList.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/list/BaseList.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `items` | `readonly T[]` | Obligatoire | `—` |
| `itemKey` | `(item: T) => K` | Obligatoire | `—` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `activate`: `[item: T]`

Modèles:

- `selected`: `K &#124; null`, `{ default: null }`

Slots: `default { item: item, selected: index === selectedIndex }`, `empty`.

### BaseTable.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/table/BaseTable.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `columns` | `readonly TableColumn[]` | Obligatoire | `—` |
| `rows` | `readonly T[]` | Obligatoire | `—` |
| `rowKey` | `(row: T) => K` | Obligatoire | `—` |
| `selectable` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `activate`: `[row: T]`

Modèles:

- `selected`: `K &#124; null`, `{ default: null }`
- `sort`: `TableSort &#124; null`, `{ default: null }`

Slots: `cell-${column.key} { row: row, value: cellValue(row, column.key) }`, `empty`.
