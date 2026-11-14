---
title: "Normalización de eventos, lotes y huecos"
docId: "developer.events"
translationKey: "developer.events"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "caded5529a62e826a91d2deb86936c29c709ac713c2a0cd021f467b1dfa2b6e7"
translationRevision: "381296887776b3746ddfc5e6a2d66dc6ec0c9bcd48fead1acda9343e491b86d1"
order: 100
---

# Normalización de eventos, lotes y huecos {#event-normalization-batches-and-gaps}

## Objetivo y requisitos {#goal-and-prerequisites}

Consume eventos LIVE sin contabilizarlos dos veces ni confundir recuperación visual con ejecución. Conoce la [persistencia](storage.md) y utiliza muestras saneadas en lugar de capturas de cuentas reales. `tiktools-tiktok` envuelve las dependencias fijadas de firma y decodificación y exporta valores estables; el núcleo no publica objetos protobuf como API de aplicación.

## Seguir el recorrido implementado {#follow-the-implemented-pipeline}

La entrada captura el contexto original de creador/sala antes del trabajo asíncrono. La normalización produce chat, regalos, likes, eventos sociales y de ciclo de vida. El núcleo publica originales y gestiona por separado contabilidad y efectos. Los procesadores pueden adjuntar evidencia acotada en `event.intel`; ante fallos conservan el evento y registran degradación. Los contratos generados describen únicamente campos emitidos.

La identidad de repetición incluye creador, sala, método, tipo e identificador de mensaje distinto de cero. Sin identificador no se demuestra repetición. Los combos usan identidad de remitente/regalo/grupo y máximos acumulados, por lo que el paquete final no vuelve a premiar la cantidad. Los precios desconocidos permanecen en cero. Las transiciones agregadas permiten detectar `previous < threshold <= total` sin reemplazar los originales públicos.

## Implementar un consumidor {#implement-a-consumer}

1. Usa los tipos de `src/automation/contracts/generated/` o el analizador de sobres del widget, según el límite.
2. Conserva cursores al serializar y elimina solapamientos entre tiempo real y recuperación.
3. Procesa lotes ordenados sin renderizar reactivamente una vez por evento.
4. Muestra señales de hueco y sobrecarga. Recupera la presentación con páginas acotadas o una instantánea, nunca ejecutando puntos o acciones otra vez.

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts test bun automation
```

Los [contratos Rust](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-core/src/contracts) generan registro, esquemas y muestras; evita copiar manualmente tablas completas de campos.

## Resultado esperado y estado {#expected-result-and-status}

El solapamiento de reconexión solo se deduplica mientras el evento identificado permanezca en el almacén finito actual. Colas, contadores, historial y cachés permanecen acotados. No es un archivo ilimitado ni una garantía universal de ejecución única. Agotar capacidad o perder persistencia afecta explícitamente a la salud.

## Recuperación y verificación {#recover-and-verify}

Utiliza muestras saneadas de regalos/emotes y prueba finales de combo, identificadores ausentes, cambios de sala y huecos. Consulta la revisión del firmador en `Cargo.toml`, no un hash antiguo. Los métodos desconocidos siguen sin semántica inventada hasta ampliar normalización y contratos.

## Detalles de operación {#operational-details}

Los defaults actuales (2026-11-13) son límites de implementación, no un SLA. La entrada LIVE admite 2.048 elementos y el canal de efectos 128; no son presupuestos de bytes. El ciclo de vida espera capacidad. Regalos/follows/shares esperan como máximo un segundo antes de descartarse con hueco. Chat/likes/joins/otros datos se descartan inmediatamente al saturarse. Revisa `droppedCritical`, lotes/unidades de likes perdidos y pérdidas de efectos: una entrada descartada puede no haber ejecutado sus efectos. Backfill solo repara presentación, nunca puntos ni acciones.

La deduplicación guarda como máximo 4.096 identidades durante 300 segundos en memoria del proceso. Sobrevive a reconexiones solo mientras vive ese almacén. Caducidad, expulsión, IDs ausentes o reinicio eliminan la protección; el solapamiento solo está protegido dentro de esos límites.

`ui.feed.backfill` acepta `{version:1,after:0,limit:64}`, cursor numérico global y límite 1–512. Revisa `oldestCursor`, `latestCursor`, `gap` y `truncated`; continúa desde el último cursor recibido. `ui.feed.creator` usa `{creatorId,streamId,after,limit}`, cursores de texto, stream por creador, límite 1–64 y presupuesto de bytes. No intercambies cursores. Si cambia/se expulsa el stream, el cursor es futuro, hay hueco de ingreso o `CreatorFeedUnavailable`, refresca la presentación autoritativa y marca la historia faltante; no inventes efectos.

TikTok inicializa sesión, sala, catálogo de regalos, bundle de firma y socket. Configuración: `TIKTOOLS_TIKTOK_SIGNING_BUNDLE`, `TIKTOOLS_TIKTOK_BUNDLE_CACHE`, `TIKTOOLS_TIKTOK_BUNDLE_URL`. Defaults: cinco intentos, backoff inicial 2 s hasta 30 s. HTTP 429 usa Retry-After o mínimo creciente 30–300 s con jitter, hasta dos reintentos por rate limit; conserva cooldown tras cancelar/agotarse. Otros reintentos exigen fallo transitorio de transporte. Revisa `Reconnecting { attempt, delay_ms }` y la etapa; no adjuntes cookies ni material privado de firma.

```sh
tiktools api call system.health '{}' --json
```
