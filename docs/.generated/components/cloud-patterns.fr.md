
### BaseJsonDiff.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `before` | `unknown` | Facultative | `undefined` |
| `after` | `unknown` | Facultative | `undefined` |
| `beforeLabel` | `string` | Facultative | `undefined` |
| `afterLabel` | `string` | Facultative | `undefined` |
| `label` | `string` | Facultative | `undefined` |
| `switchable` | `boolean` | Facultative | `false` |
| `context` | `number` | Facultative | `3` |
| `maxHeight` | `string &#124; number` | Facultative | `undefined` |
| `maxLines` | `number` | Facultative | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.

### SyncPanel.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/features/sync-changes/ui/SyncPanel.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `adapter` | `SyncAdapter` | Obligatoire | `—` |
| `preparing` | `boolean` | Facultative | `false` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

Aucun événement propre déclaré.

Modèles:

- `open`: `boolean`, `{ default: false }`

Slots: `preparation`.
