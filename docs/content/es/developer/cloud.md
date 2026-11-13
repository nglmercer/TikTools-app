---
title: "Contratos Cloud, sincronización y revisión"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a8e695f0431f934e989dca5460a6338a3a24e04acf0d71b66b7c9050bd4ce839"
translationRevision: "a8e695f0431f934e989dca5460a6338a3a24e04acf0d71b66b7c9050bd4ce839"
order: 180
---

# Contratos Cloud, sincronización y revisión

## Objetivo y requisitos

Desarrolla configuración de cuenta y catálogo contra la instancia Cloud aprovisionada. El funcionamiento exclusivamente local sigue siendo válido. Para trabajo autenticado, utiliza cuenta/backend de prueba y entrega nativa de sesión; esta documentación no demuestra aceptación del token de producción ni de dos dispositivos.

El propietario verificado es [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend), con [contratos v2](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md) y [superficies v3](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md) adicionales. Los tipos portables y transporte del backend están representados localmente en `crates/cloud-contracts` y `crates/cloud-client`. Durabilidad y autenticación del escritorio siguen siendo responsabilidad del host.

## Comprobar capacidades y preparar operaciones

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

## Implementación actual

V2 tiene estado/almacén/coordinador nativos, migración v1 protegida, revisión de conflictos, reconstrucción y reconciliación de plugins instalados. La adopción es explícita y por cuenta; las instalaciones heredadas sin marca mantienen su recorrido anterior. La aceptación completa de dos escritorios y algunos adaptadores siguen pendientes. Configurar épocas de escritura tiene efectos irreversibles de servidor explicados en la revisión de migración.

V3 añade catálogo, equipos, integraciones, recursos, overlays y paquetes según capacidades. Los metadatos del catálogo no sustituyen firmas de plugins. Importar plantillas/perfiles pasa por el diario del propietario y queda desactivado. Aplicar paquetes registra pasos interrumpidos sin repetirlos automáticamente. Las lecturas mediante relay Cloud no equivalen a autoridad de mutación por colaboración directa.

## Resultado esperado y recuperación

Los cambios conservan límites de identidad, revisión y autorización. Changes de Automations es local; Variables revisadas usa métodos tipados nativos reales sin sincronizar valores crudos. Changes de Overlays no está integrado. Ante conflicto, compara base/local/Cloud reciente; ante confirmación incierta, resuelve la intención retenida. Un error HTTP reintentable no autoriza repetir mutaciones. [Colaboración](collaboration.md) explica la autoridad del host activo.
