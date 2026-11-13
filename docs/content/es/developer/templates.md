---
title: "Plantillas, perfiles y revisión del propietario"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
translationRevision: "d7dba1e983106bce9842f50ee83567f002bbe0c28910dcd136221d3af2db0cda"
order: 160
---

# Plantillas, perfiles y revisión del propietario {#rule-templates-profiles-and-owner-review}

## Objetivo y requisitos {#goal-and-prerequisites}

Importa reglas reutilizables conservando propiedad local y recuperación. Necesitas un host privado, los proveedores de acciones instalados y un `.tiktemplate.json` o `.tikprofile.json` válido. Lee [automatizaciones](automations.md) antes de habilitar una regla importada. Importar configuración no demuestra que su integración externa sea accesible.

## Preparar el documento {#prepare-a-document}

Una plantilla v1 incluye `templateVersion`, identificador portable, esquema de parámetros, acciones y un evento vinculado. Un perfil contiene `profileVersion`, parámetros compartidos y plantillas incluidas. Los analizadores autoritativos están en `crates/tiktools-core/src/services/templates/` y `src/web/entities/rule-template/lib/`; las muestras de `tests/fixtures/templates/` mantienen su concordancia.

Los parámetros se combinan desde valores predeterminados del esquema hasta perfil, entrada y valores explícitos del diálogo/CLI. Un segmento completo <code v-pre>{{ params.* }}</code> conserva el tipo JSON; los incrustados producen texto. Las sustituciones de evento quedan para la ejecución. Los globales se resuelven después y no deben incrustarse como secretos de cuenta en documentos compartibles.

## Previsualizar e importar {#preview-and-import}

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

Los ejemplos suponen un archivo preparado y un host compatible activo. En la interfaz, Rule templates y la importación de perfiles preparan un candidato para revisión. El coordinador nativo `automation.library.*` controla transiciones atómicas y recuperación del resultado original; su esquema actual se descubre mediante [Control API](control-api.md).

## Resultado esperado y estado {#expected-result-and-status}

Las importaciones revisadas crean registros vinculados coherentes y conservan un diario del propietario. El catálogo Cloud guarda documentos de plantilla; importar perfiles y aplicar plantillas exige revisar enabled, sin asumir desactivación. Un proveedor ausente o deshabilitado causa un error de requisitos, sin sustituir acciones silenciosamente. El perfil seleccionado controla el contexto correspondiente; seleccionarlo no concede autoridad remota.

## Recuperación y verificación {#recover-and-verify}

Ante conflictos de revisión, conserva el candidato, carga una instantánea y revisa la diferencia real. Resuelve cualquier diario pendiente antes de crear otra intención. No conviertas una actualización fallida en una creación implícita. Ejecuta `bun scripts/task.ts test web rule-templates` y `bun scripts/task.ts test web library-journal`. El [servicio nativo](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) y los esquemas RPC sustituyen referencias antiguas a `src/web/views/behavior`.

## Detalles de operación {#operational-details}

Guarda el primer JSON como `docs-log.tiktemplate.json`; el segundo es un `docs-pack.tikprofile.json` completo de una entrada. Ambos desactivan acciones/eventos expresamente. Precedencia: default del schema → perfil → entrada → valores CLI/diálogo. Un span completo de parámetro conserva tipo; uno incrustado se vuelve texto. Si falta enabled se usa true: emplea `--disabled` y revisa la comparación UI.

Límites actuales: 8 acciones y 12 filtros/plantilla, 32 plantillas/perfil; texto UI 64 KiB/plantilla y 256 KiB/perfil; expansión profundidad 64, 65.536 nodos, 256 KiB/registro y 8 MiB total. Ante exceso reduce contenido, no evites validación. Los límites de biblioteca, fuente y expansión son distintos. Sigue [recuperación de diario](automations.md#operational-details) antes de otro import si el resultado es incierto.

```json
{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false,"filters":[],"cooldownMs":1000,"cooldownScope":"global","runMode":"all"}}
```

```json
{"profileVersion":1,"id":"docs-pack","name":"Docs pack","description":"","params":{"message":"profile"},"templates":[{"params":{"message":"entry"},"template":{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false}}}]}
```

```sh
tiktools template import docs-log.tiktemplate.json --dry-run
tiktools template import docs-log.tiktemplate.json --disabled
tiktools template profile-import docs-pack.tikprofile.json --disabled
```
