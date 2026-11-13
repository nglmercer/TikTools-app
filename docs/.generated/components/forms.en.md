
### BaseFormRow.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormRow.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `label` | `string` | Required | `—` |
| `for` | `string` | Optional | `undefined` |
| `hint` | `string` | Optional | `undefined` |
| `error` | `string` | Optional | `undefined` |
| `required` | `boolean` | Optional | `false` |
| `stacked` | `boolean` | Optional | `false` |
| `group` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `default { control-attrs: { id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required &#124;&#124; undefined } }`.

### BaseFormGrid.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormGrid.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| — | — | — | — |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `default`.

### BaseSchemaForm.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/schema-form/BaseSchemaForm.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `schema` | `JsonObject` | Required | `—` |
| `uiHints` | `JsonObject` | Optional | `undefined` |
| `fieldOptions` | `Readonly<Record<string, readonly SchemaOption[]>>` | Optional | `() => ({})` |
| `fieldErrors` | `Readonly<Record<string, string>>` | Optional | `() => ({})` |
| `suggestionsFor` | `(key: string) => readonly SchemaSuggestion[]` | Optional | `undefined` |
| `openMediaPicker` | `OpenMediaPicker` | Optional | `undefined` |
| `disabled` | `boolean` | Optional | `false` |
| `switches` | `boolean` | Optional | `false` |
| `compact` | `boolean` | Optional | `false` |
| `fieldIcons` | `Readonly<Record<string, Component>>` | Optional | `() => ({})` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `JsonObject`, `{ required: true }`

Slots: —.

### SettingsRow.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/settings-row/SettingsRow.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `label` | `string` | Required | `—` |
| `description` | `string` | Optional | `undefined` |
| `for` | `string` | Optional | `undefined` |
| `toggle` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `status`, `default`, `details`.
