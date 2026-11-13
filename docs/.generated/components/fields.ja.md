
### BaseTextField.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | 任意 | `'text'` |
| `placeholder` | `string` | 任意 | `undefined` |
| `size` | `ControlSize` | 任意 | `'md'` |
| `disabled` | `boolean` | 任意 | `false` |
| `readonly` | `boolean` | 任意 | `false` |
| `invalid` | `boolean` | 任意 | `false` |
| `revealable` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

### BaseSpinBox.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `min` | `number` | 任意 | `undefined` |
| `max` | `number` | 任意 | `undefined` |
| `step` | `number` | 任意 | `1` |
| `precision` | `number` | 任意 | `undefined` |
| `suffix` | `string` | 任意 | `undefined` |
| `size` | `ControlSize` | 任意 | `'md'` |
| `orientation` | `'vertical' &#124; 'horizontal'` | 任意 | `'vertical'` |
| `disabled` | `boolean` | 任意 | `false` |
| `readonly` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.
