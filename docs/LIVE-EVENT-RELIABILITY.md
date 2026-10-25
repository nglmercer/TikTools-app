# Correcciones de eventos LIVE

Base auditada: `remake`, HEAD `4acf53f` (reversión intencional de `c5e7cf9`).
No se restauran `gift_streak.rs`, `like_aggregation.rs` ni sus eventos derivados.

## Identidades y entrega

- Backend: identidad exacta por creador, sala, método, tipo y `msgId` no cero.
  La memoria sobrevive reconexiones y cambios de sala, con 4096 entradas / TTL
  de cinco minutos. Nunca se usa texto, nickname, precio ni cantidades.
- Frontend: un cursor del journal identifica una entrega ya aceptada por el
  backend. Solo las entregas antiguas sin cursor usan la caché de mensajes.
  El core también conserva el cursor dentro del payload para que sobreviva
  la deserialización tipada de IPC y el adapter de browser.
  Esto evita contradicciones cuando expira o se desaloja una identidad backend.
- `correlationId` del feed coincide con el ID de la automatización original,
  o se genera localmente cuando el paquete no tiene mapping. `points.awarded`
  identifica su origen mediante el ID `points-awarded-<correlationId>`.
- El feed detecta huecos mientras está visible, recupera páginas en orden,
  conserva el límite inferior durante tráfico concurrente y deduplica el
  solapamiento por cursor. La reanudación y la carga inicial también recuperan.
- Desktop deja de interceptar `ui.feed.backfill` con un journal posterior al
  bus: ahora usa el journal autoritativo del core, igual que browser y CLI.
  Recuperar lee presentación; nunca vuelve a ingresar paquetes ni ejecuta TTS,
  plugins, puntos, workflows o alertas.
- El filtro Chat continúa ocultando regalos deliberadamente. All/Gifts los
  muestran incluso sin imagen, catálogo, nombre enriquecido o precio conocido.

## Contabilidad pequeña y acotada

`live_events/totals.rs` mantiene únicamente máximos de combos y contadores.
Se ejecuta después de deduplicar y nunca sustituye los eventos originales
`tiktok.gift` / `tiktok.like`.

Un combo requiere `groupId`, ID de usuario e ID de regalo no cero, dentro de
creador/sala. Sus máximos acumulados producen incrementos, no sumas repetidas.
La finalización conserva una marca durante el TTL; una finalización repetida
con nuevo msgId no vuelve a sumar. Los incrementos se contabilizan de inmediato,
por lo que un paquete final ausente no pierde lo ya observado. La expiración
es perezosa, al observar otro regalo; no necesita tareas ni regalos sintéticos.
Sin identidad de combo, cada paquete conserva su cantidad independiente.
No se interpreta la ausencia de catálogo como prueba de regalo no acumulable.
Un precio desconocido es cero; no se inventan monedas. El máximo monetario
permite completar puntos si el precio aparece en un paquete posterior.

Las automatizaciones reciben el campo opcional y tipado `data.aggregation`:

- Gifts: `increment`, `cumulative`, `diamonds` (incremento monetario).
- Likes: transiciones `{ previous, total }` globales y por usuario; usuario
  desconocido produce `user: null`. Cantidades iguales en paquetes distintos
  se suman. Una meta T cruza cuando `previous < T && total >= T`.

El editor incluye plantillas configurables para metas globales y por usuario
(valor inicial 15). No se crean nuevos tipos de eventos TikTok.

La memoria es limitada: cinco minutos / 4096 combos y 4096 contadores de likes.
Ante saturación se conservan los originales y se informa la falta de agregado;
no se olvida un contador para volver a disparar metas por error.
`untrackedCombos` y `untrackedLikeScopes` muestran esta condición.
La garantía de replay corresponde a la ventana retenida, no a un historial
permanente. Sin msgId válido no puede confirmarse un replay.

## Persistencia y presión

El ingreso publica originales y UI antes de SQLite/puntos. Dos futuros
pertenecen al mismo worker: despacho y persistencia ordenada; ninguno crea
tareas por paquete. La cola de efectos tiene 128 plazas (mismo límite que el
worker de puntos existente), con admisión sin bloqueo y rechazo explícito.
La saturación de almacenamiento no detiene feed ni originales; efectos
rechazados se cuentan, registran y degradan `system.health`.
El contexto original se captura para evitar atribuir analytics o puntos
posteriores a otro creador tras cambiar de sala.

El apagado drena efectos dentro de los plazos existentes. Trabajo cancelado
se cuenta; una operación ya iniciada tiene resultado incierto y no se presenta
como aplicada correctamente. Los eventos originales ya no esperan puntos:
los campos opcionales de puntos del paquete original no se rellenan después;
los consumidores deben observar `points.changed` / `points.awarded`.

## Diagnóstico y SDK

