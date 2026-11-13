
### BaseFloatingPanel.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/floating-panel/BaseFloatingPanel.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `title` | `string` | Required | `—` |
| `moveLabel` | `string` | Required | `—` |
| `closeLabel` | `string` | Required | `—` |
| `minimizeLabel` | `string` | Required | `—` |
| `restoreLabel` | `string` | Required | `—` |
| `resetLabel` | `string` | Required | `—` |
| `width` | `number` | Optional | `620` |
| `activationKey` | `number` | Optional | `0` |

Own events (native control attributes are forwarded according to the component):

- `viewportChange`: `[width: number]`

Models:

- `open`: `boolean`, `{ default: false }`

Slots: `default { minimized: minimized }`.
