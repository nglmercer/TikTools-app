
### BaseJsonView.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonView.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `value` | `unknown` | Opcional | `undefined` |
| `source` | `string` | Opcional | `undefined` |
| `label` | `string` | Opcional | `undefined` |
| `lineNumbers` | `boolean` | Opcional | `false` |
| `switchable` | `boolean` | Opcional | `false` |
| `copyable` | `boolean` | Opcional | `true` |
| `maxHeight` | `string &#124; number` | Opcional | `undefined` |
| `maxLines` | `number` | Opcional | `DEFAULT_MAX_LINES` |
| `maxChars` | `number` | Opcional | `DEFAULT_MAX_CHARS` |
| `sortKeys` | `boolean` | Opcional | `false` |
| `treeDepth` | `number` | Opcional | `DEFAULT_TREE_DEPTH` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `copy`: `[text: string]`

Modelos:

- `mode`: `'code' &#124; 'tree'`, `{ default: 'code' }`
- `wrap`: `boolean`, `{ default: true }`

Slots: —.

### BaseJsonDiff.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `before` | `unknown` | Opcional | `undefined` |
| `after` | `unknown` | Opcional | `undefined` |
| `beforeLabel` | `string` | Opcional | `undefined` |
| `afterLabel` | `string` | Opcional | `undefined` |
| `label` | `string` | Opcional | `undefined` |
| `switchable` | `boolean` | Opcional | `false` |
| `context` | `number` | Opcional | `3` |
| `maxHeight` | `string &#124; number` | Opcional | `undefined` |
| `maxLines` | `number` | Opcional | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.
