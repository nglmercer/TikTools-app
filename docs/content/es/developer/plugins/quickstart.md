---
title: "Crear el primer plugin"
docId: "developer.plugins.quickstart"
translationKey: "developer.plugins.quickstart"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "17f8e12522f04bad46b814f437e80c3f480b43331ed1278e49597878e2cee4f8"
translationRevision: "fe261fa9392d23b160448f609b91a26d50370f9cd52f2714eec559cd678a45de"
order: 250
---

# Crear el primer plugin {#build-a-first-plugin}

## Objetivo y requisitos {#goal-and-prerequisites}

Produce un paquete validado que devuelva un resumen de acción sin efectos. Usa el devkit `tiktools-plugin` correspondiente al repositorio y Bun 1.4.4 para TypeScript. Crea la muestra en otro directorio de desarrollo; es un ejemplo de autoría, no un cambio de dependencias de la aplicación.

## Crear y validar {#scaffold-and-validate}

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

## Escribir y compilar {#write-and-build-the-guest}

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

## Resultado esperado y estado {#expected-result-and-status}

`validate` informa compatibilidad y problemas de publicación; compilar correctamente produce un `.plugin` con manifiesto, entrada e integridad. Una acción que solo devuelve resumen no necesita efectos adicionales. Si añades eventos, HTTP o audio, actualiza declaraciones y revisa autoridad antes de probar.

## Recuperación y verificación {#recovery-and-verification}

Si falta la entrada, compila y comprueba su ruta relativa al manifiesto. No dirijas runtime de proceso a JavaScript sin compilar. Para preparación local utiliza `tiktools-plugin dev stage --into /absolute/path/dev-plugins` y un host privado configurado para esa raíz. Preparación e instalación son flujos separados; no demuestra confianza de registro firmado. Continúa con [límites del runtime](runtimes.md) y [firmas](packaging-registry.md).

## Detalles de operación {#operational-details}

Lee `hostCompatibility`, `publishReady` y `warnings` del resultado de `validate`. Una entrada sin compilar es advertencia; `--publish` falla por problemas de publicación, no por todas las advertencias de compatibilidad. Revalida tras compilar, inspecciona entrada/destino empaquetados y prueba el host destino. Salida correcta antes de compilar no prueba paquete utilizable.

Para Rust, crea `process-rust` o `native-rust` en directorio separado. Ambos implementan `Plugin` del SDK y devuelven `ActionResult::summary`; proceso usa `tiktools_process_plugin!`, nativo usa `tiktools_export_native_plugin!` y `cdylib`. ABI/confianza nativa son específicos del destino y en proceso. Compila con la sintaxis de targets del devkit, conserva la ruta generada y usa host privado. Prefiere proceso cuando convenga aislamiento de fallos.

```sh
tiktools-plugin new acme.process --template process-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-process
tiktools-plugin new acme.native --template native-rust --sdk-path /absolute/path/TikTools/crates/tiktools-plugin-sdk --dir acme-native
tiktools-plugin build acme-process --release
tiktools-plugin validate acme-process --publish
tiktools-plugin build acme-native --release
tiktools-plugin validate acme-native --publish
```
