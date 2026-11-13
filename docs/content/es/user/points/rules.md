---
title: "Configurar reglas de puntos"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "89e0189daf4193853d17ec9e576a4ff9973cdfc5df06d0b27fc59296d9ee66de"
translationRevision: "19492ea15ca52a025bf7120e4dbc719d64f6e86f23aed323a802d6cad3f045fe"
order: 10
---

# Configurar reglas de puntos {#configure-point-rules}

## Objetivo y requisitos {#goal-and-prerequisites}

Decide cómo la actividad entrante genera tu moneda local y verifica una asignación sencilla. Necesitas TikTools abierto y un creador conocido para la comprobación LIVE posterior. Las reglas de puntos y las acciones de automatización son independientes: una regla puede asignar puntos aunque no hayas creado automatizaciones.

<DocCallout title="Una fuente para la primera comprobación" tone="tip">

Empieza con un punto por mensaje de chat y el resto de fuentes desactivadas. Así puedes comparar un evento nuevo con el cambio de saldo de un único espectador.

</DocCallout>

## Definir cómo se obtienen puntos {#set-the-earning-policy}

<DocSteps>

1. **Nombra tu moneda.** Abre **Puntos**, localiza el panel de reglas y elige un nombre reconocible, por ejemplo `Estrellas`.
2. **Selecciona las fuentes.** Puedes asignar puntos por monedas de regalos, chat, likes, compartidos, seguidores o entradas. Cada fuente tiene su propia tasa e interruptor.
3. **Aísla el primer ejemplo.** Activa chat con `1` punto y desactiva las demás fuentes. Deja la bonificación de suscriptor en `0` mientras estableces la referencia.
4. **Define los niveles.** Configura, por ejemplo, `100` puntos por nivel. El editor exige un mínimo de `10`; las tasas no admiten valores negativos.
5. **Guarda la política.** Pulsa **Guardar reglas** y espera la confirmación. **Descartar** recupera la configuración guardada mientras estás editando.
6. **Verifica una asignación real.** Conecta desde [Conexiones](../live/connections.md), observa un mensaje nuevo en el [feed](../live/feed.md), actualiza la tabla y busca a ese espectador.

</DocSteps>

<DemoFrame scenario="points" locale="es" panel="points" title="Explora las reglas y los saldos de puntos" />

Las tasas de regalos y likes utilizan las cantidades de contribución recibidas. Por ejemplo, una contribución de `10` likes con una tasa de `0.1` produce `1` punto antes de una bonificación aplicable. Una fila del feed no equivale necesariamente a una unidad de contribución. Cambiar las tasas establece la política de asignaciones posteriores; no esperes que recalcule el pasado.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles las reglas locales, los saldos y los controles manuales de añadir o descontar. Con la configuración aislada de chat, un mensaje nuevo aceptado debería aumentar en uno el saldo del espectador. Los niveles siguen el valor de puntos por nivel. Son puntos de la aplicación, sin equivalencia prometida con moneda de TikTok ni pagos.

Por ejemplo, si un espectador tiene `20` puntos y llega un chat nuevo aceptado con la regla anterior, el saldo esperado es `21`. Comprueba el resultado en la tabla después del evento; cambiar la tasa no modifica las asignaciones anteriores.

## Recuperación {#recovery}

Comprueba primero el interruptor de la fuente, la tasa guardada y el identificador del espectador. Una automatización que también asigne puntos puede sumar otra asignación. Los reinicios se bloquean durante conexiones LIVE activas o en curso; desconecta antes de reiniciar y lee la confirmación. Conserva una [copia con la aplicación cerrada](../preferences/backups.md) antes de borrar saldos.

## Detalles de operación {#operational-details}

El menú de una fila reinicia un espectador a 0 puntos y nivel 1 y añade un marcador.

<DocCallout title="Alcance de Reiniciar todos" tone="warning">

**Reiniciar todos** afecta a todos los espectadores guardados en esta instalación, sin limitarse al creador: pone los saldos a 0 y niveles a 1 y elimina el historial de transacciones de puntos. Conserva los registros de espectadores y sus contadores acumulados. No limpia el feed ni **Analíticas**. Desconecta el LIVE, guarda una copia con la aplicación cerrada y revisa el alcance antes de confirmar.

</DocCallout>

Los ajustes manuales positivos aplican el bonus de suscriptor guardado: +100 con bonus del 50% entrega +150 a un suscriptor. Descontar 100 no aplica ese bonus; el saldo mínimo es cero. Comprueba el saldo final, sin asumir que el delta coincide siempre con lo escrito. La tabla solo busca en su instantánea de los 100 primeros; no encontrar un nombre no demuestra que no tenga registro.