El Event Debugger del feed consulta `system.health` bajo demanda y muestra
contadores de recepción/decodificación en ingreso, admisión, duplicados,
procesamiento, publicación, descarte, colas, entrega desktop, presentación,
coalescencia, retención y recuperación. Incluye las últimas 12 identidades,
cursores y correlaciones, sin cuerpos de chat, cookies ni credenciales.
CLI equivalente: `tiktools rpc system.health '{}'` (o JSON-RPC existente).

La revisión fijada del SDK `4f9d0b63799c3e23cf20be717779ece507b9eed4`
normaliza Chat, EmoteChat, Gift, Like, Member, Social y RoomUser; todos tienen
mapping en TikTools (RoomUser se proyecta como snapshot `room.stats`). Los
métodos restantes o payloads individuales inválidos llegan como Unknown.
Se contabilizan por método saneado en un mapa acotado; no se inventa un
contrato TikTok para ellos. Un fallo del sobre completo continúa por el
canal de error existente del transporte. `received` incluye lifecycle;
`decoded` cuenta paquetes canónicos que alcanzaron ingreso, no frames WS.

## OBS

Los widgets conservan el mismo TTL de identidad que el core, sin supresión
permanente de IDs. Los combos visuales incluyen creador, sala, remitente y
regalo en su clave; `groupId` vacío/cero nunca agrupa. El agregado tipado del
backend permite reconocer combos sin catálogo, y una finalización sin
incremento no abre de nuevo una alerta ya terminada. Se limita también el
número de combos visuales activos; no se inventa precio para regalos sin él.

## Validación

| Validación | Resultado |
| --- | --- |
| `bun run contracts:check` | OK; los contratos generados coinciden |
| `bun run check:web` | OK: lint, tipos, 291 tests Bun, 477 tests Vitest y build |
| `bun run check:rust` | OK, workspace locked |
| `bun run lint:rust` | OK, todos los targets/features, warnings como errores |
| `bun run test:rust` | OK: 803 tests pasados, 0 fallos, 1 ignorado existente |
| Tests Rust LIVE específicos tras los últimos ajustes | 34 pasados; además 1 test de cursor a través de IPC tipado |
| `bun run test:e2e` | 55 pasados, 5 fallos existentes, reproducidos en `4acf53f` |
| `bun run test:widgets-e2e` | OK: 15 tests visuales/funcionales y 13 con gateway real |

Antes de widgets se ejecutó `bun run build:widgets:test`; los bundles de
producción no exponen el hook de pruebas. También pasó `cargo fmt --all`.
La primera ejecución conjunta de las suites Playwright colisionó en su
`test-results`; se repitieron secuencialmente y los resultados de la tabla
corresponden a ejecuciones sin esa colisión.

Los cinco fallos E2E se verificaron en un worktree limpio del HEAD original:

- `connections.spec.ts:65`: falta el botón accesible `Test connection`.
- `screenshots.spec.ts:128`: el mismo botón ausente impide completar el test.
- `screenshots.spec.ts:53`, `:143` y `:194`: referencias visuales anteriores
  al texto/botón de monitor presente en el HEAD original (144/297 píxeles).

No se cambian esos tests ni sus referencias para ocultar los fallos. Las
pruebas nuevas de LIVE y debugger pasan. La validación inicial no usó una cuenta TikTok real. La corrección posterior
del decoder sí se comprobó contra el mismo LIVE público del reporte.

Pendientes/límites: corregir esos controles/fixtures de Connections en un
cambio separado; replay fuera del TTL o sin identidad no puede confirmarse;
la recuperación está limitada a las 512 entregas del journal en memoria;
los efectos rechazados por saturación se diagnostican, pero no se recuperan
reaplicando el feed. Los agregados no inventan cantidades si su capacidad
está agotada.
Las pruebas existentes del apagado ahora usan cuatro msgIds distintos y
verifican entrega de originales y contabilidad independiente de efectos.
Las dos referencias visuales del feed se actualizan únicamente por la nueva
fila Event Debugger, tras inspección de las imágenes generadas.

## Regalos reales todavía invisibles: causa y corrección posterior

El proceso activo mostraba 64 `WebcastGiftMessage` como Unknown y solo 3
regalos publicados, sin pérdidas, duplicados ni saturación. Una segunda
conexión pública permitió comprobar un paquete real de 6441 bytes:
Prost lo decodificaba correctamente (`typed_error=None`), pero la vista
Full de diagnóstico marcaba `schema.truncated=true` y el SDK convertía el
regalo válido en Unknown antes del ingreso.

El transporte ahora pide exclusivamente `DecodeMode::Normalized`. Se incluye
una copia fijada de `ttl-live-events` con una corrección mínima documentada
en `vendor/ttl-live-events/README.tiktools.md`: el preflight de normalización
usa la profundidad de Prost, independiente de la vista diagnóstica de ocho
niveles. Los límites de bytes, mensajes y campos no aumentan. La memoria de
catálogo se actualiza en orden entre paquetes del mismo batch.

