
### BaseMenu.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenu.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | 必須 | `—` |
| `anchor` | `AnchorTarget &#124; null` | 必須 | `—` |
| `placement` | `Placement` | 任意 | `'bottom-start'` |
| `focusFirst` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `select`: `[action: MenuAction]`
- `navigate`: `[direction: 1 &#124; -1]`

モデル:

- `open`: `boolean`, `{ default: false }`

Slots: —.
