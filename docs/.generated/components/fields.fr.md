
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
