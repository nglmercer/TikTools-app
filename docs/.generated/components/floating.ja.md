
### BaseFloatingPanel.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/floating-panel/BaseFloatingPanel.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `title` | `string` | 必須 | `—` |
| `moveLabel` | `string` | 必須 | `—` |
| `closeLabel` | `string` | 必須 | `—` |
| `minimizeLabel` | `string` | 必須 | `—` |
| `restoreLabel` | `string` | 必須 | `—` |
| `resetLabel` | `string` | 必須 | `—` |
| `width` | `number` | 任意 | `620` |
| `activationKey` | `number` | 任意 | `0` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `viewportChange`: `[width: number]`

モデル:

- `open`: `boolean`, `{ default: false }`

Slots: `default`.
