
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

### BaseTextArea.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-area/BaseTextArea.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `rows` | `number` | Opcional | `4` |
| `placeholder` | `string` | Opcional | `undefined` |
| `maxLength` | `number` | Opcional | `undefined` |
| `monospace` | `boolean` | Opcional | `false` |
| `disabled` | `boolean` | Opcional | `false` |
| `readonly` | `boolean` | Opcional | `false` |
| `invalid` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseSlider.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/slider/BaseSlider.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `min` | `number` | Opcional | `0` |
| `max` | `number` | Opcional | `100` |
| `step` | `number` | Opcional | `1` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `number`, `{ default: 0 }`

Slots: —.

### BaseRadio.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/radio/BaseRadio.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `value` | `T` | Obrigatória | `—` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `T`, ``

Slots: `default`.

### BaseTagsInput.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tags-input/BaseTagsInput.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `placeholder` | `string` | Opcional | `undefined` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `string[]`, `{ default: () => [] }`

Slots: —.

### BaseDateField.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/date-field/BaseDateField.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `type` | `'date' &#124; 'time' &#124; 'datetime-local'` | Opcional | `'date'` |
| `min` | `string` | Opcional | `undefined` |
| `max` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseColorField.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/color-field/BaseColorField.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `string`, `{ default: '#000000' }`

Slots: —.

### BaseFileUpload.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/file-upload/BaseFileUpload.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `label` | `string` | Obrigatória | `—` |
| `hint` | `string` | Obrigatória | `—` |
| `accept` | `string` | Obrigatória | `—` |
| `maxBytes` | `number` | Obrigatória | `—` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

- `error`: `[message: string]`

Modelos:

- `modelValue`: `File &#124; null`, `{ default: null }`

Slots: —.
