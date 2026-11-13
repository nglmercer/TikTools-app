
### BaseSplitter.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/splitter/BaseSplitter.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Opcional | `'horizontal'` |
| `min` | `number` | Opcional | `80` |
| `max` | `number` | Opcional | `Infinity` |
| `step` | `number` | Opcional | `16` |
| `sized` | `'start' &#124; 'end'` | Opcional | `'start'` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `size`: `number`, `{ default: 240 }`

Slots: `start`, `end`.

### MasterDetailLayout.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/layout/MasterDetailLayout.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `narrow` | `boolean` | Obligatoria | `—` |
| `detailOpen` | `boolean` | Obligatoria | `—` |
| `backLabel` | `string` | Obligatoria | `—` |
| `detailTitle` | `string` | Opcional | `''` |
| `min` | `number` | Opcional | `240` |
| `max` | `number` | Opcional | `900` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `back`: `[]`

Modelos:

- `size`: `number`, `{ default: 300 }`

Slots: `master`, `detail-actions`, `detail`.
