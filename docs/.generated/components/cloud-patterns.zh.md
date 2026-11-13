
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

### SyncPanel.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/features/sync-changes/ui/SyncPanel.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `adapter` | `SyncAdapter` | 必需 | `—` |
| `preparing` | `boolean` | 可选 | `false` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `open`: `boolean`, `{ default: false }`

Slots: `preparation`.
