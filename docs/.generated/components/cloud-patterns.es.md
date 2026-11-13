
### BaseJsonDiff.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/json/BaseJsonDiff.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `before` | `unknown` | Opcional | `undefined` |
| `after` | `unknown` | Opcional | `undefined` |
| `beforeLabel` | `string` | Opcional | `undefined` |
| `afterLabel` | `string` | Opcional | `undefined` |
| `label` | `string` | Opcional | `undefined` |
| `switchable` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `context` | `number` | Opcional | `3` |
| `maxHeight` | `string &#124; number` | Opcional | `undefined` |
| `maxLines` | `number` | Opcional | `DEFAULT_MAX_LINES` |
| `sortKeys` | `boolean` | Opcional | `undefined / Vue boolean false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `layout`: `'unified' &#124; 'side-by-side'`, `{ default: 'unified' }`

Slots: —.

### BasePanel.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `title` | `string` | Obligatoria | `—` |
| `subtitle` | `string` | Opcional | `undefined / Vue boolean false` |
| `flush` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Opcional | `undefined / Vue boolean false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `leading`, `status`, `actions`, `default`, `footer`.
