---
title: "Adaptar recetas de automatización"
docId: "user.automations.recipes"
translationKey: "user.automations.recipes"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a05a9d97ff68ec9ba2e83ab0ebecc3bea274062b1989d8e6967952a00d308dd3"
translationRevision: "a05a9d97ff68ec9ba2e83ab0ebecc3bea274062b1989d8e6967952a00d308dd3"
order: 30
---

# Adaptar recetas de automatización

## Objetivo y requisitos

Convierte una [regla de registro](first-rule.md) funcional en reacciones con condiciones concretas. Necesitas un perfil activo y campos de eventos observados. Comienza con acciones `core.log`; sustituye su efecto únicamente después de confirmar coincidencias y frecuencia.

## Reconocer un comando de chat

Crea un evento `tiktok.chat` con `event.data.comment` igual (`eq`) a `!hello`. Registra:

```text
Hola {{ event.user.uniqueId }}
```

Usa una espera por usuario de `30` segundos: limita la repetición de un espectador y permite que otro tenga su oportunidad independiente. La igualdad exacta resulta útil para comandos; la comparación de contenido también acepta mensajes con texto adicional. Elige según tu intención.

## Registrar un regalo que cumpla el umbral

Crea un evento `tiktok.gift` con `event.data.diamondCount` mayor o igual (`gte`) a `10`. Registra:

```text
Regalo {{ event.data.giftName }} de {{ event.user.uniqueId }}
```

Inspecciona primero los detalles reales. El valor en diamantes, las repeticiones y el indicador de finalización son campos distintos. Si necesitas reaccionar solo al finalizar, comprueba que llega `event.data.repeatEnd` y añade la condición booleana verdadera. Un filtro por nombre no garantiza una ejecución por combo terminado.

## Limitar una reacción compartida

Crea un evento `tiktok.follow` con espera global de `10` segundos. Vincula una acción de registro breve. Si dos espectadores siguen durante ese intervalo, la espera global puede omitir la segunda reacción. Usa el ámbito por usuario si cada espectador necesita un límite independiente.

Para varias acciones, ordena la lista y conserva el modo de todas las acciones cuando necesites cada paso. El modo aleatorio elige una acción; no ejecuta todos los pasos en orden aleatorio. Añade `core.delay` únicamente cuando los pasos posteriores deban esperar.

## Resultado esperado y estado

Estos disparadores, filtros y tiempos de espera están disponibles en el motor local. Una ejecución debe corresponder a un evento aceptado y coincidente, sujeto a la espera y los interruptores. La aceptación del proveedor todavía requiere una comprobación LIVE real.

## Recuperación

Si falla una receta, vuelve a una sola acción de registro, inspecciona los datos y simplifica temporalmente las condiciones. Evita quitar los límites durante una emisión concurrida. Las pruebas de audio, puntos, HTTP y plugins pueden producir efectos reales. Consulta [audio](../plugins/audio.md), [variables](variables.md) y [diagnósticos](../preferences/troubleshooting.md) antes de añadir esas dependencias.
