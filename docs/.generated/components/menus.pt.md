
### BaseMenu.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Obrigatória | `—` |
| `anchor` | `AnchorTarget &#124; null` | Obrigatória | `—` |
| `placement` | `Placement` | Opcional | `'bottom-start'` |
| `focusFirst` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: —.

### BaseMenuBar.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenuBar.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `menus` | `readonly MenuBarMenu[]` | Obrigatória | `—` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `select`: `[action: MenuAction]`

Modelos:

Sem v-model próprio.

Slots: —.

### BaseContextMenu.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseContextMenu.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | Obrigatória | `—` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `open`: `[event: MouseEvent &#124; KeyboardEvent]`
- `select`: `[action: MenuAction]`

Modelos:

Sem v-model próprio.

Slots: `default`.
