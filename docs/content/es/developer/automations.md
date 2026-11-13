---
title: "Ejecución de automatizaciones y contratos de scripts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a8902b1b03921afa12cad0cea9bbc55912e6b55081154b3d511577dae3b6a400"
translationRevision: "a8902b1b03921afa12cad0cea9bbc55912e6b55081154b3d511577dae3b6a400"
order: 150
---

# Ejecución de automatizaciones y contratos de scripts

## Objetivo y requisitos

Implementa automatizaciones con descriptores del host y contratos de eventos generados. Trabaja en un perfil privado con efectos externos desactivados hasta probarlos intencionalmente. Conoce la [normalización](events.md) y los [metadatos de control](control-api.md); el editor Vue configura ejecución propiedad de Rust.

## Crear una regla mínima

Una acción de comportamiento guarda un tipo configurado. Un evento selecciona disparador, filtros, enfriamiento e identificadores de acciones. Los workflows componen nodos y puertos versionados. Los descriptores integrados provienen del núcleo y los de plugins de sus manifiestos instalados. No copies otro catálogo de acciones al editor.

1. Selecciona un disparador emitido en `src/automation/contracts/generated/event-registry.generated.ts`.
2. Usa su muestra para elegir una ruta real como `event.user.uniqueId` o `event.data.giftName`.
3. Empieza con registro o una acción desactivada y prueba filtro/enfriamiento con un evento saneado.
4. Guarda mediante el coordinador nativo correspondiente, conservando candidato y versiones revisadas cuando proceda.

```sh
bun scripts/task.ts contracts
bun scripts/task.ts contracts --check
bun scripts/task.ts test bun automation
```

El [directorio de contratos generados](https://github.com/nglmercer/TikTools-app/tree/remake/src/automation/contracts/generated) posee tipos frontend, metadatos y muestras. Para exponer otro campo, amplía el contrato Rust y regenera ambos juntos.

## Límite del script

`core.code` y los nodos script ejecutan QuickJS acotado. Reciben JSON `event`, `inputs` y `data`, alias de inputs. No tienen cargador Node, archivos, red, WebView ni base de datos. Mantén transformaciones deterministas y serializables; emplea acciones del host para HTTP, puntos, audio y plugins. Los límites de código/resultado y presupuesto de ejecución pertenecen al host, no a un temporizador del navegador.

## Resultado esperado y estado

Las reglas guardadas ejecutan mediante el runtime y su intermediario de capacidades. Los procesadores añaden evidencia en `event.intel` sin reemplazar campos originales. Changes de Automations es revisión local, no sincronización de configuración de la cuenta. Variables revisadas tiene implementación nativa real; Changes de Overlays es otro seguimiento sin integrar.

## Recuperación

Ante conflictos, compara los registros actuales sin perder el candidato. Un borrador remoto permanece desactivado hasta revisión y transferencia de autoridad; inspeccionar una vista previa no autoriza ejecución. Los efectos externos inciertos requieren inspección y recuperación retenida. Repetir historial visual nunca debe disparar automatizaciones. Usa [plantillas](templates.md) para importaciones controladas y [pruebas](testing.md) para evidencia por límite.
