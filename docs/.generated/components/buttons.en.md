
### BaseButton.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseButton.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `variant` | `'default' &#124; 'primary' &#124; 'danger' &#124; 'ghost'` | Optional | `'default'` |
| `size` | `ControlSize` | Optional | `'md'` |
| `type` | `'button' &#124; 'submit' &#124; 'reset'` | Optional | `'button'` |
| `block` | `boolean` | Optional | `undefined / Vue boolean false` |
| `disabled` | `boolean` | Optional | `undefined / Vue boolean false` |
| `pressed` | `boolean` | Optional | `undefined` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `icon`, `default`.

### BaseIconButton.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/button/BaseIconButton.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `label` | `string` | Required | `—` |
| `hint` | `string` | Optional | `undefined` |
| `variant` | `'default' &#124; 'ghost' &#124; 'primary' &#124; 'danger'` | Optional | `'ghost'` |
| `size` | `ControlSize` | Optional | `'md'` |
| `disabled` | `boolean` | Optional | `undefined / Vue boolean false` |
| `pressed` | `boolean` | Optional | `undefined` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `default`.
