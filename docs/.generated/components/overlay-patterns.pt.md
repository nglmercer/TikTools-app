
### BasePanel.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/panel/BasePanel.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `title` | `string` | Obrigatória | `—` |
| `subtitle` | `string` | Opcional | `undefined` |
| `flush` | `boolean` | Opcional | `false` |
| `headerLayout` | `'auto' &#124; 'inline' &#124; 'stacked'` | Opcional | `undefined` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: `leading`, `status`, `actions`, `default`, `footer`.

### OverlayLibraryPreview.vue

[Código-fonte](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/widgets/overlay-cloud/ui/OverlayLibraryPreview.vue)

| Propriedade | Tipo | Presença | Padrão |
| --- | --- | --- | --- |
| `kind` | `WidgetKind` | Obrigatória | `—` |
| `design` | `WidgetStyle` | Opcional | `undefined` |
| `label` | `string` | Obrigatória | `—` |

Eventos próprios (os atributos de controles nativos são repassados conforme o componente):

Nenhum evento próprio declarado.

Modelos:

Sem v-model próprio.

Slots: —.
