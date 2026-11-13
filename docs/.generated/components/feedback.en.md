
### BaseAlert.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/alert/BaseAlert.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `tone` | `'error' &#124; 'warning' &#124; 'info' &#124; 'ok'` | Optional | `'error'` |
| `title` | `string` | Required | `—` |
| `compact` | `boolean` | Optional | `false` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `default`.

### BaseEmptyState.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/empty-state/BaseEmptyState.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `title` | `string` | Required | `—` |
| `description` | `string` | Optional | `undefined` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: `icon`, `default`.

### BaseStatus.vue

[Source](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/ui/status/BaseStatus.vue)

| Property | Type | Presence | Default |
| --- | --- | --- | --- |
| `label` | `string` | Required | `—` |
| `tone` | `StatusTone` | Optional | `'neutral'` |

Own events (native control attributes are forwarded according to the component):

No own declared events.

Models:

No own v-model.

Slots: —.
