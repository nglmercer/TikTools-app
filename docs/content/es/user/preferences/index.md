---
title: "Encontrar preferencias y controles de teclado"
docId: "user.preferences.index"
translationKey: "user.preferences.index"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6790ab7171a90bda1df882c0f88626a5875da68bf4f624968b0e54815842030f"
translationRevision: "557ac7d37d8f3a516e9b48f5722120d36bfdb33c1aab8b8fce56191e4cfc9ad3"
order: 10
---

# Encontrar preferencias y controles de teclado {#find-preferences-and-keyboard-controls}

## Objetivo y requisitos {#goal-and-prerequisites}

Configura el idioma, localiza el área de ajustes correcta y utiliza cómodamente los controles de teclado admitidos. Necesitas la ventana de escritorio abierta; Internet solo es necesario para el servicio que estés configurando.

## Localizar cada ajuste {#find-the-relevant-setting}

1. Abre **Settings → General** para elegir el idioma de interfaz. Las opciones actuales son inglés (`en`), español (`es`), portugués (`pt`), francés (`fr`), chino simplificado (`zh`) y japonés (`ja`). Esta selección no modifica identificadores de eventos de TikTok ni nombres de API de plugins.
2. Revisa los controles de actualización de General: canal, comprobaciones automáticas, notificaciones y consulta manual cuando sea compatible. Las comprobaciones automáticas son opcionales. La disponibilidad de actualizaciones firmadas depende de la configuración de firma publicada con la compilación.
3. Usa **Account** para identidad de Cloud y diagnósticos. **Collaboration** configura acceso del propietario; **Teams** aparece cuando el host anuncia esa capacidad.
4. Abre **Plugins** para preferencias generales de plugins y los paneles Settings/Connection de cada uno. Sus credenciales de servicio no pertenecen a los ajustes generales.
5. Abre **Automations → Variables** para valores compartidos, **Points** para reglas de obtención y **Overlays** para diseños. Estas superficies están junto a las funciones que configuran.

La instalación automática es experimental y está desactivada por los controles de compilación del flujo de publicación actual. La opción de consultas automáticas no promete instalar automáticamente. Sigue el [proceso de actualización](../updates.md) y crea una [copia](backups.md) antes de cambiar de versión.

## Utilizar el teclado {#use-supported-keyboard-controls}

Tab y Shift+Tab mueven el foco entre controles ordinarios; Enter/Espacio activan botones enfocados. En el inspector de diseños, Ctrl/Meta+Z deshace y Ctrl/Meta+Shift+Z o Ctrl/Meta+Y rehace. Los campos de texto conservan su propio deshacer. Son atajos del editor, no disparadores globales de automatización.

Escape cierra el menú lateral estrecho si otro control no lo ha consumido. Los diálogos y editores pueden manejarlo primero, incluida una confirmación por cambios pendientes. Las automatizaciones con teclas globales requieren un plugin compatible y las capacidades anunciadas por su entorno.

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

Estos ajustes e interacciones locales están disponibles; no se acredita aceptación completa de accesibilidad ni de todas las plataformas. Prefiere botones etiquetados, tamaños legibles y pausar el feed si su movimiento distrae. Si falta un ajuste, comprueba su página y la capacidad del host antes de borrar datos. Consulta [solución de problemas](troubleshooting.md) y las [preguntas frecuentes](faq.md).

<DemoFrame scenario="settings" locale="es" title="Demo de settings" />
