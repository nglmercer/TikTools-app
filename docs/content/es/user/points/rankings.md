---
title: "Entender rankings y competiciones"
docId: "user.points.rankings"
translationKey: "user.points.rankings"
locale: "es"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d2c0041373921fc8f25f11ee7bdd713daed7c8d2af41b57c5f14324dfaf22a96"
translationRevision: "d2c0041373921fc8f25f11ee7bdd713daed7c8d2af41b57c5f14324dfaf22a96"
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
