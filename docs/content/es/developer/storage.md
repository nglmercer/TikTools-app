---
title: "Persistencia y recuperación duradera"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7ec9d94e484af17e8e82aaaa2122cc47ecd206cf9306047795d6aaa72eea2edb"
translationRevision: "896bd5209b72c3b0f4f847f32efb8af9808c3dfc098053237a901a0a6645121b"
order: 60
---

# Persistencia y recuperación duradera {#persistence-and-durable-recovery}

## Objetivo y requisitos {#goal-and-prerequisites}

Modifica los datos persistentes mediante su servicio propietario y conserva resultados recuperables. Utiliza un perfil desechable, detén su host antes de copiar bases de datos y lee los [metadatos de control](control-api.md) antes de editar recursos versionados. `TIKTOOLS_HOME` selecciona la raíz; `AppPaths` resuelve sus datos y almacenes privados.

## Identificar el almacén {#identify-the-store}

`crates/tiktools-core/src/paths.rs` define `data/tiktok-points.db`, `data/tiktok-automation.db` y `data/tiktok-analytics.db`. Los módulos de servicio SQLite poseen sus esquemas. Puntos/espectadores y definiciones de automatización tienen registros de negocio separados; analítica utiliza otra base, ausente en los diagramas antiguos de dos bases.

El inicio puede copiar bases heredadas del checkout a los datos de la plataforma únicamente si el destino no existe. Nunca reemplaza un destino existente. Las definiciones JSON siguen siendo legibles durante cambios de ejecución nativa. No implementes migraciones borrando perfiles ni reescribiendo tablas desde Vue.

## Crear un cambio recuperable {#make-a-recoverable-change}

1. Localiza la transacción del servicio y sus pruebas de esquema y migración.
2. Guarda el cambio, su revisión y la reserva del diario o bandeja de salida en una misma transacción cuando ese dominio lo admita.
3. Conserva las revisiones de borrado. Reutilizar un identificador no debe aceptar una edición anterior a su eliminación.
4. Publica invalidaciones confirmadas mediante la bandeja existente; reintentar su entrega no debe repetir la acción original.
5. Tras una interrupción, abre el mismo perfil privado y resuelve la orden original antes de intentar otra mutación.

Comprobaciones de lectura contra un host de prueba activo:

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system doctor
target/debug/tiktools api discover
```

## Resultado esperado y estado {#expected-result-and-status}

Los flujos duraderos de workflows, puntos y diarios del propietario conservan resultados originales tras reiniciar. Un recibo describe una orden histórica; consulta aparte el estado actual. Los almacenes Cloud añaden particiones por origen/cuenta y comprobaciones de generación sin reemplazar la autoridad SQLite local. La cobertura no es idéntica para todos los efectos externos o dominios.

## Recuperación {#recovery}

Conserva juntos SQLite y sus archivos WAL/SHM al crear una copia fuera de línea. Un tiempo de espera de la interfaz no demuestra reversión. Ante `revision_conflict`, guarda el borrador y compara una lectura reciente. Para efectos inciertos, inspecciona su registro en lugar de repetir el feed. Consulta la [autoridad de rutas](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/paths.rs) y la [recuperación Cloud](cloud.md).

## Detalles de operación {#operational-details}

Consulta rutas antes de experimentar/copiar con `tiktools api call system.info --json`. `TIKTOOLS_HOME` elige raíz; `TIKTOOLS_DATA_DIR`, `TIKTOOLS_PLUGINS_DIR`, `TIKTOOLS_PLUGIN_DATA_DIR`, `TIKTOOLS_PLUGIN_STORE_DIR`, `TIKTOOLS_LOG_DIR` y `TIKTOOLS_TEMP_DIR` sustituyen sus directorios derivados. Los valores relativos parten del directorio de lanzamiento. Una raíz aislada no anula overrides heredados: revísalos/retíralos primero.

Analytics guarda agregados con escritor propio: cola actual 1.024, lotes 128 y flush de 10 s. El archivo vecino `*.pending.json` deriva de la ruta de su base. El arranque reproduce lotes con ID y retira el archivo antes de que `ready` admita trabajo. Recuperación ilegible/mayor de 16 MiB, base indisponible o fallo al retirar archivo mantienen admisión deshabilitada. Conserva base y archivo, revisa `ready`/pendientes/errores y respalda antes de reparar. No borres evidencia pendiente para aparentar salud; los diarios de puntos tienen otro contrato durable.

`system.info` informa home/dataDir efectivos; resuelve los demás directorios con el entorno real de lanzamiento del host, no otra terminal. Usa `system.health` para ready/pendientes analíticos y contadores de ingreso LIVE.
