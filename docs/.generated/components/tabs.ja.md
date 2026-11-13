
### BaseTabs.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tabs/BaseTabs.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `tabs` | `readonly TabOption<T>[]` | 必須 | `—` |
| `label` | `string` | 任意 | `undefined` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `T`, `{ required: true }`

Slots: `bar`, `default`.

### BaseToolbar.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/toolbar/BaseToolbar.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | 任意 | `'horizontal'` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `default`.
