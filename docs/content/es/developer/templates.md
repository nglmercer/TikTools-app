---
title: "Plantillas, perfiles y revisión del propietario"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
translationRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
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

Las importaciones revisadas crean registros vinculados coherentes y conservan un diario del propietario. Las plantillas/perfiles del catálogo Cloud permanecen desactivados hasta su activación intencional. Un proveedor ausente o deshabilitado causa un error de requisitos, sin sustituir acciones silenciosamente. El perfil seleccionado controla el contexto correspondiente; seleccionarlo no concede autoridad remota.

## Recuperación y verificación {#recover-and-verify}

Ante conflictos de revisión, conserva el candidato, carga una instantánea y revisa la diferencia real. Resuelve cualquier diario pendiente antes de crear otra intención. No conviertas una actualización fallida en una creación implícita. Ejecuta `bun scripts/task.ts test web rule-templates` y `bun scripts/task.ts test web library-journal`. El [servicio nativo](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) y los esquemas RPC sustituyen referencias antiguas a `src/web/views/behavior`.
