
### BaseImportDialog.vue

[ソースコード](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/import-dialog/BaseImportDialog.vue)

| プロパティ | 型 | 必須・任意 | 既定値 |
| --- | --- | --- | --- |
| `title` | `string` | 必須 | `—` |
| `width` | `number` | 任意 | `720` |
| `tabs` | `readonly ImportTab[]` | 必須 | `—` |
| `hideTabs` | `boolean` | 任意 | `false` |
| `manualTab` | `string` | 任意 | `MANUAL_IMPORT_TAB` |
| `accept` | `string` | 任意 | `'.json,application/json'` |
| `maxBytes` | `number` | 任意 | `undefined` |
| `staged` | `readonly StagedImportItem[]` | 任意 | `() => []` |
| `stagedSource` | `string` | 任意 | `''` |
| `busy` | `boolean` | 任意 | `false` |
| `errors` | `readonly string[]` | 任意 | `() => []` |
| `success` | `string` | 任意 | `''` |
| `importLabel` | `string` | 任意 | `''` |
| `labels` | `ImportDialogLabels` | 任意 | `() => ({})` |

独自のイベント（ネイティブコントロールの属性はコンポーネントの仕様に従って転送されます）:

- `content`: `[text: string, source: string]`
- `error`: `[message: string]`
- `clear`: `[]`
- `import`: `[]`

モデル:

- `open`: `boolean`, `{ default: false }`
- `tab`: `string`, `{ required: true }`

Slots: `tab`, `manual-lead`, `item { item: item }`, `staged-extra`, `footer { tab: tab }`.
