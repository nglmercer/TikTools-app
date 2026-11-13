
### CollectionToolbar.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/collection-toolbar/CollectionToolbar.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `label` | `string` | Required | `—` |
| `count` | `string` | Optional | `undefined` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `search`, `default`, `actions`.

### BaseCheckbox.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/checkbox/BaseCheckbox.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `disabled` | `boolean` | Optional | `false` |
| `indeterminate` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

- `modelValue`: `boolean`, `{ default: false }`

Slots: `default`.
