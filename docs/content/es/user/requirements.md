---
title: "Requisitos"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
translationRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
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
