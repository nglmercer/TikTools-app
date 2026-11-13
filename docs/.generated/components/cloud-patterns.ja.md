
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

### SyncPanel.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/features/sync-changes/ui/SyncPanel.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `adapter` | `SyncAdapter` | 必須 | `—` |
| `preparing` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `open`: `boolean`, `{ default: false }`

Slots: `preparation`.
