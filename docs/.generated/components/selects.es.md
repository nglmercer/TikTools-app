
### BaseComboBox.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/combo-box/BaseComboBox.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `options` | `readonly SelectOption<T>[]` | Obligatoria | `—` |
| `placeholder` | `string` | Opcional | `''` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `T`, ``

Slots: `prefix`.

### BaseSearchSelect.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/search-select/BaseSearchSelect.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `options` | `readonly SearchOption[]` | Obligatoria | `—` |
| `placeholder` | `string` | Opcional | `undefined` |
| `allowCustom` | `boolean` | Opcional | `false` |
| `disabled` | `boolean` | Opcional | `false` |
| `invalid` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `search`: `[query: string]`

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`.
