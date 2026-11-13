---
title: "Diagnosticar una operación fallida"
docId: "user.preferences.troubleshooting"
translationKey: "user.preferences.troubleshooting"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
translationRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
order: 30
---

# Diagnosticar una operación fallida {#diagnose-a-failed-operation}

## Objetivo y requisitos {#goal-and-prerequisites}

Identifica el componente que falla y reúne un informe útil sin repetir efectos continuamente. Necesitas la versión, la hora aproximada y la operación intentada. Conserva una [copia](backups.md) antes de reiniciar o sustituir datos.

## Inspeccionar la evidencia cercana {#inspect-the-nearest-evidence}

1. Lee el error mostrado donde ocurrió el fallo. Usa su acción de diagnóstico o abre **Diagnostics** para inspeccionar las entradas relacionadas.
2. Filtra por origen, nivel o texto. Entre los orígenes útiles están LIVE, supervisión, plugins, cuenta, sincronización, actualizaciones, transporte, catálogo, instalación y colaboración.
3. Selecciona la entrada pertinente y conserva hora, código, identificadores de solicitud/entidad y descripción breve. Despliega los detalles disponibles.
4. Copia los diagnósticos filtrados o exporta el informe de texto. Si falla el portapapeles o la exportación, utiliza el texto alternativo mostrado. Revísalo antes de compartir y elimina tokens, cookies, enlaces de invitación y contenido privado.
5. Incluye una reproducción breve: página, creador/perfil/plugin seleccionado, acción, resultado esperado y resultado real. Indica si ocurrió antes o después de actualizar.

## Aislar fallos comunes {#isolate-common-failures}

Si el feed está vacío, borra búsqueda y filtros, reanuda el desplazamiento y confirma la [conexión](../live/connections.md). Pausar la vista no desconecta. Un aviso de pérdida visual no demuestra que puntos o estadísticas perdieran el mismo evento.

Si no se ejecuta una automatización, comprueba perfil activo, interruptores de evento/acción, disparador, rutas de campos y espera. Inspecciona el historial antes de modificar efectos. **Test** puede ejecutar audio, HTTP, puntos y acciones de plugins.

Para plugins, lee el error del trabajo de instalación o de conexión. Para Cloud, revisa la sesión y el área afectada en lugar de cambiar una cookie de TikTok. Para superposiciones, inspecciona pasarela y fuente. Para actualizaciones, consulta la [vía manual](../updates.md) cuando no se pueda instalar.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles filtros, copia y exportación de diagnósticos. Un paquete también requiere sus recursos web y el entorno de plataforma. En Linux, verifica GTK/WebKitGTK y una sesión X11/XWayland compatible; en Windows, WebView2. Consulta el alcance de [requisitos](../requirements.md).

## Recuperación {#recovery}

Repite después de corregir la causa concreta. Ante conflictos de revisión, carga y revisa el estado guardado actual. Si una orden remota queda sin confirmar, inspecciona su historial/estado antes de enviarla otra vez: el efecto puede haberse producido pese al tiempo de espera. Si falla el inicio, informa de paquete, versión y registro; evita borrar el directorio de datos.

<DemoFrame scenario="diagnostics" locale="es" tour="diagnostics" title="Demo de diagnostics" />
