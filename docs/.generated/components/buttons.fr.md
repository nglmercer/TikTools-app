
### BaseButton.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseButton.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `variant` | `'default' &#124; 'primary' &#124; 'danger' &#124; 'ghost'` | Facultative | `'default'` |
| `size` | `ControlSize` | Facultative | `'md'` |
| `type` | `'button' &#124; 'submit' &#124; 'reset'` | Facultative | `'button'` |
| `block` | `boolean` | Facultative | `false` |
| `disabled` | `boolean` | Facultative | `false` |
| `pressed` | `boolean` | Facultative | `undefined` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `icon`, `default`.

### BaseIconButton.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseIconButton.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoire | `—` |
| `hint` | `string` | Facultative | `undefined` |
| `variant` | `'default' &#124; 'ghost' &#124; 'primary' &#124; 'danger'` | Facultative | `'ghost'` |
| `size` | `ControlSize` | Facultative | `'md'` |
| `disabled` | `boolean` | Facultative | `false` |
| `pressed` | `boolean` | Facultative | `undefined` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `default`.
