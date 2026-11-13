
### BaseAlert.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/alert/BaseAlert.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | Facultative | `'error'` |
| `title` | `string` | Obligatoire | `—` |
| `compact` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `default`.

### BaseEmptyState.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/empty-state/BaseEmptyState.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoire | `—` |
| `description` | `string` | Facultative | `undefined` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `icon`, `default`.

### BaseStatus.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/status/BaseStatus.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoire | `—` |
| `tone` | `StatusTone` | Facultative | `'neutral'` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: —.
