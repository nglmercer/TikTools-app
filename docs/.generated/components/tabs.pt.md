
### BaseTabs.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tabs/BaseTabs.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `tabs` | `readonly TabOption<T>[]` | Obrigatória | `—` |
| `label` | `string` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

- `modelValue`: `T`, `{ required: true }`

Slots: `bar`, `default { value: model }`.

### BaseToolbar.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/toolbar/BaseToolbar.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Opcional | `'horizontal'` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `default`.
