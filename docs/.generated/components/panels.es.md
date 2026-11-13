
### BasePanel.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoria | `—` |
| `subtitle` | `string` | Opcional | `undefined` |
| `flush` | `boolean` | Opcional | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### BaseDialog.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/dialog/BaseDialog.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoria | `—` |
| `width` | `number` | Opcional | `480` |
| `closable` | `boolean` | Opcional | `true` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: `header-actions`, `default`, `footer-start`, `footer`.