Se obtuvo una estructura de regresión de otro regalo válido, reemplazando
todos los valores numéricos/texto y eliminando bytes opacos y campos
desconocidos. El fixture reproduce el fallo del decoder anterior y valida
el nuevo camino hasta `live.ui-event` y `tiktok.gift`, incluyendo msgId grande,
metadatos ausentes y aislamiento de paquetes inválidos. No contiene datos
originales de la sesión. La sonda pública corregida confirmó
`canonical gift received` desde el mismo LIVE.

Validación posterior: 803 tests Rust del workspace, 17 tests del SDK
vendorizado, 17 tests de la frontera TikTok, contratos, check:web, check:rust
y clippy pasan. La prueba SDK aislada requirió reubicar su fixture upstream
(con sus mismos bytes y aserciones) al directorio del crate vendorizado.
El desktop debug fue recompilado con la corrección.

El binario desktop está recompilado; un proceso que ya estaba abierto continúa
usando su decoder anterior hasta que se reinicie TikTools.

## Archivos modificados

- `Cargo.lock`
- `Cargo.toml`
- `crates/tiktools-core/src/automation_runtime/samples.rs`
- `crates/tiktools-core/src/contracts/mod.rs`
- `crates/tiktools-core/src/control/system.rs`
- `crates/tiktools-core/src/db/analytics/mod.rs`
- `crates/tiktools-core/src/events.rs`
- `crates/tiktools-core/src/lib.rs`
- `crates/tiktools-core/src/live_events/dedup.rs`
- `crates/tiktools-core/src/live_events/handlers.rs`
- `crates/tiktools-core/src/live_events/ingress.rs`
- `crates/tiktools-core/src/live_events/mapping.rs`
- `crates/tiktools-core/src/live_events/mod.rs`
- `crates/tiktools-core/src/live_events/pump.rs`
- `crates/tiktools-core/src/live_events/session.rs`
- `crates/tiktools-core/src/live_events/totals.rs`
- `crates/tiktools-core/src/services/points.rs`
- `crates/tiktools-desktop/src/window/ipc.rs`
- `crates/tiktools-desktop/src/window/projection.rs`
- `crates/tiktools-tiktok/Cargo.toml`
- `crates/tiktools-tiktok/examples/live_decode_probe.rs`
- `crates/tiktools-tiktok/src/events.rs`
- `crates/tiktools-tiktok/src/lib.rs`
- `crates/tiktools-tiktok/tests/fixtures/README.md`
- `crates/tiktools-tiktok/tests/fixtures/deep-gift.batch.pb`
- `crates/tiktools-tiktok/tests/fixtures/deep-gift.pb`
- `docs/LIVE-EVENT-RELIABILITY.md`
- `src/automation/behavior/live-goal-templates.test.ts`
- `src/automation/behavior/live-goal-templates.ts`
- `src/automation/contracts/generated/automation-events.schema.json`
- `src/automation/contracts/generated/automation-events.ts`
- `src/automation/contracts/generated/event-registry.generated.ts`
- `src/automation/contracts/generated/index.ts`
- `src/shared/live-identity.test.ts`
- `src/shared/live-identity.ts`
- `src/shared/messages.ts`
- `src/web/entities/live-event/model/live-feed-state.test.ts`
- `src/web/entities/live-event/model/live-feed-state.ts`
- `src/web/features/edit-automation-event/ui/EventEditor.vue`
- `src/web/widgets/live-feed/ui/EventDebugger.vue`
- `src/web/widgets/live-feed/ui/LiveFeedPanel.vue`
- `src/widgets/shared/alert-queue.test.ts`
- `src/widgets/shared/alert-queue.ts`
- `src/widgets/shared/gift-controller.test.ts`
- `src/widgets/shared/gift-controller.ts`
- `tests/e2e/fixtures/tiktools-host.ts`
- `tests/e2e/live-gifts-emotes-monitor.spec.ts`
- `tests/e2e/screenshots.spec.ts-snapshots/feed-connected-chromium-linux.png`
- `tests/e2e/screenshots.spec.ts-snapshots/feed-empty-chromium-linux.png`
- `vendor/ttl-live-events/.gitignore`
- `vendor/ttl-live-events/Cargo.toml`
- `vendor/ttl-live-events/README.tiktools.md`
- `vendor/ttl-live-events/fixtures/chat.pb`
- `vendor/ttl-live-events/src/dynamic.rs`
- `vendor/ttl-live-events/src/event.rs`
- `vendor/ttl-live-events/src/lib.rs`
- `vendor/ttl-live-events/src/normalize.rs`
- `vendor/ttl-live-events/src/social.rs`
- `vendor/ttl-live-events/src/user.rs`
