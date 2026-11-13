---
title: "Leer e inspeccionar el feed LIVE"
docId: "user.live.feed"
translationKey: "user.live.feed"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a72d330eea30a08b1c1607a3d9fb363a7dbb4f788a677d294d7e215312615057"
translationRevision: "a72d330eea30a08b1c1607a3d9fb363a7dbb4f788a677d294d7e215312615057"
order: 20
---

# Leer e inspeccionar el feed LIVE

## Objetivo y requisitos

Encuentra un evento, inspecciona sus campos y distingue una vista pausada de una sala desconectada. Comienza con una [conexión LIVE](connections.md) o utiliza los eventos que ya están en el feed actual. El feed es una vista de trabajo limitada, no una grabación completa de la transmisión.

## Inspeccionar la actividad

1. Abre **Live feed** y confirma el creador activo en el estado de conexión.
2. Elige un filtro: todos los eventos, chat, regalos, likes, seguidores, entradas u otra actividad social. Combínalo con la búsqueda para localizar un autor, mensaje, nombre de regalo o ID exacto del regalo.
3. Pulsa **Pause** para detener el desplazamiento automático mientras lees. Pueden seguir llegando eventos y aumentar el contador de pendientes. Este control no desconecta TikTok ni suspende automatizaciones.
4. Selecciona un evento para abrir su inspector. Lee el tipo y los detalles disponibles antes de elegir campos de automatización. No todos los eventos incluyen texto de chat, datos de regalos o identidad del espectador.
5. Selecciona el nombre o avatar de un espectador para revisar su perfil registrado. Usa el control del inspector si el panel de detalles está oculto.
6. Pulsa **Resume** o el botón de salto a eventos pendientes para volver a la actividad entrante. Borra la búsqueda y selecciona todos los eventos al comprobar si llega actividad.

Utiliza los controles etiquetados con el foco del teclado cuando sea posible. Pausar el desplazamiento ayuda cuando el movimiento dificulta leer. Si falta un avatar, se muestran iniciales; la imagen no es necesaria para identificar al espectador.

## Resultado esperado y estado

Los filtros y la inspección están disponibles. Con mucho tráfico, la interfaz puede limitar las filas dibujadas y avisar de eventos omitidos en la vista. Estos indicadores describen la presentación; no uses el número de filas visibles como total contable. [Estadísticas](../points/statistics.md) consulta agregados persistentes por separado.

## Recuperación

Si no ves eventos, elimina filtros, reanuda el desplazamiento y revisa la conexión. Una sala tranquila puede funcionar correctamente. Reconectar al mismo creador conserva el contexto; cambiarlo lo sustituye. **Clear** vacía el feed de trabajo y no reinicia puntos ni estadísticas. Investiga los avisos mediante [diagnósticos](../preferences/troubleshooting.md), sin asumir que cada fila ausente representa una pérdida del evento original.

<DemoFrame scenario="feed" locale="es" title="Demo de feed" />
