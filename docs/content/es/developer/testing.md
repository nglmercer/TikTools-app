---
title: "Pruebas y límites de la evidencia"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "be694b0c471f68219481d14ca77bf7842c6b7935e4c97b5dbe6cd2a8cfe1fc1d"
translationRevision: "be694b0c471f68219481d14ca77bf7842c6b7935e4c97b5dbe6cd2a8cfe1fc1d"
order: 210
---

# Pruebas y límites de la evidencia

## Objetivo y requisitos

Elige comprobaciones capaces de demostrar el comportamiento modificado. Usa Bun 1.4.4, instalación limpia del archivo de bloqueo y Rust solo para los límites nativos afectados. Aísla las pruebas de hosts, credenciales y cuentas de producción. Una muestra de navegador exitosa demuestra su propio límite, no todas las integraciones inferiores.

## Elegir una suite dirigida

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

La suite rápida combina unidades Bun y Vitest de aplicación/widgets. El ejecutor dirigido valida argumentos y rechaza filtros Rust sin coincidencias en lugar de aceptar cero pruebas. Usa el acceso `rust` cuando necesites banderas Cargo fuera de la gramática dirigida.

## Verificar el límite real

Las pruebas Vue utilizan el cliente de control falso compartido. Playwright principal ejecuta la misma SPA contra un host `window.ipc` falso instalado antes de navegar. Las llamadas RPC desconocidas fallan y se comprueban errores de consola/página. Ejecuta Chromium, no Wry. Las suites de interfaz de plugins viven en su repositorio externo. Las pruebas de gateway de widgets verifican cliente, protocolo y recursos compilados.

Para trabajo de navegador autorizado, tras compilar recursos utiliza `bun scripts/task.ts test e2e`. Lanza un navegador y por eso se separa de la edición documental y revisión de código sin interfaz. Shell nativo, OBS Browser Source, teléfono, almacén seguro del sistema, sustitución del actualizador y dos equipos necesitan sus entornos reales y evidencia explícita.

## Resultado esperado y estado

El informe identifica suite aprobada y alcance. Los cambios de implementación requieren lint, tipos, compilación y puertas CI correspondientes; el contenido usa `bun scripts/task.ts docs check` y `docs test`. No cambies dependencias para que una prueba descargue herramientas más recientes.

## Recuperación

Si un filtro no encuentra nada, corrígelo sin declarar éxito. Si una muestra carece de un RPC nuevo, amplía su contrato explícito en lugar de aceptar cualquier llamada. Mantén visibles las verificaciones manuales inciertas u omitidas. El [ejecutor de pruebas](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts) define la sintaxis vigente; [rendimiento](performance.md) explica evidencia reproducible bajo carga.
