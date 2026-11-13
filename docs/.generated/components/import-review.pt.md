
### BaseImportDialog.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/import-dialog/BaseImportDialog.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `title` | `string` | Obrigatória | `—` |
| `width` | `number` | Opcional | `720` |
| `tabs` | `readonly ImportTab[]` | Obrigatória | `—` |
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

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `content`: `[text: string, source: string]`
- `error`: `[message: string]`
- `clear`: `[]`
- `import`: `[]`

Modelos:

- `open`: `boolean`, `{ default: false }`
- `tab`: `string`, `{ required: true }`

Slots: `tab`, `manual-lead`, `item`, `staged-extra`, `footer`.
