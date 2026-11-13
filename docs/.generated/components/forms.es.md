
### BaseFormRow.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormRow.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoria | `—` |
| `for` | `string` | Opcional | `undefined` |
| `hint` | `string` | Opcional | `undefined` |
| `error` | `string` | Opcional | `undefined` |
| `required` | `boolean` | Opcional | `false` |
| `stacked` | `boolean` | Opcional | `false` |
| `group` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `default { control-attrs: { id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required &#124;&#124; undefined } }`.

### BaseFormGrid.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormGrid.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| — | — | — | — |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `default`.

### BaseSchemaForm.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/schema-form/BaseSchemaForm.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `schema` | `JsonObject` | Obligatoria | `—` |
| `uiHints` | `JsonObject` | Opcional | `undefined` |
| `fieldOptions` | `Readonly<Record<string, readonly SchemaOption[]>>` | Opcional | `() => ({})` |
| `fieldErrors` | `Readonly<Record<string, string>>` | Opcional | `() => ({})` |
| `suggestionsFor` | `(key: string) => readonly SchemaSuggestion[]` | Opcional | `undefined` |
| `openMediaPicker` | `OpenMediaPicker` | Opcional | `undefined` |
| `disabled` | `boolean` | Opcional | `false` |
| `switches` | `boolean` | Opcional | `false` |
| `compact` | `boolean` | Opcional | `false` |
| `fieldIcons` | `Readonly<Record<string, Component>>` | Opcional | `() => ({})` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `JsonObject`, `{ required: true }`

Slots: —.

### SettingsRow.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/settings-row/SettingsRow.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoria | `—` |
| `description` | `string` | Opcional | `undefined` |
| `for` | `string` | Opcional | `undefined` |
| `toggle` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `status`, `default`, `details`.
