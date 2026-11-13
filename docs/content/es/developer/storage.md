---
title: "Persistencia y recuperación duradera"
docId: "developer.storage"
translationKey: "developer.storage"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
translationRevision: "f6f86e9e87d6727f4dace206cef26a37d501f8030f7a1ff42688af77592a7445"
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
