---
title: "Referencia de comandos CLI"
docId: "developer.cli"
translationKey: "developer.cli"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
translationRevision: "4bed4172e24c685db852a6786742e40d68364c85f0ce82373b5d5eb93d58d0e0"
order: 75
---

# Referencia de comandos CLI {#cli-command-reference}

## Objetivo y requisitos {#goal-and-prerequisites}

Descubre cada forma de comando `tiktools` sin leer el código Rust ni analizar la salida de ayuda humana. Necesitas un binario `tiktools` compilado correspondiente; la referencia es offline y nunca requiere un host activo. La fuente canónica es el registro de especificaciones en `crates/tiktools-cli/src/commands/spec.rs`; cada módulo de dominio posee las cadenas de uso que su parser reporta en errores, por lo que esta referencia no puede divergir de lo que el parser acepta.

## Inspeccionar la ayuda offline {#inspect-help-offline}

La ayuda humana agrupa los verbos por dominio; el filtro muestra una sección de dominio. La forma `--json` imprime el registro legible por máquina que todo agente debería preferir: uso, resumen y marcador destructivo por verbo, más las convenciones globales de la CLI, versionado por el campo `schema` superior.

```sh
tiktools help
tiktools help plugin
tiktools help --json
```

Los errores de uso imprimen solo la forma relevante con una pista en lugar de la página completa, así que un error tipográfico nunca desplaza la terminal. El código de salida 2 siempre significa error de uso; consulta el pie de convenciones para las reglas de resolución del host, salida y redacción de secretos.

## Referencia de comandos generada {#generated-command-reference}

Generada desde la instantánea de ayuda CLI versionada en `docs/contracts/cli-help.json`; el binario sigue siendo la autoridad. Las líneas de uso son sintaxis del parser y permanecen en inglés en todas las locales; los resúmenes y convenciones provienen de los catálogos de traducción de la propia CLI, por lo que esta referencia no puede divergir de lo que `tiktools help` imprime en el mismo idioma. Los verbos marcados como destructivos mutan o destruyen estado; los scripts deberían confirmar antes de ejecutarlos.

<!--@include: ../../../.generated/cli.es.md-->

## Detalles de operación {#operational-details}

La instantánea se regenera desde un binario compilado con `--features local-web` para que los dominios condicionados por features (cloud, publisher, web) estén completos. La verificación de procedencia en `docs check` compara hashes de cada fuente Rust, así que cualquier cambio en las especificaciones de comandos falla la compilación hasta regenerar el contrato. Las compilaciones normales de documentación nunca ejecutan Cargo.

```sh
bun docs/scripts/regenerate-control.ts
bun docs/scripts/generate-references.ts --check
bun docs/scripts/cli-coverage.ts
```

Ejecuta la regeneración tras cambiar cualquier parser o especificación de comando, y revisa el diff del fragmento como parte del cambio. Los agentes que consumen `help --json` deberían tratar la versión 1 del esquema como estable de forma aditiva: pueden aparecer nuevos verbos, dominios o campos sin aviso; las claves existentes mantienen su significado.
