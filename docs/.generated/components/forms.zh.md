
### BaseFormRow.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormRow.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `label` | `string` | 必需 | `—` |
| `for` | `string` | 可选 | `undefined` |
| `hint` | `string` | 可选 | `undefined` |
| `error` | `string` | 可选 | `undefined` |
| `required` | `boolean` | 可选 | `false` |
| `stacked` | `boolean` | 可选 | `false` |
| `group` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `default { control-attrs: { id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required &#124;&#124; undefined } }`.

### BaseFormGrid.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormGrid.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| — | — | — | — |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `default`.

### BaseSchemaForm.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/schema-form/BaseSchemaForm.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `schema` | `JsonObject` | 必需 | `—` |
| `uiHints` | `JsonObject` | 可选 | `undefined` |
| `fieldOptions` | `Readonly<Record<string, readonly SchemaOption[]>>` | 可选 | `() => ({})` |
| `fieldErrors` | `Readonly<Record<string, string>>` | 可选 | `() => ({})` |
| `suggestionsFor` | `(key: string) => readonly SchemaSuggestion[]` | 可选 | `undefined` |
| `openMediaPicker` | `OpenMediaPicker` | 可选 | `undefined` |
| `disabled` | `boolean` | 可选 | `false` |
| `switches` | `boolean` | 可选 | `false` |
| `compact` | `boolean` | 可选 | `false` |
| `fieldIcons` | `Readonly<Record<string, Component>>` | 可选 | `() => ({})` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `modelValue`: `JsonObject`, `{ required: true }`

Slots: —.

### SettingsRow.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/settings-row/SettingsRow.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `label` | `string` | 必需 | `—` |
| `description` | `string` | 可选 | `undefined` |
| `for` | `string` | 可选 | `undefined` |
| `toggle` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

没有组件自身的 v-model。

Slots: `status`, `default`, `details`.
