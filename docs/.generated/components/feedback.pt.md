
### BaseAlert.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/alert/BaseAlert.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | Opcional | `'error'` |
| `title` | `string` | Obrigatória | `—` |
| `compact` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `default`.

### BaseEmptyState.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/empty-state/BaseEmptyState.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `title` | `string` | Obrigatória | `—` |
| `description` | `string` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `icon`, `default`.

### BaseStatus.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/status/BaseStatus.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `label` | `string` | Obrigatória | `—` |
| `tone` | `StatusTone` | Opcional | `'neutral'` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: —.
