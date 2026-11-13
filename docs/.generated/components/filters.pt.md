
### CollectionToolbar.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/collection-toolbar/CollectionToolbar.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `label` | `string` | Obrigatória | `—` |
| `count` | `string` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `search`, `default`, `actions`.

### BaseCheckbox.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/checkbox/BaseCheckbox.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Opcional | `false` |
| `indeterminate` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `boolean`, `{ default: false }`

Slots: `default`.
