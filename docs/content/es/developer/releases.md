---
title: "Versiones, paquetes y puertas del actualizador"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a0c1a45f5279fec0b2fae29f40f47090d0600237815b293e9600e28221ec969b"
translationRevision: "4e57958118e2b25139828d748812d10f37b19809958071dbcdaf40d8b3e7660f"
order: 220
---

# Versiones, paquetes y puertas del actualizador {#releases-packages-and-updater-gates}

## Objetivo y requisitos {#goal-and-prerequisites}

Prepara artefactos revisables y distingue descargas verificadas de instalación automática. Este flujo de mantenimiento requiere herramientas de plataforma y material de firma protegido. Los colaboradores pueden inspeccionar empaquetado y versiones sin publicar ni crear etiquetas.

## Preparar artefactos coherentes {#prepare-consistent-artifacts}

`package.json`, `Cargo.toml` del workspace y `Cargo.lock` comparten versión. Usa el ejecutor en vez de modificar una sola por separado:

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

Un cambio intencional de mantenimiento utiliza `bun scripts/task.ts version bump v4.2.3` con la versión elegida. La compilación completa también genera widgets. El empaquetado coloca el ejecutable junto a `web/index.html` y publica sumas de comprobación. Los flujos de instalación generan además NSIS, DEB y AppImage y comparan sus recursos con la preparación portable. Los plugins siguen siendo instalaciones independientes.

Las tarjetas de versiones de la documentación utilizan una instantánea de las publicaciones de GitHub. Actualízala explícitamente cuando quieras incorporar publicaciones nuevas:

```sh
bun scripts/refresh-docs-releases.ts
```

Este paso consulta GitHub y conserva únicamente publicaciones y archivos publicados cuya carga haya terminado. Revisa los datos resultantes antes de incluirlos en el sitio. La compilación consume la instantánea local y funciona sin consultar GitHub; las tarjetas muestran su fecha de consulta y enlazan las publicaciones actuales para que el lector compruebe cambios posteriores. Una etiqueta de código sin instaladores publicados no debe convertirse en un botón de descarga inventado.

## Revisar la confianza del actualizador {#review-updater-trust}

`tiktools-app-update` utiliza metadatos de canal y paquetes firmados. Clave pública confiable, activación del feed y recursos publicados/verificados son requisitos separados. Las sumas SHA256 por sí solas no habilitan el actualizador. No reutilices claves de editor de plugins para actualizaciones de aplicación.

El host limita descargas y redirecciones; la entrega al instalador espera un cierre seguro de núcleo y WebView. AppImage exige además `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` al compilar. NSIS requiere su puerta `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. La existencia del código o una prueba de firma no justifica activarlas en distribuciones.

## Resultado esperado y estado {#expected-result-and-status}

Empaquetado portable y herramientas de instalación están implementados. Comprobaciones/descargas dependen del aprovisionamiento; reemplazo y reinicio automáticos siguen experimentales. Linux extraído o desconocido usa instalación manual. Iniciar NSIS no permite que el proceso antiguo, ya cerrado, declare éxito del instalador.

## Verificación y recuperación {#verify-and-recover}

Ejecuta `bun scripts/task.ts rust test -p tiktools-app-update --locked` para muestras locales de protocolo/firma. La aceptación de actualización, fallos, conservación de perfil y reinicio necesita instalaciones compatibles desechables. Conserva el artefacto previo verificado; publicar parcialmente feed/firma debe fallar de forma cerrada. No actives la entrega si el cierre de plugins permanece incierto. El [flujo de versiones](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) y el código poseen las puertas actuales; los planes históricos no son instrucciones de despliegue.

## Detalles de operación {#operational-details}

Empaquetado portable local exige versiones coincidentes de fuente/manifiestos/binario, widgets/web construidos y ejecutable release. El ejemplo usa versión actual; cambia tag solo con cambio intencional. `package` valida binario/layout y escribe `release/TikTools-<version>-<platform>.tar.gz` (Linux) o `.zip` (Windows). `RELEASE_BINARY` elige otro binario. En Windows usa asignaciones PowerShell y `windows-x86_64`. Instaladores requieren dependencias y cargo-packager fijado del workflow; después `bun scripts/package-installers.ts` con mismo tag/plataforma. Portable no produce NSIS/DEB/AppImage.

Conjunción compilada: checks exigen `TIKTOOLS_APP_UPDATE_FEED_ENABLED=1` Y `TIKTOOLS_APP_UPDATE_PUBLIC_KEY` válida; AppImage también `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` Y origen compatible detectado; NSIS además `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1`. Son valores de compilación. El workflow transforma variable de repositorio `TIKTOOLS_UPDATE_PUBLIC_KEY` en clave/feed. Sin ella puede publicar assets/checksums sin feed firmado. Con ella exige clave/password protegidas, firma de manifiesto exacto y paquete, assets publicados verificados y promoción autenticada. Firma updater es distinta de firma OS y editor de plugins.

Tests predeterminados omiten aceptación AppImage real. Dos tests ignorados necesitan fixture AppImage real descartable que informe versión 4.0.0 (aserción actual), FUSE y signer cargo-packager; reemplazo necesita pantalla. Configura `TIKTOOLS_TEST_APPIMAGE`, lista tests y ejecuta expresamente `preflight_runs_a_real_appimage` y `packager_replaces_a_running_disposable_appimage_without_touching_its_profile` con `-- --ignored --nocapture`. Árbol extraído no sustituye montaje. Observa reemplazo, versión, reinicio y perfil. NSIS exige aceptación posterior independiente; proceso antiguo no certifica éxito.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
RELEASE_TAG=v4.2.2 RELEASE_PLATFORM=linux-x86_64 bun scripts/task.ts package
```

```powershell
$env:RELEASE_TAG = 'v4.2.2'
$env:RELEASE_PLATFORM = 'windows-x86_64'
bun scripts/task.ts package
```

```sh
TIKTOOLS_TEST_APPIMAGE=/absolute/path/disposable-v4.0.0.AppImage bun scripts/task.ts rust test -p tiktools-app-update --locked preflight_runs_a_real_appimage -- --ignored --nocapture
```
