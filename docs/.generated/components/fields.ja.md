
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

### BaseTextArea.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-area/BaseTextArea.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `rows` | `number` | 任意 | `4` |
| `placeholder` | `string` | 任意 | `undefined` |
| `maxLength` | `number` | 任意 | `undefined` |
| `monospace` | `boolean` | 任意 | `false` |
| `disabled` | `boolean` | 任意 | `false` |
| `readonly` | `boolean` | 任意 | `false` |
| `invalid` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseSlider.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/slider/BaseSlider.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `min` | `number` | 任意 | `0` |
| `max` | `number` | 任意 | `100` |
| `step` | `number` | 任意 | `1` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `number`, `{ default: 0 }`

Slots: —.

### BaseRadio.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/radio/BaseRadio.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `value` | `T` | 必須 | `—` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `T`, ``

Slots: `default`.

### BaseTagsInput.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tags-input/BaseTagsInput.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `placeholder` | `string` | 任意 | `undefined` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `string[]`, `{ default: () => [] }`

Slots: —.

### BaseDateField.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/date-field/BaseDateField.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `type` | `'date' &#124; 'time' &#124; 'datetime-local'` | 任意 | `'date'` |
| `min` | `string` | 任意 | `undefined` |
| `max` | `string` | 任意 | `undefined` |
| `size` | `ControlSize` | 任意 | `'md'` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseColorField.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/color-field/BaseColorField.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `string`, `{ default: '#000000' }`

Slots: —.

### BaseFileUpload.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/file-upload/BaseFileUpload.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `label` | `string` | 必須 | `—` |
| `hint` | `string` | 必須 | `—` |
| `accept` | `string` | 必須 | `—` |
| `maxBytes` | `number` | 必須 | `—` |
| `disabled` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `error`: `[message: string]`

モデル:

- `modelValue`: `File &#124; null`, `{ default: null }`

Slots: —.
