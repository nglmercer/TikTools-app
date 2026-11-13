
### BaseJsonView.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonView.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `value` | `unknown` | Facultative | `undefined` |
| `source` | `string` | Facultative | `undefined` |
| `label` | `string` | Facultative | `undefined` |
| `lineNumbers` | `boolean` | Facultative | `false` |
| `switchable` | `boolean` | Facultative | `false` |
| `copyable` | `boolean` | Facultative | `true` |
| `maxHeight` | `string &#124; number` | Facultative | `undefined` |
| `maxLines` | `number` | Facultative | `DEFAULT_MAX_LINES` |
| `maxChars` | `number` | Facultative | `DEFAULT_MAX_CHARS` |
| `sortKeys` | `boolean` | Facultative | `false` |
| `treeDepth` | `number` | Facultative | `DEFAULT_TREE_DEPTH` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `copy`: `[text: string]`

Modèles:

- `mode`: `'code' &#124; 'tree'`, `{ default: 'code' }`
- `wrap`: `boolean`, `{ default: true }`

Slots: —.

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
