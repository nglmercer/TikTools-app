# Widget templates

All five widgets use `src/widgets/sdk/WidgetHost.vue` in OBS and in the
desktop preview. Previews render locally, require no gateway or credential,
and keep their final sample visible. Replay and template changes dispose the
previous controller and timers. Live mode retains normal alert durations,
deduplication, gift aggregation and the gateway transport.

## Add a widget

1. Create a Vue renderer inside the shared WidgetStage, with scoped styles. Do not style html/body/#app;
   the standalone entry owns those. Size relative to the parent, not the viewport.
2. Use `defineWidget` from `sdk/template.ts`. Supply an id, a `samples()`
   factory returning fresh domain envelopes, and `create(search, clock)`.
3. In create, parse settings, allocate local reactive state and create a
   controller. Return that controller and a `render(status, debug)` function.
   The controller must implement handleEnvelope, clear and dispose. Pass the
   supplied clock into timed controllers so previews can hold the sample.
4. Register the definition in `sdk/templates.ts`, add the desktop tab, and
   create an OBS main.ts calling `mountWidget(template)`. Add the standalone
   build entry, gateway route allowlist and packaging entry for a new public URL.

Existing definitions are working examples; event processing remains independent
of Vue. Registering a template does not automatically expose a gateway route.

## Editor integration

Mount WidgetHost with template, mode="preview", search (serialized behavior
settings), design, and replayKey. Increment replayKey to replay. Changes to
template/search/mode rebuild the controller; design changes apply immediately
without restarting playback.

`WidgetStyle` is a JSON-serializable style contract. `widgetStyleFields`
provides color/number controls for background, textColor, accent and radius.
Only six/eight-digit hex colors and finite radii (clamped to 0–48px) are applied.
Renderers consume the corresponding `--widget-*` CSS variables. A future
editor can persist this object alongside a template id and behavior settings.
Persistence, editor UI and delivery of saved designs to OBS are not implemented.

## Why the previous preview was blank

The desktop embedded gateway HTML with sandbox="allow-scripts". Module requests
from that sandbox have Origin: null; the gateway rejects origins outside its
allowlist before serving assets. Local rendering removes that HTTP/sandbox
dependency, preserves the gateway policy, and uses the same renderer as OBS.
The old demo also expired after a few seconds; preview mode now holds its sample.
