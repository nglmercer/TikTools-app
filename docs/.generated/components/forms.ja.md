
### BaseFormRow.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormRow.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `label` | `string` | 必須 | `—` |
| `for` | `string` | 任意 | `undefined` |
| `hint` | `string` | 任意 | `undefined` |
| `error` | `string` | 任意 | `undefined` |
| `required` | `boolean` | 任意 | `false` |
| `stacked` | `boolean` | 任意 | `false` |
| `group` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `default { control-attrs: { id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required &#124;&#124; undefined } }`.

### BaseFormGrid.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormGrid.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| — | — | — | — |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `default`.

### BaseSchemaForm.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/schema-form/BaseSchemaForm.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `schema` | `JsonObject` | 必須 | `—` |
| `uiHints` | `JsonObject` | 任意 | `undefined` |
| `fieldOptions` | `Readonly<Record<string, readonly SchemaOption[]>>` | 任意 | `() => ({})` |
| `fieldErrors` | `Readonly<Record<string, string>>` | 任意 | `() => ({})` |
| `suggestionsFor` | `(key: string) => readonly SchemaSuggestion[]` | 任意 | `undefined` |
| `openMediaPicker` | `OpenMediaPicker` | 任意 | `undefined` |
| `disabled` | `boolean` | 任意 | `false` |
| `switches` | `boolean` | 任意 | `false` |
| `compact` | `boolean` | 任意 | `false` |
| `fieldIcons` | `Readonly<Record<string, Component>>` | 任意 | `() => ({})` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

- `modelValue`: `JsonObject`, `{ required: true }`

Slots: —.

### SettingsRow.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/settings-row/SettingsRow.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `label` | `string` | 必須 | `—` |
| `description` | `string` | 任意 | `undefined` |
| `for` | `string` | 任意 | `undefined` |
| `toggle` | `boolean` | 任意 | `false` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

独自のイベントは宣言されていません。

モデル:

独自の v-model はありません。

Slots: `status`, `default`, `details`.
