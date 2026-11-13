
### BasePanel.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `title` | `string` | 必須 | `—` |
| `subtitle` | `string` | 任意 | `undefined` |
| `flush` | `boolean` | 任意 | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | 任意 | `undefined` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### BaseDialog.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/dialog/BaseDialog.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `title` | `string` | 必須 | `—` |
| `width` | `number` | 任意 | `480` |
| `closable` | `boolean` | 任意 | `true` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `open`: `boolean`, `{ default: false }`

Slots: `header-actions`, `default`, `footer-start`, `footer`.
