---
title: "Ejecución de automatizaciones y contratos de scripts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "57cc664e4ef2ff26fafb31df6458da439f075c673fbb3e06e635fa03c1032f42"
translationRevision: "d769546add745734d5ff24ec4c46522b5edfab0a55efed05d06e1243473e2e3f"
order: 150
---

# Ejecución de automatizaciones y contratos de scripts {#automation-execution-and-script-contracts}

## Objetivo y requisitos {#goal-and-prerequisites}

Implementa automatizaciones con descriptores del host y contratos de eventos generados. Trabaja en un perfil privado con efectos externos desactivados hasta probarlos intencionalmente. Conoce la [normalización](events.md) y los [metadatos de control](control-api.md); el editor Vue configura ejecución propiedad de Rust.

## Crear una regla mínima {#build-a-minimal-rule}

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

## Límite del script {#script-boundary}

`core.code` y los nodos script ejecutan QuickJS acotado. Reciben JSON `event`, `inputs` y `data`, alias de inputs. No tienen cargador Node, archivos, red, WebView ni base de datos. Mantén transformaciones deterministas y serializables; emplea acciones del host para HTTP, puntos, audio y plugins. Los límites de código/resultado y presupuesto de ejecución pertenecen al host, no a un temporizador del navegador.

## Resultado esperado y estado {#expected-result-and-status}

Las reglas guardadas ejecutan mediante el runtime y su intermediario de capacidades. Los procesadores añaden evidencia en `event.intel` sin reemplazar campos originales. Changes de Automations es revisión local, no sincronización de configuración de la cuenta. Variables revisadas tiene implementación nativa real; Changes de Overlays es otro seguimiento sin integrar.

## Recuperación {#recovery}

Ante conflictos, compara los registros actuales sin perder el candidato. Un borrador remoto permanece desactivado hasta revisión y transferencia de autoridad; inspeccionar una vista previa no autoriza ejecución. Los efectos externos inciertos requieren inspección y recuperación retenida. Repetir historial visual nunca debe disparar automatizaciones. Usa [plantillas](templates.md) para importaciones controladas y [pruebas](testing.md) para evidencia por límite.

## Detalles de operación {#operational-details}

El grafo siguiente usa puertos v1 de catálogo trigger → transform → log y está desactivado. Guárdalo como `docs-graph.json` y contrasta versiones/puertos con `automation.nodes.list`. `workflow save --record` persiste; `workflow get` y enable/disable con metadatos verifican estado guardado. Usa revisiones recién leídas. Guardar/habilitar no prueba ejecutor completo ni editor visual en todo build: el runtime normal de este checkout ejecuta Eventos/Acciones de comportamiento. Usa la plantilla de log de [plantillas](templates.md#operational-details), revísala, habilita intencionalmente ambos registros y prueba chat saneado para observar un efecto inocuo. Guardar grafo no demuestra ejecución LIVE.

Para enriquecimiento, instala/habilita TextIntel con `events.enrich`, revisa estado/métricas en Processors y ejecuta su muestra. Inspecciona `event.intel.providers.textintel.comment.moderation.blocked` antes de crear filtro is-true hacia `core.log`; la ausencia del campo no es evidencia falsa. Circuito abierto o saturación pueden omitir procesadores. Una muestra no valida LIVE completo.

Ante trabajo incierto consulta `automation.library.get`, `automation.definition.get`, `automation.adoption.get` o `automation.approval.get`. Conserva pending.commandId, identidad de host y candidato. Resuelve la operación original del diario (`*.resolve`); si es unknown, revisa estado actual antes de `*.review_unknown` con ID original y aceptación explícita. Adoptar exige revisión nueva de autoridad/dependencias, no basta inspect/preview. `automation.effects.get {effectId}` inspecciona efecto externo retenido: no autoriza despacho ni reintento ciego. Conserva lo desconocido para reconciliación operativa.

```json
{"schemaVersion":1,"id":"docs-example","name":"Docs","enabled":false,"nodes":[{"id":"start","type":"trigger.event","version":1,"position":{"x":0,"y":0},"config":{"eventType":"tiktok.chat"}},{"id":"text","type":"transform.template","version":1,"position":{"x":200,"y":0},"config":{"template":"docs-example"}},{"id":"log","type":"action.log","version":1,"position":{"x":400,"y":0},"config":{"message":"docs-example"}}],"edges":[{"id":"a","kind":"flow","source":"start","sourcePort":"flow","target":"text","targetPort":"flow"},{"id":"b","kind":"flow","source":"text","sourcePort":"flow","target":"log","targetPort":"flow"}]}
```

```sh
tiktools api call automation.nodes.list '{}' --json
tiktools workflow save --record "$(cat docs-graph.json)"
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.enable '{"id":"docs-example"}' --expected-revision REVISION --dry-run --json
tiktools api call automation.library.get '{}' --json
tiktools api call automation.adoption.get '{}' --json
```
