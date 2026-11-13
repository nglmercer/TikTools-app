
### BaseImportDialog.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/import-dialog/BaseImportDialog.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `title` | `string` | Required | `—` |
| `width` | `number` | Optional | `720` |
| `tabs` | `readonly ImportTab[]` | Required | `—` |
| `hideTabs` | `boolean` | Optional | `false` |
| `manualTab` | `string` | Optional | `MANUAL_IMPORT_TAB` |
| `accept` | `string` | Optional | `'.json,application/json'` |
| `maxBytes` | `number` | Optional | `undefined` |
| `staged` | `readonly StagedImportItem[]` | Optional | `() => []` |
| `stagedSource` | `string` | Optional | `''` |
| `busy` | `boolean` | Optional | `false` |
| `errors` | `readonly string[]` | Optional | `() => []` |
| `success` | `string` | Optional | `''` |
| `importLabel` | `string` | Optional | `''` |
| `labels` | `ImportDialogLabels` | Optional | `() => ({})` |

Own events (native control attributes are forwarded according to the component):

- `content`: `[text: string, source: string]`
- `error`: `[message: string]`
- `clear`: `[]`
- `import`: `[]`

Models:

- `open`: `boolean`, `{ default: false }`
- `tab`: `string`, `{ required: true }`

Slots: `tab`, `manual-lead`, `item { item: item }`, `staged-extra`, `footer { tab: tab }`.
