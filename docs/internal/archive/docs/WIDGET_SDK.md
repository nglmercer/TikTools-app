# Widget templates

All five widgets use `src/widgets/sdk/WidgetHost.vue` in OBS and in the
desktop preview. Previews render locally, require no gateway or credential,
and keep their final sample visible. Replay and template changes dispose the
previous controller and timers. Live mode retains normal alert durations,
deduplication, gift aggregation and the gateway transport.

## Add a widget

1. Create a Vue renderer inside the shared WidgetStage, with scoped styles. Do not style html/body/#app;
   the standalone entry owns those. Size relative to the parent, not the viewport.
2. Use `defineWidget` from `sdk/template.ts`. Supply an id, a `schema`, a
   `samples()` factory returning fresh domain envelopes, and `create(search, clock)`.
3. Define every supported text field, required editor field, token and visual
   default in the schema. `WidgetHost` resolves `schema.defaultDesign` before
   rendering, so the dashboard preview, builder preview and OBS bundle share
   the same starting appearance.
4. In create, parse settings, allocate local reactive state and create a
   controller. Return that controller and a `render(status, debug)` function.
   The controller must implement handleEnvelope, clear and dispose. Pass the
   supplied clock into timed controllers so previews can hold the sample.
5. Register the definition in `sdk/templates.ts`, add the desktop tab, and
   create an OBS main.ts calling `mountWidget(template)`. Add the standalone
   build entry, gateway route allowlist and packaging entry for a new public URL.

Existing definitions are working examples; event processing remains independent
of Vue. Registering a template does not automatically expose a gateway route.

## Editor integration

Mount WidgetHost with template, mode="preview", search (serialized behavior
settings), design, and replayKey. Increment replayKey to replay. Changes to
template/search/mode rebuild the controller; design changes apply immediately
without restarting playback.

Do not create a second preview renderer in the desktop UI. Pass the same
template and partial design to `WidgetHost`; it merges the partial design with
the template schema. A field is hidden only when the saved design contains it
in `hiddenText` or its text override is an empty string. There is no implicit
editor-only field filter, so a new or reset design looks the same in both
preview surfaces.

`WidgetStyle` is a JSON-serializable style contract. `widgetStyleFields`
provides color/number controls for background, textColor, accent and radius.
Only six/eight-digit hex colors and finite radii (clamped to 0–48px) are applied.
Renderers consume the corresponding `--widget-*` CSS variables. The desktop
style editor uses this schema with an isolated draft, live preview, save, cancel
and reset. Designs are stored through `app.state.set` at `widgets.design.<kind>`.
The host includes validated saved tokens in the copied OBS URL's `design`
fragment parameter; credentials still never cross into the WebView. Standalone
widgets parse that snapshot through `sdk/design.ts`. After saving, users must
copy the updated URL into OBS; already installed URLs do not update live.

Text overrides live in `design.text` (300 characters per field). The template
schema owns field order, required fields and supported tokens; default strings
remain in `sdk/text.ts` so the Vue renderers and editor use one resolver. An
empty string hides a line, an explicit `hiddenText` entry hides the default
line, and an absent key uses the default. `{{name}}` and `{{username}}` resolve
viewer data, gifts also expose `{{gift}}`, `{{count}}`, `{{diamonds}}`, and chat
exposes `{{message}}`. Substitution is single-pass plain text, not HTML or
JavaScript; unknown placeholders remain literal. Preview and OBS use the same
resolver and the same schema defaults.

## Why the previous preview was blank

The desktop embedded gateway HTML with sandbox="allow-scripts". Module requests
from that sandbox have Origin: null; the gateway rejects origins outside its
allowlist before serving assets. Local rendering removes that HTTP/sandbox
dependency, preserves the gateway policy, and uses the same renderer as OBS.
The old demo also expired after a few seconds; preview mode now holds its sample.
