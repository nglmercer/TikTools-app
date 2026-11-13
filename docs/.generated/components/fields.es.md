
### BaseTextField.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | Opcional | `'text'` |
| `placeholder` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `readonly` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `invalid` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `revealable` | `boolean` | Opcional | `undefined / Vue boolean false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

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
