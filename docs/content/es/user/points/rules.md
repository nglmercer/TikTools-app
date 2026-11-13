---
title: "Configurar reglas de puntos"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b7e9f9456c4e6f5abbdbb3858192c39593e5e7393972bab1d2ca2457d63537e6"
translationRevision: "b7e9f9456c4e6f5abbdbb3858192c39593e5e7393972bab1d2ca2457d63537e6"
order: 10
---

# Configurar reglas de puntos

## Objetivo y requisitos

Decide cómo la actividad entrante genera tu moneda local y verifica una asignación sencilla. Necesitas TikTools abierto y un creador conocido para la comprobación LIVE posterior. Las reglas de puntos y las acciones de automatización son independientes: una regla puede asignar puntos aunque no hayas creado automatizaciones.

## Definir cómo se obtienen puntos

1. Abre **Points** y localiza el panel de reglas. Elige un nombre reconocible para la moneda, por ejemplo `Estrellas`.
2. Activa únicamente las fuentes deseadas: monedas de regalos, chat, likes, compartidos, seguidores o entradas. Cada fuente tiene su propia tasa e interruptor.
3. Para una primera comprobación clara, activa chat con `1` punto y desactiva las demás fuentes. Deja la bonificación de suscriptor en `0` mientras estableces la referencia.
4. Configura los puntos por nivel, por ejemplo `100`. El editor exige un mínimo de `10`; las tasas no admiten valores negativos.
5. Pulsa **Save** y espera la confirmación. **Discard** recupera la configuración guardada mientras estás editando.
6. Conecta desde [Connections](../live/connections.md), observa un mensaje nuevo en el [feed](../live/feed.md), actualiza la tabla y busca a ese espectador.

Las tasas de regalos y likes utilizan las cantidades de contribución recibidas. Por ejemplo, una contribución de `10` likes con una tasa de `0.1` produce `1` punto antes de una bonificación aplicable. Una fila del feed no equivale necesariamente a una unidad de contribución. Cambiar las tasas establece la política de asignaciones posteriores; no esperes que recalcule el pasado.

## Resultado esperado y estado

Están disponibles las reglas locales, los saldos y los controles manuales de añadir o descontar. Con la configuración aislada de chat, un mensaje nuevo aceptado debería aumentar en uno el saldo del espectador. Los niveles siguen el valor de puntos por nivel. Son puntos de la aplicación, sin equivalencia prometida con moneda de TikTok ni pagos.

## Recuperación

Comprueba primero el interruptor de la fuente, la tasa guardada y el identificador del espectador. Una automatización que también asigne puntos puede sumar otra asignación. Los reinicios se bloquean durante conexiones LIVE activas o en curso; desconecta antes de reiniciar y lee la confirmación. Conserva una [copia con la aplicación cerrada](../preferences/backups.md) antes de borrar saldos.

<DemoFrame scenario="points" locale="es" tour="points" title="Demo de points" />
