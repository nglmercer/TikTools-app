
### BaseButton.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseButton.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `variant` | `'default' &#124; 'primary' &#124; 'danger' &#124; 'ghost'` | 任意 | `'default'` |
| `size` | `ControlSize` | 任意 | `'md'` |
| `type` | `'button' &#124; 'submit' &#124; 'reset'` | 任意 | `'button'` |
| `block` | `boolean` | 任意 | `false` |
| `disabled` | `boolean` | 任意 | `false` |
| `pressed` | `boolean` | 任意 | `undefined` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `icon`, `default`.

### BaseIconButton.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseIconButton.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `label` | `string` | 必須 | `—` |
| `hint` | `string` | 任意 | `undefined` |
| `variant` | `'default' &#124; 'ghost' &#124; 'primary' &#124; 'danger'` | 任意 | `'ghost'` |
| `size` | `ControlSize` | 任意 | `'md'` |
| `disabled` | `boolean` | 任意 | `false` |
| `pressed` | `boolean` | 任意 | `undefined` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `default`.
