
### BaseJsonView.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonView.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
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

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `copy`: `[text: string]`

Modelos:

- `mode`: `'code' &#124; 'tree'`, `{ default: 'code' }`
- `wrap`: `boolean`, `{ default: true }`

Slots: —.

### BaseJsonDiff.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
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

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.
