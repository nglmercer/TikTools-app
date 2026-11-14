
### BaseTabs.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/tabs/BaseTabs.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `tabs` | `readonly TabOption<T>[]` | Required | `—` |
| `label` | `string` | Optional | `undefined` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `T`, `{ required: true }`

Slots: `bar`, `default { value: model }`.

### BaseToolbar.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/toolbar/BaseToolbar.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `orientation` | `'horizontal' &#124; 'vertical'` | Optional | `'horizontal'` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `default`.
