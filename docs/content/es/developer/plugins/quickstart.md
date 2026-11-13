---
title: "Crear el primer plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "66a07fff758de08ce36d8c6494f7b8eb3d56aa1ad30bae5b5eaed42baa11956c"
translationRevision: "66a07fff758de08ce36d8c6494f7b8eb3d56aa1ad30bae5b5eaed42baa11956c"
order: 250
---

# Crear el primer plugin

## Objetivo y requisitos

Produce un paquete validado que devuelva un resumen de acción sin efectos. Usa el devkit `tiktools-plugin` correspondiente al repositorio y Bun 1.4.4 para TypeScript. Crea la muestra en otro directorio de desarrollo; es un ejemplo de autoría, no un cambio de dependencias de la aplicación.

## Crear y validar

Desde el checkout de TikTools, compila la CLI de autor mediante el ejecutor:

```sh
bun scripts/task.ts rust build -p tiktools-plugin-devkit --locked
```

Añade su `target/debug/tiktools-plugin` a la ruta de herramientas y, en el directorio elegido:

```sh
tiktools-plugin new acme.hello --template napi-vm-ts --dir acme-hello
cd acme-hello
tiktools-plugin validate
tiktools-plugin validate --publish
```

La plantilla genera manifiesto de esquema v3, identificador estable con editor, descriptor de acción, importación SDK y compilación. Mantén alineados runtime `napi-vm`, `apiVersion:1` y entrada compilada. El nombre es histórico; hoy ejecuta QuickJS.

## Escribir y compilar

La acción generada puede permanecer dentro del resultado tipado:

```ts
import { actionConfig, actionResult, definePlugin } from '@tiktools/plugin-sdk';

export default definePlugin({
  call(request) {
    if (request.type !== 'action') return {};
    return actionResult(String(actionConfig(request).message ?? 'Hello'));
  },
});
```

Instala las dependencias declaradas por el paquete generado dentro de ese paquete y ejecuta `tiktools-plugin build --release`. Compila JavaScript anticipadamente y crea el archivo. No hay transpilación TypeScript ni proceso Node durante ejecución. El [SDK TypeScript](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/src/plugin.ts) define las utilidades; el esquema ayuda al editor, pero el analizador del host mantiene autoridad.

## Resultado esperado y estado

`validate` informa compatibilidad y problemas de publicación; compilar correctamente produce un `.plugin` con manifiesto, entrada e integridad. Una acción que solo devuelve resumen no necesita efectos adicionales. Si añades eventos, HTTP o audio, actualiza declaraciones y revisa autoridad antes de probar.

## Recuperación y verificación

Si falta la entrada, compila y comprueba su ruta relativa al manifiesto. No dirijas runtime de proceso a JavaScript sin compilar. Para preparación local utiliza `tiktools-plugin dev stage --into /absolute/path/dev-plugins` y un host privado configurado para esa raíz. Preparación e instalación son flujos separados; no demuestra confianza de registro firmado. Continúa con [límites del runtime](runtimes.md) y [firmas](packaging-registry.md).
