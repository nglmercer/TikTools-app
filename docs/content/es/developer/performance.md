---
title: "Rendimiento y trabajo acotado"
docId: "developer.performance"
translationKey: "developer.performance"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "04bc0a89b86724872adae3ab8ddc8e29f5517dad14d51bc103d917582cf289ae"
translationRevision: "21aeade6188e21e4f81184f14fff6e80f9ae9c189ce3003f24d1e57e236bcf96"
order: 110
---

# Rendimiento y trabajo acotado {#performance-and-bounded-work}

## Objetivo y requisitos {#goal-and-prerequisites}

Mide un cuello de botella concreto conservando contabilidad y cierre correcto. Parte de una compilación fija, entrada determinista y perfil aislado. Lee [eventos](events.md) para no confundir un feed más rápido con persistencia más fiable. Los números históricos pertenecen a su compilación y equipo; no son garantías actuales.

## Medir el límite adecuado {#measure-the-right-boundary}

La bandeja del escritorio separa presentación descartable y respuestas fiables. Las instantáneas pueden combinarse; el desbordamiento fiable debe fallar visiblemente. Entrada LIVE, plugins, procesadores y automatizaciones tienen límites propios. Aumentar una cola puede trasladar presión de memoria o latencia a la siguiente etapa.

El runtime frontend expone diagnósticos agregados en desarrollo o al compilar con `VITE_TIKTOOLS_DIAGNOSTICS=1`. Incluyen filas renderizadas, percentiles de intervalos de cuadros, llamadas pendientes, imágenes y temporizadores propios. Una WebView puede no proporcionar memoria del montón JavaScript. La visibilidad de página y ventana determina el interés de presentación, sin detener la contabilidad de negocio.

## Ejecutar una comparación dirigida {#run-a-focused-comparison}

1. Registra revisión, plataforma, mezcla de eventos, imágenes activas, pestaña y duración.
2. Establece una referencia con la misma muestra y periodo de medición.
3. Cambia un límite y compara caudal, latencia p95/p99, descartes, huecos, memoria retenida y drenaje al cerrar.
4. Incluye un plugin o consumidor lento y una cola saturada; una medición en reposo no prueba contrapresión.

```sh
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts rust test -p tiktools-core --features persistence,native-tiktok --test live_throughput --locked
```

Revisa las características requeridas antes de activar una prueba nativa de rendimiento; las unidades dirigidas no necesitan una conexión TikTok real. Consulta `crates/tiktools-core/tests/live_throughput.rs` y las pruebas de la bandeja del escritorio.

## Resultado esperado y estado {#expected-result-and-status}

Las rutas acotadas implementan contadores y avisos de huecos. El [escritor ordenado actual](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/points/queue.rs) rechaza escrituras saturadas antes de modificar memoria y prohíbe escribir síncronamente como alternativa. Los informes antiguos describen defectos de orden ya superados. Evalúa caché y recuperación con código/pruebas actuales; memoria visual acotada no demuestra corrección completa.

## Recuperación {#recover}

Si mejoran los cuadros y empeora la salud contable, revierte el cambio del límite e inspecciona admisión y escritura. Si crecen los temporizadores en pestañas ocultas, revisa la [liberación](lifecycle.md). Conserva cantidades de entrada, aceptadas y pérdidas clasificadas. Un hueco puede recuperar la vista, pero no reparar efectos rechazados o inciertos.
