
### CollectionToolbar.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/collection-toolbar/CollectionToolbar.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoria | `—` |
| `count` | `string` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `search`, `default`, `actions`.

### BaseCheckbox.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/checkbox/BaseCheckbox.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Opcional | `false` |
| `indeterminate` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `boolean`, `{ default: false }`

Slots: `default`.
