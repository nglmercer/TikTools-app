
### CollectionToolbar.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/collection-toolbar/CollectionToolbar.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `label` | `string` | 必須 | `—` |
| `count` | `string` | 任意 | `undefined` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `search`, `default`, `actions`.

### BaseCheckbox.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/checkbox/BaseCheckbox.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `disabled` | `boolean` | 任意 | `false` |
| `indeterminate` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `boolean`, `{ default: false }`

Slots: `default`.
