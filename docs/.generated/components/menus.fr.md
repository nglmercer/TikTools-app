
### BaseMenu.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Obligatoire | `—` |
| `anchor` | `AnchorTarget &#124; null` | Obligatoire | `—` |
| `placement` | `Placement` | Facultative | `'bottom-start'` |
| `focusFirst` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

Modèles:

- `open`: `boolean`, `{ default: false }`

Slots: —.
