
### BaseJsonView.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonView.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `value` | `unknown` | 任意 | `undefined` |
| `source` | `string` | 任意 | `undefined` |
| `label` | `string` | 任意 | `undefined` |
| `lineNumbers` | `boolean` | 任意 | `false` |
| `switchable` | `boolean` | 任意 | `false` |
| `copyable` | `boolean` | 任意 | `true` |
| `maxHeight` | `string &#124; number` | 任意 | `undefined` |
| `maxLines` | `number` | 任意 | `DEFAULT_MAX_LINES` |
| `maxChars` | `number` | 任意 | `DEFAULT_MAX_CHARS` |
| `sortKeys` | `boolean` | 任意 | `false` |
| `treeDepth` | `number` | 任意 | `DEFAULT_TREE_DEPTH` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `copy`: `[text: string]`

モデル:

- `mode`: `'code' &#124; 'tree'`, `{ default: 'code' }`
- `wrap`: `boolean`, `{ default: true }`

Slots: —.

### BaseJsonDiff.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `before` | `unknown` | 任意 | `undefined` |
| `after` | `unknown` | 任意 | `undefined` |
| `beforeLabel` | `string` | 任意 | `undefined` |
| `afterLabel` | `string` | 任意 | `undefined` |
| `label` | `string` | 任意 | `undefined` |
| `switchable` | `boolean` | 任意 | `false` |
| `context` | `number` | 任意 | `3` |
| `maxHeight` | `string &#124; number` | 任意 | `undefined` |
| `maxLines` | `number` | 任意 | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.
