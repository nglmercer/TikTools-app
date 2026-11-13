
### BasePanel.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `title` | `string` | Obrigatória | `—` |
| `subtitle` | `string` | Opcional | `undefined` |
| `flush` | `boolean` | Opcional | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### BaseDialog.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/dialog/BaseDialog.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `title` | `string` | Obrigatória | `—` |
| `width` | `number` | Opcional | `480` |
| `closable` | `boolean` | Opcional | `true` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: `header-actions`, `default`, `footer-start`, `footer`.
