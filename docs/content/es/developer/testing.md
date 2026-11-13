---
title: "Pruebas y límites de la evidencia"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
translationRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
order: 210
---

# Pruebas y límites de la evidencia {#tests-and-evidence-boundaries}

## Objetivo y requisitos {#goal-and-prerequisites}

Elige comprobaciones capaces de demostrar el comportamiento modificado. Usa Bun 1.4.4, instalación limpia del archivo de bloqueo y Rust solo para los límites nativos afectados. Aísla las pruebas de hosts, credenciales y cuentas de producción. Una muestra de navegador exitosa demuestra su propio límite, no todas las integraciones inferiores.

## Elegir una suite dirigida {#choose-a-focused-suite}

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

La suite rápida combina unidades Bun y Vitest de aplicación/widgets. El ejecutor dirigido valida argumentos y rechaza filtros Rust sin coincidencias en lugar de aceptar cero pruebas. Usa el acceso `rust` cuando necesites banderas Cargo fuera de la gramática dirigida.

## Verificar el límite real {#verify-the-actual-boundary}

Las pruebas Vue utilizan el cliente de control falso compartido. Playwright principal ejecuta la misma SPA contra un host `window.ipc` falso instalado antes de navegar. Las llamadas RPC desconocidas fallan y se comprueban errores de consola/página. Ejecuta Chromium, no Wry. Las suites de interfaz de plugins viven en su repositorio externo. Las pruebas de gateway de widgets verifican cliente, protocolo y recursos compilados.

Para trabajo de navegador autorizado, tras compilar recursos utiliza `bun scripts/task.ts test e2e`. Lanza un navegador y por eso se separa de la edición documental y revisión de código sin interfaz. Shell nativo, OBS Browser Source, teléfono, almacén seguro del sistema, sustitución del actualizador y dos equipos necesitan sus entornos reales y evidencia explícita.

## Tiempo de ejecución y coste CI {#execution-time-and-ci-cost}

Empieza localmente por un filtro de archivo/nombre o `bun scripts/task.ts test --changed`. La suite completa conserva el aislamiento de archivos; Vitest usa `vmThreads` para reutilizar el entorno DOM por worker sin compartir el estado de la aplicación entre archivos.

CI compara el rango completo del push o pull request. Las rutas documentales revisadas omiten jobs nativos; los cambios de Vue/demo ejecutan la puerta frontend. Código nativo, infraestructura de compilación, manifiestos de dependencias, rutas desconocidas o una comparación no disponible conservan todas las puertas existentes. `scripts/ci-scope.mjs` contiene la política probada de selección. CI manual ofrece `frontend`, `native` o `full`; las llamadas reutilizables validan todo por defecto. El estado final CI rechaza jobs seleccionados fallidos o cancelados.

Las cachés Rust se separan por función, sistema operativo, toolchain, lockfile y revisión de fuentes. CI conserva tablas de líneas para depuración y desactiva sesiones incrementales para reducir el volumen de caché. Las pruebas seriales de SQLite/perfiles siguen siendo seriales hasta aislar sus recursos compartidos. No elimines esa protección para obtener un tiempo menor.

El workflow del sitio es manual: editar o subir documentación no inicia otra compilación remota. La recuperación de releases también es manual: por defecto comprueba solo Linux y no publica. Publicar requiere las puertas de ambas plataformas y una elección explícita. No inicies ni repitas trabajo en runners de pago solo para confirmar una edición Markdown.

## Resultado esperado y estado {#expected-result-and-status}

El informe identifica suite aprobada y alcance. Los cambios de implementación requieren lint, tipos, compilación y puertas CI correspondientes; el contenido usa `bun scripts/task.ts docs check` y `docs test`. No cambies dependencias para que una prueba descargue herramientas más recientes.

## Recuperación {#recovery}

Si un filtro no encuentra nada, corrígelo sin declarar éxito. Si una muestra carece de un RPC nuevo, amplía su contrato explícito en lugar de aceptar cualquier llamada. Mantén visibles las verificaciones manuales inciertas u omitidas. El [ejecutor de pruebas](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts) define la sintaxis vigente; [rendimiento](performance.md) explica evidencia reproducible bajo carga.
