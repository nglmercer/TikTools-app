
### BaseMenu.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Obligatoria | `—` |
| `anchor` | `AnchorTarget &#124; null` | Obligatoria | `—` |
| `placement` | `Placement` | Opcional | `'bottom-start'` |
| `focusFirst` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: —.

### BaseMenuBar.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenuBar.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `menus` | `readonly MenuBarMenu[]` | Obligatoria | `—` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `select`: `[action: MenuAction]`

Modelos:

Sin v-model propio.

Slots: —.

### BaseContextMenu.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseContextMenu.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Obligatoria | `—` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `open`: `[event: MouseEvent &#124; KeyboardEvent]`
- `select`: `[action: MenuAction]`

Modelos:

Sin v-model propio.

Slots: `default`.
