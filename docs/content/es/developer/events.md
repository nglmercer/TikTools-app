---
title: "Normalización de eventos, lotes y huecos"
docId: "developer.events"
translationKey: "developer.events"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a9f927c44b7508f65cf0d9145a18ddaf72778e626237696ad998f92cd13789db"
translationRevision: "a9f927c44b7508f65cf0d9145a18ddaf72778e626237696ad998f92cd13789db"
order: 100
---

# Normalización de eventos, lotes y huecos

## Objetivo y requisitos

Consume eventos LIVE sin contabilizarlos dos veces ni confundir recuperación visual con ejecución. Conoce la [persistencia](storage.md) y utiliza muestras saneadas en lugar de capturas de cuentas reales. `tiktools-tiktok` envuelve las dependencias fijadas de firma y decodificación y exporta valores estables; el núcleo no publica objetos protobuf como API de aplicación.

## Seguir el recorrido implementado

La entrada captura el contexto original de creador/sala antes del trabajo asíncrono. La normalización produce chat, regalos, likes, eventos sociales y de ciclo de vida. El núcleo publica originales y gestiona por separado contabilidad y efectos. Los procesadores pueden adjuntar evidencia acotada en `event.intel`; ante fallos conservan el evento y registran degradación. Los contratos generados describen únicamente campos emitidos.

La identidad de repetición incluye creador, sala, método, tipo e identificador de mensaje distinto de cero. Sin identificador no se demuestra repetición. Los combos usan identidad de remitente/regalo/grupo y máximos acumulados, por lo que el paquete final no vuelve a premiar la cantidad. Los precios desconocidos permanecen en cero. Las transiciones agregadas permiten detectar `previous < threshold <= total` sin reemplazar los originales públicos.

## Implementar un consumidor

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

## Resultado esperado y estado

El solapamiento de reconexión no duplica entregas visuales ni la contabilidad protegida por identidad. Colas, contadores, historial y cachés permanecen acotados. No es un archivo ilimitado ni una garantía universal de ejecución única. Agotar capacidad o perder persistencia afecta explícitamente a la salud.

## Recuperación y verificación

Utiliza muestras saneadas de regalos/emotes y prueba finales de combo, identificadores ausentes, cambios de sala y huecos. Consulta la revisión del firmador en `Cargo.toml`, no un hash antiguo. Los métodos desconocidos siguen sin semántica inventada hasta ampliar normalización y contratos.
