---
title: "Consultar estadísticas por fechas"
docId: "user.points.statistics"
translationKey: "user.points.statistics"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f225b60d175147b26bdcda6972e1c4a71bd122230ee7d4c58b337162f9672bb0"
translationRevision: "f225b60d175147b26bdcda6972e1c4a71bd122230ee7d4c58b337162f9672bb0"
order: 20
---

# Consultar estadísticas por fechas {#review-statistics-across-dates}

## Objetivo y requisitos {#goal-and-prerequisites}

Compara la actividad de un creador durante un período útil y comprende por qué sus totales difieren del feed visible o del saldo de puntos. Necesitas actividad registrada en la instalación local y el creador correcto seleccionado desde [Connections](../live/connections.md). El inicio de sesión en Cloud no proporciona estas estadísticas locales.

## Consultar el período registrado {#query-the-recorded-period}

1. Abre **Analytics** y confirma el creador junto a los controles de fecha. El creador seleccionado determina qué agregados guardados se consultan.
2. Elige **Today**, `7d`, `30d`, `90d` o un rango personalizado. Para fechas personalizadas, establece inicio y fin explícitamente y comprueba su orden.
3. Lee el intervalo mostrado y el número de sesiones antes de comparar resultados. Los límites diarios utilizan un desplazamiento horario; cambiar la zona del equipo puede modificar la interpretación del período.
4. Selecciona la métrica: chats, regalos, likes, diamantes o pico de espectadores. Usa los gráficos de actividad para comparar períodos y la tabla de contribuyentes para revisar los espectadores destacados devueltos.
5. Busca en la tabla un espectador incluido entre los datos recibidos. Esta búsqueda no recorre sin límites todos los espectadores observados alguna vez.
6. Pulsa **Refresh** después de nueva actividad o al volver a la página. Anota creador, fechas y métrica junto con cualquier cifra que compartas.

La página actual llama a la operación nativa `analytics.summary` con un creador y un intervalo. Lee agregados persistentes, incluidas sesiones registradas previamente; no se limita a los eventos que permanecen en el feed actual. La base de estadísticas guarda agregados y no una transcripción del chat.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles las estadísticas locales persistentes y las consultas por fechas. Es normal que períodos sin actividad devuelvan resultados vacíos. Los saldos pueden diferir porque las [tasas](rules.md), los ajustes manuales y las automatizaciones miden otra cosa. Diamantes, número de regalos y puntos no son unidades intercambiables.

## Recuperación {#recovery}

Si faltan datos, borra la búsqueda, amplía las fechas, confirma el creador y actualiza. Ante un error del host, conserva su código de diagnóstico y repite cuando se recupere la conexión. Vaciar el feed no borra estadísticas. Un directorio de datos nuevo no contiene historial anterior; consulta [copias de seguridad](../preferences/backups.md) para trasladar una instalación.

<DemoFrame scenario="analytics" locale="es" title="Demo de analytics" />
