
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
