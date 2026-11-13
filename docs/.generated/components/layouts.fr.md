
### BaseSplitter.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/splitter/BaseSplitter.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Facultative | `'horizontal'` |
| `min` | `number` | Facultative | `80` |
| `max` | `number` | Facultative | `Infinity` |
| `step` | `number` | Facultative | `16` |
| `sized` | `'start' &#124; 'end'` | Facultative | `'start'` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `size`: `number`, `{ default: 240 }`

Slots: `start`, `end`.

### MasterDetailLayout.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/layout/MasterDetailLayout.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `narrow` | `boolean` | Obligatoire | `—` |
| `detailOpen` | `boolean` | Obligatoire | `—` |
| `backLabel` | `string` | Obligatoire | `—` |
| `detailTitle` | `string` | Facultative | `''` |
| `min` | `number` | Facultative | `240` |
| `max` | `number` | Facultative | `900` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `back`: `[]`

Modèles:

- `size`: `number`, `{ default: 300 }`

Slots: `master`, `detail-actions`, `detail`.
