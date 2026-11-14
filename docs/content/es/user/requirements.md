---
title: "Requisitos"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c68c149552a943cf0f03393c5dac91bd62abf9c50f791d7b0757599754f4804e"
translationRevision: "1664c2a41360e5483b50977d7e29bcce1c4d590447196b2a0cf0c394ab9f2ef8"
order: 10
---

# Requisitos {#requirements}

## Objetivo {#goal}

Elegir un paquete de escritorio compatible e identificar los servicios necesarios antes de instalar.

## Qué necesitas {#what-you-need}

- Un equipo Windows x86_64 o Linux x86_64 para los paquetes de escritorio publicados. El workflow actual compila esos dos destinos. No publica escritorio para macOS ni un APK de Android.
- Una sesión gráfica funcional. Windows necesita el runtime WebView2 de Microsoft. Linux necesita un entorno GTK/WebKitGTK compatible; el DEB declara sus paquetes de ejecución y el AppImage incluye los auxiliares multimedia revisados.
- Acceso de red a TikTok para una conexión LIVE real. Una sala sin transmisión, una sesión de invitado caducada o un límite del servicio no se corrigen cambiando una automatización.
- Espacio fuera de la instalación para SQLite, plugins y registros. La aplicación utiliza un directorio de datos por usuario; mover el ejecutable no mueve esos datos.

La cuenta Cloud es necesaria para operaciones de catálogo o colaboración basadas en cuenta, no para todos los editores locales. OBS sólo es necesario para mostrar widgets en una transmisión. Las integraciones de audio/TTS pueden requerir un plugin separado y su propio proveedor o servicio.

## Comprueba el entorno {#check-the-environment}

1. Elige el paquete de tu sistema operativo y arquitectura x86_64.
2. En Windows, instala WebView2 si el instalador indica que falta. En Linux, resuelve bibliotecas de ejecución ausentes con el gestor de paquetes de tu distribución.
3. Comprueba que la ventana abre antes de conectar con TikTok o añadir plugins.
4. Continúa con la [instalación](installation.md) y el [primer LIVE](first-live.md).

## Resultado esperado {#expected-result}

La aplicación abre y su navegación principal funciona sin conexión. Las integraciones opcionales pueden permanecer deshabilitadas hasta que exista su plugin o capacidad del host.

## Recuperación {#recovery}

Ante una ventana vacía o un cierre temprano, recopila los diagnósticos de inicio y la versión exacta del paquete. AppImage y DEB son formatos distintos de instalación; ninguno se ejecuta en Android. Consulta el [alcance de Termux y del modo sin interfaz](termux.md). La compilación desde código tiene requisitos adicionales en Desarrollo.

## Detalles de operación {#operational-details}

Si no aparece la ventana, busca el log nativo `tiktools.log` (`controller.log` en modo controlador): Windows `%LOCALAPPDATA%/TikTools/logs`, Linux `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs`, salvo cambios de `TIKTOOLS_HOME` o `TIKTOOLS_LOG_DIR`. El registro puede fallar antes de crear archivo. Conserva el error de terminal disponible y revisa [retención y privacidad](preferences/troubleshooting.md#operational-details) antes de compartir.
