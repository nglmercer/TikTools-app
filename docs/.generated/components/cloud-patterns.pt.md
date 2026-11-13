
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

### SyncPanel.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/features/sync-changes/ui/SyncPanel.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `adapter` | `SyncAdapter` | Obrigatória | `—` |
| `preparing` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `open`: `boolean`, `{ default: false }`

Slots: `preparation`.
