
### BasePanel.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoire | `—` |
| `subtitle` | `string` | Facultative | `undefined` |
| `flush` | `boolean` | Facultative | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Facultative | `undefined` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

Aucun v-model propre.

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### BaseDialog.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/dialog/BaseDialog.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoire | `—` |
| `width` | `number` | Facultative | `480` |
| `closable` | `boolean` | Facultative | `true` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `open`: `boolean`, `{ default: false }`

Slots: `header-actions`, `default`, `footer-start`, `footer`.
