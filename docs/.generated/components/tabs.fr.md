
### BaseTabs.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tabs/BaseTabs.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `tabs` | `readonly TabOption<T>[]` | Obligatoire | `—` |
| `label` | `string` | Facultative | `undefined` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `modelValue`: `T`, `{ required: true }`

Slots: `bar`, `default { value: model }`.

### BaseToolbar.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/toolbar/BaseToolbar.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Facultative | `'horizontal'` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `default`.
