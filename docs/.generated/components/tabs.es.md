
### BaseTabs.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tabs/BaseTabs.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `tabs` | `readonly TabOption<T>[]` | Obligatoria | `—` |
| `label` | `string` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `T`, `{ required: true }`

Slots: `bar`, `default`.

### BaseToolbar.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/toolbar/BaseToolbar.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Opcional | `'horizontal'` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `default`.
