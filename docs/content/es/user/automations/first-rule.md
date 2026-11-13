---
title: "Crear tu primera regla de registro"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
translationRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
order: 20
---

# Crear tu primera regla de registro {#create-your-first-logging-rule}

## Objetivo y requisitos {#goal-and-prerequisites}

Verifica el recorrido de evento a acción con una línea de registro local. Necesitas el host de escritorio abierto y acceso a **Automations**. No hacen falta cuenta de Cloud, servicio de audio ni destino externo. Usa texto ficticio o no sensible: el registro conserva el mensaje resultante.

## Crear la acción {#create-the-action}

1. Abre **Automations → Actions** en el perfil que quieras utilizar.
2. Crea una acción de tipo **Write to the log** (`core.log`) llamada `Comprobar chat`.
3. Establece esta plantilla en Message, guarda y activa la acción:

```text
Comprobar chat: {{ event.user.uniqueId }} dijo {{ event.data.comment }}
```

Estas son las rutas normalizadas actuales del chat. `event.data.comment` contiene el mensaje; `event.user.uniqueId`, el identificador del espectador. El nombre visible utiliza otro campo.

## Crear el evento {#create-the-event}

1. Abre **Events**, crea `Evento comprobar chat` y selecciona el disparador `tiktok.chat`.
2. Vincula únicamente `Comprobar chat`. Conserva el modo normal de ejecutar todas las acciones.
3. Deja los filtros vacíos para la primera prueba de muestra. Configura una espera global de `10` segundos para limitar después el tráfico real.
4. Guarda y revisa cualquier vista previa de cambios. Con la acción de registro como único vínculo, pulsa **Test** e inspecciona el resultado y su texto. El host puede aportar una muestra; ese resultado no demuestra conectividad real con TikTok.
5. Añade una condición sobre `event.data.comment`, operador igual (`eq`), valor `!hello`. Guarda otra vez y activa explícitamente el evento.
6. Conecta a un [LIVE](../live/connections.md) donde tengas autorización para probar. Observa un mensaje nuevo `!hello` en el [feed](../live/feed.md) y consulta el historial cuando la espera permita ejecutarlo.

## Resultado esperado y estado {#expected-result-and-status}

La acción local de registro y el disparador de chat están disponibles. La muestra debería producir una línea legible; un chat real coincidente, una ejecución con el identificador y mensaje recibidos. Otro texto no debería superar la condición exacta.

## Recuperación {#recovery}

Si la muestra deja de coincidir tras añadir el filtro, su comentario puede ser distinto de `!hello`. Comprueba campos reales, perfil e interruptores antes de repetir. Las pruebas pueden ejecutar efectos; esta regla solo escribe un registro, por lo que su efecto es previsible. Desactívala al terminar o continúa con las [recetas](recipes.md).

<DemoFrame scenario="automation" locale="es" tour="automation" title="Demo de automation" />
