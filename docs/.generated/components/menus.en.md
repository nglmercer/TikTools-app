
### BaseMenu.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Required | `—` |
| `anchor` | `AnchorTarget &#124; null` | Required | `—` |
| `placement` | `Placement` | Optional | `'bottom-start'` |
| `focusFirst` | `boolean` | Optional | `undefined / Vue boolean false` |

Own events (native control attributes are forwarded according to the component):

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

Models:

- `open`: `boolean`, `{ default: false }`

Slots: —.
