
### BaseImportDialog.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/import-dialog/BaseImportDialog.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `title` | `string` | 必需 | `—` |
| `width` | `number` | 可选 | `720` |
| `tabs` | `readonly ImportTab[]` | 必需 | `—` |
| `hideTabs` | `boolean` | 可选 | `false` |
| `manualTab` | `string` | 可选 | `MANUAL_IMPORT_TAB` |
| `accept` | `string` | 可选 | `'.json,application/json'` |
| `maxBytes` | `number` | 可选 | `undefined` |
| `staged` | `readonly StagedImportItem[]` | 可选 | `() => []` |
| `stagedSource` | `string` | 可选 | `''` |
| `busy` | `boolean` | 可选 | `false` |
| `errors` | `readonly string[]` | 可选 | `() => []` |
| `success` | `string` | 可选 | `''` |
| `importLabel` | `string` | 可选 | `''` |
| `labels` | `ImportDialogLabels` | 可选 | `() => ({})` |

组件自身的事件（原生控件属性按组件约定透传）:

- `content`: `[text: string, source: string]`
- `error`: `[message: string]`
- `clear`: `[]`
- `import`: `[]`

模型:

- `open`: `boolean`, `{ default: false }`
- `tab`: `string`, `{ required: true }`

Slots: `tab`, `manual-lead`, `item { item: item }`, `staged-extra`, `footer { tab: tab }`.
