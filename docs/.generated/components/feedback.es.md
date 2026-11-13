
### BaseAlert.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/alert/BaseAlert.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | Opcional | `'error'` |
| `title` | `string` | Obligatoria | `—` |
| `compact` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `default`.

### BaseEmptyState.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/empty-state/BaseEmptyState.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoria | `—` |
| `description` | `string` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `icon`, `default`.

### BaseStatus.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/status/BaseStatus.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoria | `—` |
| `tone` | `StatusTone` | Opcional | `'neutral'` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: —.

### BaseNotification.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/notification/BaseNotification.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `message` | `string` | Obligatoria | `—` |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | Opcional | `'info'` |
| `actionLabel` | `string` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `action`: `[]`
- `dismiss`: `[]`

Modelos:

Sin v-model propio.

Slots: —.
