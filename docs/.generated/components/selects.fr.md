
### BaseComboBox.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/combo-box/BaseComboBox.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `options` | `readonly SelectOption<T>[]` | Obligatoire | `—` |
| `placeholder` | `string` | Facultative | `''` |
| `size` | `ControlSize` | Facultative | `'md'` |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `T`, ``

Slots: `prefix`.

### BaseSearchSelect.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/search-select/BaseSearchSelect.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `options` | `readonly SearchOption[]` | Obligatoire | `—` |
| `placeholder` | `string` | Facultative | `undefined` |
| `allowCustom` | `boolean` | Facultative | `false` |
| `disabled` | `boolean` | Facultative | `false` |
| `invalid` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `search`: `[query: string]`

Modèles:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`.
