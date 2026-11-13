
### CollectionToolbar.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/collection-toolbar/CollectionToolbar.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoire | `—` |
| `count` | `string` | Facultative | `undefined` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `search`, `default`, `actions`.

### BaseCheckbox.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/checkbox/BaseCheckbox.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Facultative | `false` |
| `indeterminate` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `boolean`, `{ default: false }`

Slots: `default`.
