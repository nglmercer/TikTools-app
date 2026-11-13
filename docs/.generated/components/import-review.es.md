
### BaseImportDialog.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/import-dialog/BaseImportDialog.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoria | `—` |
| `width` | `number` | Opcional | `720` |
| `tabs` | `readonly ImportTab[]` | Obligatoria | `—` |
| `hideTabs` | `boolean` | Opcional | `false` |
| `manualTab` | `string` | Opcional | `MANUAL_IMPORT_TAB` |
| `accept` | `string` | Opcional | `'.json,application/json'` |
| `maxBytes` | `number` | Opcional | `undefined` |
| `staged` | `readonly StagedImportItem[]` | Opcional | `() => []` |
| `stagedSource` | `string` | Opcional | `''` |
| `busy` | `boolean` | Opcional | `false` |
| `errors` | `readonly string[]` | Opcional | `() => []` |
| `success` | `string` | Opcional | `''` |
| `importLabel` | `string` | Opcional | `''` |
| `labels` | `ImportDialogLabels` | Opcional | `() => ({})` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `content`: `[text: string, source: string]`
- `error`: `[message: string]`
- `clear`: `[]`
- `import`: `[]`

Modelos:

- `open`: `boolean`, `{ default: false }`
- `tab`: `string`, `{ required: true }`

Slots: `tab`, `manual-lead`, `item`, `staged-extra`, `footer`.
