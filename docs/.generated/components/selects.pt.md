
### BaseComboBox.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/combo-box/BaseComboBox.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `options` | `readonly SelectOption<T>[]` | Obrigatória | `—` |
| `placeholder` | `string` | Opcional | `''` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `T`, ``

Slots: `prefix`.

### BaseSearchSelect.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/search-select/BaseSearchSelect.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `options` | `readonly SearchOption[]` | Obrigatória | `—` |
| `placeholder` | `string` | Opcional | `undefined` |
| `allowCustom` | `boolean` | Opcional | `false` |
| `disabled` | `boolean` | Opcional | `false` |
| `invalid` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `search`: `[query: string]`

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`.
