
### BaseFormRow.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormRow.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `label` | `string` | Obrigatória | `—` |
| `for` | `string` | Opcional | `undefined` |
| `hint` | `string` | Opcional | `undefined` |
| `error` | `string` | Opcional | `undefined` |
| `required` | `boolean` | Opcional | `false` |
| `stacked` | `boolean` | Opcional | `false` |
| `group` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `default { control-attrs: { id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required &#124;&#124; undefined } }`.

### BaseFormGrid.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormGrid.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| — | — | — | — |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `default`.

### BaseSchemaForm.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/schema-form/BaseSchemaForm.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `schema` | `JsonObject` | Obrigatória | `—` |
| `uiHints` | `JsonObject` | Opcional | `undefined` |
| `fieldOptions` | `Readonly<Record<string, readonly SchemaOption[]>>` | Opcional | `() => ({})` |
| `fieldErrors` | `Readonly<Record<string, string>>` | Opcional | `() => ({})` |
| `suggestionsFor` | `(key: string) => readonly SchemaSuggestion[]` | Opcional | `undefined` |
| `openMediaPicker` | `OpenMediaPicker` | Opcional | `undefined` |
| `disabled` | `boolean` | Opcional | `false` |
| `switches` | `boolean` | Opcional | `false` |
| `compact` | `boolean` | Opcional | `false` |
| `fieldIcons` | `Readonly<Record<string, Component>>` | Opcional | `() => ({})` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `JsonObject`, `{ required: true }`

Slots: —.

### SettingsRow.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/settings-row/SettingsRow.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `label` | `string` | Obrigatória | `—` |
| `description` | `string` | Opcional | `undefined` |
| `for` | `string` | Opcional | `undefined` |
| `toggle` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `status`, `default`, `details`.
