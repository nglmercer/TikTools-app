
### BaseTextField.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-field/BaseTextField.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `type` | `'text' &#124; 'password' &#124; 'search' &#124; 'email' &#124; 'url'` | Opcional | `'text'` |
| `placeholder` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |
| `readonly` | `boolean` | Opcional | `false` |
| `invalid` | `boolean` | Opcional | `false` |
| `revealable` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: `prefix`, `suffix`.

### BaseSpinBox.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/spin-box/BaseSpinBox.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
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

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `number`, `{ default: 0 }`

Slots: `prefix`.

### BaseTextArea.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/text-area/BaseTextArea.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `rows` | `number` | Opcional | `4` |
| `placeholder` | `string` | Opcional | `undefined` |
| `maxLength` | `number` | Opcional | `undefined` |
| `monospace` | `boolean` | Opcional | `false` |
| `disabled` | `boolean` | Opcional | `false` |
| `readonly` | `boolean` | Opcional | `false` |
| `invalid` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseSlider.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/slider/BaseSlider.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `min` | `number` | Opcional | `0` |
| `max` | `number` | Opcional | `100` |
| `step` | `number` | Opcional | `1` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `number`, `{ default: 0 }`

Slots: —.

### BaseRadio.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/radio/BaseRadio.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `value` | `T` | Obligatoria | `—` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `T`, ``

Slots: `default`.

### BaseTagsInput.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tags-input/BaseTagsInput.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `placeholder` | `string` | Opcional | `undefined` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `string[]`, `{ default: () => [] }`

Slots: —.

### BaseDateField.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/date-field/BaseDateField.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `type` | `'date' &#124; 'time' &#124; 'datetime-local'` | Opcional | `'date'` |
| `min` | `string` | Opcional | `undefined` |
| `max` | `string` | Opcional | `undefined` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `string`, `{ default: '' }`

Slots: —.

### BaseColorField.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/color-field/BaseColorField.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

- `modelValue`: `string`, `{ default: '#000000' }`

Slots: —.

### BaseFileUpload.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/file-upload/BaseFileUpload.vue)

| Propiedad | Tipo | Presencia | Predeterminado |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoria | `—` |
| `hint` | `string` | Obligatoria | `—` |
| `accept` | `string` | Obligatoria | `—` |
| `maxBytes` | `number` | Obligatoria | `—` |
| `disabled` | `boolean` | Opcional | `false` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

- `error`: `[message: string]`

Modelos:

- `modelValue`: `File &#124; null`, `{ default: null }`

Slots: —.
