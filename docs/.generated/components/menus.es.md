
### BaseMenu.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Obligatoria | `—` |
| `anchor` | `AnchorTarget &#124; null` | Obligatoria | `—` |
| `placement` | `Placement` | Opcional | `'bottom-start'` |
| `focusFirst` | `boolean` | Opcional | `undefined / Vue boolean false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: —.
