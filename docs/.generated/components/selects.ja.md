
### BaseComboBox.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/combo-box/BaseComboBox.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `options` | `readonly SelectOption<T>[]` | 必須 | `—` |
| `placeholder` | `string` | 任意 | `''` |
| `size` | `ControlSize` | 任意 | `'md'` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `T`, ``

Slots: `prefix`.

### BaseSearchSelect.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/search-select/BaseSearchSelect.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `options` | `readonly SearchOption[]` | 必須 | `—` |
| `placeholder` | `string` | 任意 | `undefined` |
| `allowCustom` | `boolean` | 任意 | `false` |
| `disabled` | `boolean` | 任意 | `false` |
| `invalid` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `search`: `[query: string]`

モデル:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`.
