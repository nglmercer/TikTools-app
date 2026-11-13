
### BaseJsonView.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonView.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `value` | `unknown` | 可选 | `undefined` |
| `source` | `string` | 可选 | `undefined` |
| `label` | `string` | 可选 | `undefined` |
| `lineNumbers` | `boolean` | 可选 | `false` |
| `switchable` | `boolean` | 可选 | `false` |
| `copyable` | `boolean` | 可选 | `true` |
| `maxHeight` | `string &#124; number` | 可选 | `undefined` |
| `maxLines` | `number` | 可选 | `DEFAULT_MAX_LINES` |
| `maxChars` | `number` | 可选 | `DEFAULT_MAX_CHARS` |
| `sortKeys` | `boolean` | 可选 | `false` |
| `treeDepth` | `number` | 可选 | `DEFAULT_TREE_DEPTH` |

组件自身的事件（原生控件属性按组件约定透传）:

- `copy`: `[text: string]`

模型:

- `mode`: `'code' &#124; 'tree'`, `{ default: 'code' }`
- `wrap`: `boolean`, `{ default: true }`

Slots: —.

### BaseJsonDiff.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `before` | `unknown` | 可选 | `undefined` |
| `after` | `unknown` | 可选 | `undefined` |
| `beforeLabel` | `string` | 可选 | `undefined` |
| `afterLabel` | `string` | 可选 | `undefined` |
| `label` | `string` | 可选 | `undefined` |
| `switchable` | `boolean` | 可选 | `false` |
| `context` | `number` | 可选 | `3` |
| `maxHeight` | `string &#124; number` | 可选 | `undefined` |
| `maxLines` | `number` | 可选 | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.
