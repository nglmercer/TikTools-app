
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

### BaseMenuBar.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseMenuBar.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `menus` | `readonly MenuBarMenu[]` | 必須 | `—` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `select`: `[action: MenuAction]`

モデル:

独自の v-model はありません。

Slots: —.

### BaseContextMenu.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/menu/BaseContextMenu.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `items` | `readonly MenuEntry[]` | 必須 | `—` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `open`: `[event: MouseEvent &#124; KeyboardEvent]`
- `select`: `[action: MenuAction]`

モデル:

独自の v-model はありません。

Slots: `default`.
