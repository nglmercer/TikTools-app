
### BaseSplitter.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/splitter/BaseSplitter.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | 任意 | `'horizontal'` |
| `min` | `number` | 任意 | `80` |
| `max` | `number` | 任意 | `Infinity` |
| `step` | `number` | 任意 | `16` |
| `sized` | `'start' &#124; 'end'` | 任意 | `'start'` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `size`: `number`, `{ default: 240 }`

Slots: `start`, `end`.

### MasterDetailLayout.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/layout/MasterDetailLayout.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `narrow` | `boolean` | 必須 | `—` |
| `detailOpen` | `boolean` | 必須 | `—` |
| `backLabel` | `string` | 必須 | `—` |
| `detailTitle` | `string` | 任意 | `''` |
| `min` | `number` | 任意 | `240` |
| `max` | `number` | 任意 | `900` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `back`: `[]`

モデル:

- `size`: `number`, `{ default: 300 }`

Slots: `master`, `detail-actions`, `detail`.
