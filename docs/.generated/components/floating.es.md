
### BaseFloatingPanel.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/floating-panel/BaseFloatingPanel.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoria | `—` |
| `moveLabel` | `string` | Obligatoria | `—` |
| `closeLabel` | `string` | Obligatoria | `—` |
| `minimizeLabel` | `string` | Obligatoria | `—` |
| `restoreLabel` | `string` | Obligatoria | `—` |
| `resetLabel` | `string` | Obligatoria | `—` |
| `width` | `number` | Opcional | `620` |
| `activationKey` | `number` | Opcional | `0` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `viewportChange`: `[width: number]`

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: `default { minimized: minimized }`.
