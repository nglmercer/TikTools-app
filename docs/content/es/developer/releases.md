---
title: "Versiones, paquetes y puertas del actualizador"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7b1c1c12ef3768dd765301e04b7894670570ae78e72a446ace0b405da157c39a"
translationRevision: "7b1c1c12ef3768dd765301e04b7894670570ae78e72a446ace0b405da157c39a"
order: 220
---

# Versiones, paquetes y puertas del actualizador

## Objetivo y requisitos

Prepara artefactos revisables y distingue descargas verificadas de instalación automática. Este flujo de mantenimiento requiere herramientas de plataforma y material de firma protegido. Los colaboradores pueden inspeccionar empaquetado y versiones sin publicar ni crear etiquetas.

## Preparar artefactos coherentes

`package.json`, `Cargo.toml` del workspace y `Cargo.lock` comparten versión. Usa el ejecutor en vez de modificar una sola por separado:

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

Un cambio intencional de mantenimiento utiliza `bun scripts/task.ts version bump v4.2.3` con la versión elegida. La compilación completa también genera widgets. El empaquetado coloca el ejecutable junto a `web/index.html` y publica sumas de comprobación. Los flujos de instalación generan además NSIS, DEB y AppImage y comparan sus recursos con la preparación portable. Los plugins siguen siendo instalaciones independientes.

## Revisar la confianza del actualizador

`tiktools-app-update` utiliza metadatos de canal y paquetes firmados. Clave pública confiable, activación del feed y recursos publicados/verificados son requisitos separados. Las sumas SHA256 por sí solas no habilitan el actualizador. No reutilices claves de editor de plugins para actualizaciones de aplicación.

El host limita descargas y redirecciones; la entrega al instalador espera un cierre seguro de núcleo y WebView. AppImage exige además `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` al compilar. NSIS requiere su puerta `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. La existencia del código o una prueba de firma no justifica activarlas en distribuciones.

## Resultado esperado y estado

Empaquetado portable y herramientas de instalación están implementados. Comprobaciones/descargas dependen del aprovisionamiento; reemplazo y reinicio automáticos siguen experimentales. Linux extraído o desconocido usa instalación manual. Iniciar NSIS no permite que el proceso antiguo, ya cerrado, declare éxito del instalador.

## Verificación y recuperación

Ejecuta `bun scripts/task.ts rust test -p tiktools-app-update --locked` para muestras locales de protocolo/firma. La aceptación de actualización, fallos, conservación de perfil y reinicio necesita instalaciones compatibles desechables. Conserva el artefacto previo verificado; publicar parcialmente feed/firma debe fallar de forma cerrada. No actives la entrega si el cierre de plugins permanece incierto. El [flujo de versiones](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) y el código poseen las puertas actuales; los planes históricos no son instrucciones de despliegue.
