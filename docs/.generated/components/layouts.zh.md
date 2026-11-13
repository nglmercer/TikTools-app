
### BaseSplitter.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/splitter/BaseSplitter.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | 可选 | `'horizontal'` |
| `min` | `number` | 可选 | `80` |
| `max` | `number` | 可选 | `Infinity` |
| `step` | `number` | 可选 | `16` |
| `sized` | `'start' &#124; 'end'` | 可选 | `'start'` |

组件自身的事件（原生控件属性按组件约定透传）:

未声明组件自身的事件。

模型:

- `size`: `number`, `{ default: 240 }`

Slots: `start`, `end`.

### MasterDetailLayout.vue

[源代码](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/layout/MasterDetailLayout.vue)

| 属性 | 类型 | 是否必需 | 默认值 |
| --- | --- | --- | --- |
| `narrow` | `boolean` | 必需 | `—` |
| `detailOpen` | `boolean` | 必需 | `—` |
| `backLabel` | `string` | 必需 | `—` |
| `detailTitle` | `string` | 可选 | `''` |
| `min` | `number` | 可选 | `240` |
| `max` | `number` | 可选 | `900` |

组件自身的事件（原生控件属性按组件约定透传）:

- `back`: `[]`

模型:

- `size`: `number`, `{ default: 300 }`

Slots: `master`, `detail-actions`, `detail`.
