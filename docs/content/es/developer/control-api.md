---
title: "Contratos y descubrimiento de Control API"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4f959d837a2a1c3ff01346f2d3ad63ed5fc68680818d711b2aae995171b3703c"
translationRevision: "4f959d837a2a1c3ff01346f2d3ad63ed5fc68680818d711b2aae995171b3703c"
order: 70
---

# Contratos y descubrimiento de Control API

## Objetivo y requisitos

Llama al contrato publicado por el host sin inventar métodos ni esquemas de respuesta. Utiliza un host privado activo y la CLI correspondiente. El [registro Rust canónico](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs) describe los métodos; los módulos de registro aportan esquemas, capacidades, requisitos de escritorio y concurrencia. Descubre la compilación real antes de elegir una operación.

## Inspeccionar y llamar

```sh
target/debug/tiktools api discover
target/debug/tiktools api schema system.info
target/debug/tiktools api call system.info '{}' --format json
target/debug/tiktools api verify --sandbox
```

La última orden utiliza un runtime de verificación aislado. No valida una conexión LIVE real. En stdio/IPC local se transmite JSON delimitado por líneas; WSS remoto transporta un mensaje por trama y exige primero `collaboration.hello`.

```json
{"jsonrpc":"2.0","id":"info-1","method":"system.info","params":{}}
```

La respuesta devuelve el identificador y contiene `result` o `error`. Los errores emplean `code` textual, mensaje y `data` segura opcional; no supongas los códigos numéricos de otro servidor JSON-RPC. Las notificaciones utilizan `method:"event"` y `params:{topic,data}`. Sepáralas de la resolución de respuestas pendientes.

## Conservar los metadatos

Las llamadas versionadas pueden llevar `meta.expectedRevision` en el nivel superior. Las duraderas también vinculan identificador de orden e instancia del host según descubrimiento. Lee `meta.resourceVersions`, `meta.command` y `meta.hostInstanceId` junto con el resultado del dominio. Las revisiones son cadenas decimales, no números JavaScript. `callWithMeta`, en `src/web/shared/api/control-contracts.ts`, conserva este sobre.

## Resultado esperado y estado

Un método descubierto está disponible en esa compilación; siguen aplicándose permisos remotos y restricciones de recursos. Los métodos del propietario local no se convierten automáticamente en métodos remotos. Las solicitudes genéricas recorren la misma operación que el SDK tipado. Las referencias técnicas compartidas deben derivarse del registro y sus esquemas, evitando otra tabla manual completa.

## Recuperación

Ante `method_not_found`, compara descubrimiento y versiones. Ante `revision_conflict`, conserva el candidato y compara las revisiones actuales. Un tiempo de espera de transporte puede dejar una mutación aceptada en ejecución; consulta su identificador original cuando esté retenido. No generes otro identificador para eludir la incertidumbre. Continúa con [agentes](agents.md) o [recuperación remota](remote-client.md).

## Referencia versionada de métodos {#method-reference}

Generada desde el snapshot versionado; el host en ejecución sigue siendo la autoridad.

<!--@include: ../../../.generated/control-api.es.md-->
