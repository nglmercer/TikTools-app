
### BaseButton.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseButton.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `variant` | `'default' &#124; 'primary' &#124; 'danger' &#124; 'ghost'` | Opcional | `'default'` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `type` | `'button' &#124; 'submit' &#124; 'reset'` | Opcional | `'button'` |
| `block` | `boolean` | Opcional | `false` |
| `disabled` | `boolean` | Opcional | `false` |
| `pressed` | `boolean` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `icon`, `default`.

### BaseIconButton.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseIconButton.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `label` | `string` | Obrigatória | `—` |
| `hint` | `string` | Opcional | `undefined` |
| `variant` | `'default' &#124; 'ghost' &#124; 'primary' &#124; 'danger'` | Opcional | `'ghost'` |
| `size` | `ControlSize` | Opcional | `'md'` |
| `disabled` | `boolean` | Opcional | `false` |
| `pressed` | `boolean` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `default`.
