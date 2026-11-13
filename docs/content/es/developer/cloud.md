---
title: "Contratos Cloud, sincronización y revisión"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "06ae46545d1366d4f456c78a0e6583ea0338afbd496467e4d25b3abe1a5430fb"
translationRevision: "0713578c1bd68eec313961653afc6d68eaaa387e0cfb83da739808ae62b926b6"
order: 180
---

# Contratos Cloud, sincronización y revisión {#cloud-contracts-synchronization-and-review}

## Objetivo y requisitos {#goal-and-prerequisites}

Desarrolla configuración de cuenta y catálogo contra la instancia Cloud aprovisionada. El funcionamiento exclusivamente local sigue siendo válido. Para trabajo autenticado, utiliza cuenta/backend de prueba y entrega nativa de sesión; esta documentación no demuestra aceptación del token de producción ni de dos dispositivos.

El propietario verificado es [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend), con [contratos v2](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md) y [superficies v3](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md) adicionales. Los tipos portables y transporte del backend están representados localmente en `crates/cloud-contracts` y `crates/cloud-client`. Durabilidad y autenticación del escritorio siguen siendo responsabilidad del host.

## Comprobar capacidades y preparar operaciones {#gate-and-stage-operations}

1. Descubre métodos nativos con `rpc.discover`; un host antiguo puede conservar vistas heredadas.
2. Deja que el host consulte `/api/v2/capabilities` antes del endpoint protegido. No conviertas una capacidad del servidor en una bandera fija del navegador.
3. Usa el origen aprovisionado por el operador. Las versiones de distribución no aceptan sustituciones arbitrarias en ejecución.
4. Previsualiza migración y conflictos, conserva bases congeladas y escrituras inciertas, y envía únicamente lo revisado.

```sh
tiktools api discover
tiktools api schema sync.v2.migrate
tiktools api schema globals.typed.replace
```

Estas órdenes inspeccionan esquemas; no migran cuentas ni guardan variables.

## Implementación actual {#current-implementation}

V2 tiene estado/almacén/coordinador nativos, migración v1 protegida, revisión de conflictos, reconstrucción y reconciliación de plugins instalados. La adopción es explícita y por cuenta; las instalaciones heredadas sin marca mantienen su recorrido anterior. La aceptación completa de dos escritorios y algunos adaptadores siguen pendientes. Configurar épocas de escritura tiene efectos irreversibles de servidor explicados en la revisión de migración.

V3 añade catálogo, equipos, integraciones, recursos, overlays y paquetes según capacidades. Los metadatos del catálogo no sustituyen firmas de plugins. Importar plantillas/perfiles pasa por el diario del propietario; revisa enabled al aplicar/importar perfil. Aplicar paquetes registra pasos interrumpidos sin repetirlos automáticamente. Las lecturas mediante relay Cloud no equivalen a autoridad de mutación por colaboración directa.

## Resultado esperado y recuperación {#expected-result-and-recovery}

Los cambios conservan límites de identidad, revisión y autorización. Changes de Automations es local; Variables revisadas usa métodos tipados nativos reales sin sincronizar valores crudos. Changes de Overlays no está integrado. Ante conflicto, compara base/local/Cloud reciente; ante confirmación incierta, resuelve la intención retenida. Un error HTTP reintentable no autoriza repetir mutaciones. [Colaboración](collaboration.md) explica la autoridad del host activo.

## Detalles de operación {#operational-details}

`TIKTOOLS_CLOUD_ORIGIN` usa runtime → valor compilado → default en debug, pero compilado → default en release: release ignora overrides runtime. Vacío, `off`, `none` deshabilitan solo en la capa aplicable. Admite origen HTTPS sin credenciales/ruta/query/fragmento; HTTP solo loopback explícito. Debug aislado: `TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:8787 bun run start`; release: `TIKTOOLS_CLOUD_ORIGIN=https://cloud.example bun scripts/task.ts rust build -p tiktools-desktop --release --locked`. No restaura identidad de otro origen: revisa origen/cuenta antes de autenticar. Un default compilado no prueba disponibilidad del servicio.

Sigue [migración y recuperación por modo](../user/cloud/changes.md#operational-details). Coordina dispositivos antes de política irreversible de época, conserva conteos/resultados y consulta estado tras barrera ambigua; no la repitas para averiguar si ocurrió. La recuperación congela entradas y revisión aprobadas; una revisión caducada se reconstruye antes de reanudar, y reanudar no envía escritura Cloud.
