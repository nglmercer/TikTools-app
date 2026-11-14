
### BaseTextField.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | Facultative | `'text'` |
| `placeholder` | `string` | Facultative | `undefined` |
| `size` | `ControlSize` | Facultative | `'md'` |
| `disabled` | `boolean` | Facultative | `false` |
| `readonly` | `boolean` | Facultative | `false` |
| `invalid` | `boolean` | Facultative | `false` |
| `revealable` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

### BaseSpinBox.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `min` | `number` | Facultative | `undefined` |
| `max` | `number` | Facultative | `undefined` |
| `step` | `number` | Facultative | `1` |
| `precision` | `number` | Facultative | `undefined` |
| `suffix` | `string` | Facultative | `undefined` |
| `size` | `ControlSize` | Facultative | `'md'` |
| `orientation` | `'vertical' &#124; 'horizontal'` | Facultative | `'vertical'` |
| `disabled` | `boolean` | Facultative | `false` |
| `readonly` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.

### BaseTextArea.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-area/BaseTextArea.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `rows` | `number` | Facultative | `4` |
| `placeholder` | `string` | Facultative | `undefined` |
| `maxLength` | `number` | Facultative | `undefined` |
| `monospace` | `boolean` | Facultative | `false` |
| `disabled` | `boolean` | Facultative | `false` |
| `readonly` | `boolean` | Facultative | `false` |
| `invalid` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseSlider.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/slider/BaseSlider.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `min` | `number` | Facultative | `0` |
| `max` | `number` | Facultative | `100` |
| `step` | `number` | Facultative | `1` |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `number`, `{ default: 0 }`

Slots: —.

### BaseRadio.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/radio/BaseRadio.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `value` | `T` | Obligatoire | `—` |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `T`, ``

Slots: `default`.

### BaseTagsInput.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tags-input/BaseTagsInput.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `placeholder` | `string` | Facultative | `undefined` |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `string[]`, `{ default: () => [] }`

Slots: —.

### BaseDateField.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/date-field/BaseDateField.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `type` | `'date' &#124; 'time' &#124; 'datetime-local'` | Facultative | `'date'` |
| `min` | `string` | Facultative | `undefined` |
| `max` | `string` | Facultative | `undefined` |
| `size` | `ControlSize` | Facultative | `'md'` |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseColorField.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/color-field/BaseColorField.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `string`, `{ default: '#000000' }`

Slots: —.

### BaseFileUpload.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/file-upload/BaseFileUpload.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoire | `—` |
| `hint` | `string` | Obligatoire | `—` |
| `accept` | `string` | Obligatoire | `—` |
| `maxBytes` | `number` | Obligatoire | `—` |
| `disabled` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `error`: `[message: string]`

Modèles:

- `modelValue`: `File &#124; null`, `{ default: null }`

Slots: —.
