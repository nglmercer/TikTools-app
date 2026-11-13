
### BaseTextField.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | Opcional | `'text'` |
| `placeholder` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |
| `readonly` | `boolean` | Opcional | `false` |
| `invalid` | `boolean` | Opcional | `false` |
| `revealable` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

### BaseSpinBox.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `min` | `number` | Opcional | `undefined` |
| `max` | `number` | Opcional | `undefined` |
| `step` | `number` | Opcional | `1` |
| `precision` | `number` | Opcional | `undefined` |
| `suffix` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `orientation` | `'vertical' &#124; 'horizontal'` | Opcional | `'vertical'` |
| `disabled` | `boolean` | Opcional | `false` |
| `readonly` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.
