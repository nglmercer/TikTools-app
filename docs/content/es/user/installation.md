---
title: "Instalar en Windows o Linux"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6f8673490bebb3447474a70d914d06102de87a9ef2f3f3c51da897d0d2cc8c1c"
translationRevision: "600e415b4e68fb14a427711469f321a76ba73aad666f9a7485ac21fed1113fe2"
order: 20
---

# Instalar en Windows o Linux {#install-windows-or-linux}

## Objetivo y requisitos {#goal-and-prerequisites}

Instalar un paquete de escritorio publicado sin preparar compiladores. Revisa los [requisitos](requirements.md), cierra las instancias abiertas de TikTools y descarga desde las [versiones de GitHub](https://github.com/nglmercer/TikTools-app/releases).

## Elige un paquete {#choose-a-package}

| Paquete | Uso |
| --- | --- |
| Windows `installer.exe` | Instalación NSIS por usuario; necesita WebView2. |
| Linux `installer.AppImage` | Imagen ejecutable portátil para un escritorio Linux compatible. |
| Linux `installer.deb` | Instalación mediante el gestor de paquetes Debian/Ubuntu. |
| ZIP de Windows / TAR.GZ de Linux | Directorio portátil con el ejecutable y sus recursos. |

Comprueba si la versión es estable o preliminar. Los prereleases son candidatos de depuración, no una garantía automática de compatibilidad en producción. Verifica los archivos descargados con el `SHA256SUMS.txt` de la versión antes de sustituir una instalación.

## Windows {#windows}

1. Descarga el instalador x86_64 y verifica su checksum.
2. Ejecuta el instalador con la cuenta que utilizará TikTools. Instala WebView2 si se solicita.
3. Abre TikTools. Entra en **Settings → General** para elegir el idioma.
4. Si usas un ZIP portátil, extrae todo el directorio. Conserva `web/` y los demás recursos junto al ejecutable; no ejecutes un EXE copiado por separado.

## Linux {#linux}

1. Descarga el AppImage, DEB o archivo portátil correspondiente y verifica su checksum.
2. Para AppImage, permite su ejecución desde el gestor de archivos o aplica `chmod +x` al archivo descargado y ábrelo.
3. Para DEB, utiliza el gestor de paquetes del sistema para resolver las dependencias de ejecución.
4. Para TAR.GZ, extrae el directorio completo y ejecuta el escritorio conservando los recursos empaquetados.

## Resultado esperado {#expected-result}

TikTools abre sin conexión. Sus datos se guardan fuera de la instalación, normalmente en `%LOCALAPPDATA%/TikTools` en Windows o `~/.local/share/TikTools` en Linux.

## Recuperación {#recovery}

Si el paquete falla antes de mostrar una ventana, revisa los diagnósticos y los errores de bibliotecas antes de modificar cuentas o migraciones. Antes de probar otra versión, cierra la aplicación y respalda el directorio de datos. Reinstalar un ejecutable antiguo no revierte migraciones de base de datos. Continúa con el [primer LIVE](first-live.md) o las [actualizaciones](updates.md).

## Detalles de operación {#operational-details}

Si no aparece la ventana, busca el log nativo `tiktools.log` (`controller.log` en modo controlador): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, salvo cambios de `TIKTOOLS_HOME` o `TIKTOOLS_LOG_DIR`. El registro puede fallar antes de crear archivo. Conserva el error de terminal disponible y revisa [retención y privacidad](preferences/troubleshooting.md#operational-details) antes de compartir.
