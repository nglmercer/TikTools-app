
### BaseSplitter.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/splitter/BaseSplitter.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Opcional | `'horizontal'` |
| `min` | `number` | Opcional | `80` |
| `max` | `number` | Opcional | `Infinity` |
| `step` | `number` | Opcional | `16` |
| `sized` | `'start' &#124; 'end'` | Opcional | `'start'` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `size`: `number`, `{ default: 240 }`

Slots: `start`, `end`.

### MasterDetailLayout.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/layout/MasterDetailLayout.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `narrow` | `boolean` | Obrigatória | `—` |
| `detailOpen` | `boolean` | Obrigatória | `—` |
| `backLabel` | `string` | Obrigatória | `—` |
| `detailTitle` | `string` | Opcional | `''` |
| `min` | `number` | Opcional | `240` |
| `max` | `number` | Opcional | `900` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `back`: `[]`

Modelos:

- `size`: `number`, `{ default: 300 }`

Slots: `master`, `detail-actions`, `detail`.
