
### BaseFloatingPanel.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/floating-panel/BaseFloatingPanel.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoire | `—` |
| `moveLabel` | `string` | Obligatoire | `—` |
| `closeLabel` | `string` | Obligatoire | `—` |
| `minimizeLabel` | `string` | Obligatoire | `—` |
| `restoreLabel` | `string` | Obligatoire | `—` |
| `resetLabel` | `string` | Obligatoire | `—` |
| `width` | `number` | Facultative | `620` |
| `activationKey` | `number` | Facultative | `0` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `viewportChange`: `[width: number]`

Modèles:

- `open`: `boolean`, `{ default: false }`

Slots: `default { minimized: minimized }`.
