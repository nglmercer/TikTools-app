---
title: "Entender rankings y competiciones"
docId: "user.points.rankings"
translationKey: "user.points.rankings"
locale: "es"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1c45cf6e9355092d95dc9f90025b2965e0a6a4cddb453fb25680192ab41e873c"
translationRevision: "ef1de0d2b918562f925dae4aed9094a146d2c1c8d657f0c168902a87d2031fa4"
order: 30
---

# Entender rankings y competiciones {#understand-rankings-and-competitions}

## Objetivo y requisitos {#goal-and-prerequisites}

Elige la clasificación adecuada y reconoce los límites del espacio de rankings en desarrollo. Necesitas actividad entrante para obtener puntuaciones útiles. Comienza con la tabla habitual de [Points](rules.md) o los paneles de espectadores destacados del [feed LIVE](../live/feed.md) si solo deseas consultar participantes actuales.

## Elegir la vista {#use-the-right-view}

1. En **Points**, busca un espectador y ordena por puntos, nivel o identidad. Actualiza antes de comparar un ajuste manual con una asignación.
2. En **Live feed**, revisa los paneles de espectadores clasificados disponibles. Pueden recibir instantáneas del motor de rankings independientemente de la tabla de saldos.
3. Para una superposición, abre [Overlays](../overlays/index.md), selecciona el tipo de ranking y revisa sus fuentes. La fuente configurada debe existir para que aparezcan resultados.

El espacio dedicado **Rankings & competitions** es experimental. Su ruta de frontend es `#/rankings`, pero está oculto deliberadamente en la navegación principal y no se recupera como pestaña de inicio. La ausencia en la barra lateral no significa que se hayan eliminado la ruta o el motor.

## Configurar un ranking experimental {#configure-an-experimental-ranking}

Si tu entorno permite acceder a ese espacio, comienza con un creador, una sala, agrupación por usuario y una ventana de sesión. Añade una métrica anunciada, como `tiktok.like` / `like.count`, con peso `1`. Establece un límite de resultados moderado y crea la definición. El formulario también admite ventanas móviles y diarias, con límites de duración y eventos o un desplazamiento horario explícito.

Lee la instantánea resultante antes de añadir fuentes. Pausar y reanudar afectan a la recopilación; reiniciar borra intencionalmente el estado del ranking. Las opciones de competición y las fuentes de votos tienen restricciones de agrupación propias. No supongas que reiniciar puntos reinicia una competición ni que toda agrupación personalizada admite cualquier métrica.

## Resultado esperado y estado {#expected-result-and-status}

Las tablas habituales están disponibles; la administración dedicada sigue siendo experimental. Las puntuaciones provienen del host, no de contar filas visibles. Esta guía no acredita aceptación real en emisión/OBS ni todas las combinaciones de competición.

## Recuperación {#recovery}

Si no hay puntuaciones, revisa creador, sala, fuente activa, ventana y pausa. Retira combinaciones incompatibles en lugar de inventar campos. Conserva datos mediante [copias de seguridad](../preferences/backups.md) antes de reiniciar e incluye el ID de definición y el código de diagnóstico al informar de un fallo.

## Detalles de operación {#operational-details}

La tabla de Points solicita 100 espectadores y filtra/ordena solo las filas recibidas. Refrescar u ordenar por nombre no consulta toda la base. Los rankings del feed muestran 10 filas por sesión de creador/sala: diamantes de regalos en Top y cantidades en Likes, con seguimiento limitado a 5.000 espectadores. No representan puntos guardados. Compara unidades y límites de sesión antes de comparar totales.

Pruébalo interactivamente antes de tocar tu configuración real:

<DemoFrame scenario="points" locale="es" panel="points" title="Prueba rankings y puntos de espectadores" />

## Operaciones de línea de comandos {#command-line-operations}

Cada operación de esta guía también está disponible desde la CLI: sin conexión, scriptable y apta para agentes. Las líneas de uso son sintaxis del parser y permanecen en inglés en todos los idiomas.

<!--@include: ../../../../.generated/cli-domain-ranking.es.md-->

<!--@include: ../../../../.generated/cli-domain-competition.es.md-->

Consulta la [referencia completa de comandos](../../developer/cli.md) para las convenciones, la salida JSON y los marcadores de operaciones destructivas.
