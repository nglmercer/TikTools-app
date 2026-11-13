
### BaseButton.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseButton.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `variant` | `'default' &#124; 'primary' &#124; 'danger' &#124; 'ghost'` | Opcional | `'default'` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `type` | `'button' &#124; 'submit' &#124; 'reset'` | Opcional | `'button'` |
| `block` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `disabled` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `pressed` | `boolean` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `icon`, `default`.

### BaseIconButton.vue

[Código fuente](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseIconButton.vue)

| Propiedad | Tipo | Presencia | Default |
| --- | --- | --- | --- |
| `label` | `string` | Obligatoria | `—` |
| `hint` | `string` | Opcional | `undefined` |
| `variant` | `'default' &#124; 'ghost' &#124; 'primary' &#124; 'danger'` | Opcional | `'ghost'` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `undefined / Vue boolean false` |
| `pressed` | `boolean` | Opcional | `undefined` |

Eventos propios (los atributos de control nativo se reenvían según el componente):

Sin eventos propios declarados.

Modelos:

Sin v-model propio.

Slots: `default`.
