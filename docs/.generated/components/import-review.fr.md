
### BaseImportDialog.vue

[Code source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/import-dialog/BaseImportDialog.vue)

| Propriété | Type | Présence | Valeur par défaut |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoire | `—` |
| `width` | `number` | Facultative | `720` |
| `tabs` | `readonly ImportTab[]` | Obligatoire | `—` |
| `hideTabs` | `boolean` | Facultative | `false` |
| `manualTab` | `string` | Facultative | `MANUAL_IMPORT_TAB` |
| `accept` | `string` | Facultative | `'.json,application/json'` |
| `maxBytes` | `number` | Facultative | `undefined` |
| `staged` | `readonly StagedImportItem[]` | Facultative | `() => []` |
| `stagedSource` | `string` | Facultative | `''` |
| `busy` | `boolean` | Facultative | `false` |
| `errors` | `readonly string[]` | Facultative | `() => []` |
| `success` | `string` | Facultative | `''` |
| `importLabel` | `string` | Facultative | `''` |
| `labels` | `ImportDialogLabels` | Facultative | `() => ({})` |

Événements propres (les attributs des contrôles natifs sont transmis selon le composant):

- `content`: `[text: string, source: string]`
- `error`: `[message: string]`
- `clear`: `[]`
- `import`: `[]`

Modèles:

- `open`: `boolean`, `{ default: false }`
- `tab`: `string`, `{ required: true }`

Slots: `tab`, `manual-lead`, `item`, `staged-extra`, `footer`.
