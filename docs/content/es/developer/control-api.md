---
title: "Contratos y descubrimiento de Control API"
docId: "developer.control-api"
translationKey: "developer.control-api"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "9ce931bc43614d39b264662b83fbdf3ddc7df875ef7cd6193fc6742eed34c509"
translationRevision: "d0a961d389aaf7bb3aa8122df95c472f673842332b1e76679449293bd51d4438"
order: 70
---

# Contratos y descubrimiento de Control API {#control-api-contracts-and-discovery}

## Objetivo y requisitos {#goal-and-prerequisites}

Llama al contrato publicado por el host sin inventar métodos ni esquemas de respuesta. Utiliza un host privado activo y la CLI correspondiente. El [registro Rust canónico](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-control-api/src/registry.rs) describe los métodos; los módulos de registro aportan esquemas, capacidades, requisitos de escritorio y concurrencia. Descubre la compilación real antes de elegir una operación.

## Inspeccionar y llamar {#inspect-then-call}

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

## Conservar los metadatos {#preserve-metadata}

Las llamadas versionadas pueden llevar `meta.expectedRevision` en el nivel superior. Las duraderas también vinculan identificador de orden e instancia del host según descubrimiento. Lee `meta.resourceVersions`, `meta.command` y `meta.hostInstanceId` junto con el resultado del dominio. Las revisiones son cadenas decimales, no números JavaScript. `callWithMeta`, en `src/web/shared/api/control-contracts.ts`, conserva este sobre.

## Resultado esperado y estado {#expected-result-and-status}

Un método descubierto está disponible en esa compilación; siguen aplicándose permisos remotos y restricciones de recursos. Los métodos del propietario local no se convierten automáticamente en métodos remotos. Las solicitudes genéricas recorren la misma operación que el SDK tipado. Las referencias técnicas compartidas deben derivarse del registro y sus esquemas, evitando otra tabla manual completa.

## Recuperación {#recovery}

Ante `method_not_found`, compara descubrimiento y versiones. Ante `revision_conflict`, conserva el candidato y compara las revisiones actuales. Un tiempo de espera de transporte puede dejar una mutación aceptada en ejecución; consulta su identificador original cuando esté retenido. No generes otro identificador para eludir la incertidumbre. Continúa con [agentes](agents.md) o [recuperación remota](remote-client.md).

## Referencia versionada de métodos {#method-reference}

Generada desde el snapshot versionado; el host en ejecución sigue siendo la autoridad.

<!--@include: ../../../.generated/control-api.es.md-->

## Detalles de operación {#operational-details}

Usa un `TIKTOOLS_HOME` descartable, retira overrides heredados según [almacenamiento](storage.md), inicia `tiktools host --ipc` y utiliza la misma raíz en la terminal de control. La receta presupone el workflow descartable `docs-example`; crea primero la muestra deshabilitada de [automatizaciones](automations.md#operational-details). Conserva revisión de `meta.resourceVersions` y `meta.hostInstanceId` de la lectura; sustituye `REVISION`/`HOST_INSTANCE`. Mantén un ID estable y su payload exacto. Tras timeout consulta el ID original y refresca el recurso; `accepted`, `running`, `unknown` no autorizan otro efecto. Un conflicto exige nueva lectura y revisión, no sobrescritura automática. `points.adjust` es aditivo: admite ID de comando pero no revisión esperada.

`--include-meta` conserva resultado y metadatos; la salida predeterminada extrae el resultado. `--dry-run --explain` valida/planifica sin ejecutar; explicaciones a stderr y JSON a stdout. Operaciones destructivas exigen `--confirm`. La salida redacta secretos reconocidos; `--secrets-visible` solo cambia presentación CLI y no recupera valores ocultados por el host. Revisa manualmente. `api events` limita tiempo/conteo y filtra temas. Discovery del host sigue mandando sobre features/permisos; las descripciones generadas conservan el texto exacto del registro.

```sh
tiktools api discover --json
tiktools api schema workflows.disable --json
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --explain --json
tiktools api call workflows.disable '{"id":"docs-example"}' --expected-revision REVISION --command-id docs-disable-001 --host-instance-id HOST_INSTANCE --include-meta --json
tiktools api call collaboration.commands.get '{"commandId":"docs-disable-001"}' --json
tiktools api call points.adjust '{"uniqueId":"docs-viewer","delta":1}' --command-id docs-points-001 --host-instance-id HOST_INSTANCE --include-meta --dry-run --json
tiktools api events --topics live.status --timeout 5 --max-events 2
```

```json
{"jsonrpc":"2.0","id":1,"method":"workflows.disable","params":{"id":"docs-example"},"meta":{"expectedRevision":"1","commandId":"docs-disable-001","hostInstanceId":"HOST_INSTANCE"}}
```

```json
{"jsonrpc":"2.0","id":1,"result":{"id":"docs-example","enabled":false},"meta":{"hostInstanceId":"HOST_INSTANCE","resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}],"command":{"commandId":"docs-disable-001","state":"succeeded"}}}
```

```json
{"jsonrpc":"2.0","id":1,"error":{"code":"revision_conflict","message":"Resource revision changed","data":{"resourceVersions":[{"kind":"workflow","id":"docs-example","scope":"host","revision":"2"}]}}}
```
