
### BaseFormRow.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormRow.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoire | `—` |
| `for` | `string` | Facultative | `undefined` |
| `hint` | `string` | Facultative | `undefined` |
| `error` | `string` | Facultative | `undefined` |
| `required` | `boolean` | Facultative | `false` |
| `stacked` | `boolean` | Facultative | `false` |
| `group` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `default { control-attrs: { id: controlId, 'aria-describedby': describedBy, 'aria-invalid': error ? true : undefined, 'aria-required': required &#124;&#124; undefined } }`.

### BaseFormGrid.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/form/BaseFormGrid.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| — | — | — | — |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `default`.

### BaseSchemaForm.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/schema-form/BaseSchemaForm.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `schema` | `JsonObject` | Obligatoire | `—` |
| `uiHints` | `JsonObject` | Facultative | `undefined` |
| `fieldOptions` | `Readonly<Record<string, readonly SchemaOption[]>>` | Facultative | `() => ({})` |
| `fieldErrors` | `Readonly<Record<string, string>>` | Facultative | `() => ({})` |
| `suggestionsFor` | `(key: string) => readonly SchemaSuggestion[]` | Facultative | `undefined` |
| `openMediaPicker` | `OpenMediaPicker` | Facultative | `undefined` |
| `disabled` | `boolean` | Facultative | `false` |
| `switches` | `boolean` | Facultative | `false` |
| `compact` | `boolean` | Facultative | `false` |
| `fieldIcons` | `Readonly<Record<string, Component>>` | Facultative | `() => ({})` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `JsonObject`, `{ required: true }`

Slots: —.

### SettingsRow.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/settings-row/SettingsRow.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoire | `—` |
| `description` | `string` | Facultative | `undefined` |
| `for` | `string` | Facultative | `undefined` |
| `toggle` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `status`, `default`, `details`.
