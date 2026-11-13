
### BaseFloatingPanel.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/floating-panel/BaseFloatingPanel.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `title` | `string` | Obrigatória | `—` |
| `moveLabel` | `string` | Obrigatória | `—` |
| `closeLabel` | `string` | Obrigatória | `—` |
| `minimizeLabel` | `string` | Obrigatória | `—` |
| `restoreLabel` | `string` | Obrigatória | `—` |
| `resetLabel` | `string` | Obrigatória | `—` |
| `width` | `number` | Opcional | `620` |
| `activationKey` | `number` | Opcional | `0` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `viewportChange`: `[width: number]`

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: `default { minimized: minimized }`.
