
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

### BaseSpinBox.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `min` | `number` | Opcional | `undefined` |
| `max` | `number` | Opcional | `undefined` |
| `step` | `number` | Opcional | `1` |
| `precision` | `number` | Opcional | `undefined` |
| `suffix` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `orientation` | `'vertical' &#124; 'horizontal'` | Opcional | `'vertical'` |
| `disabled` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `readonly` | `boolean` | Opcional | `undefined / Vue boolean false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.
