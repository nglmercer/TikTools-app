---
title: "Crear tu primera regla de registro"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "13b4b7b3cb8eba95257ec9738934813266414f504eec8b43090b73e74ad50c10"
translationRevision: "f3af951e93e22a65a789e0dba7088fbc65c1631697324f093687fec126ae47e1"
order: 20
---

# Crear tu primera regla de registro {#create-your-first-logging-rule}

## Objetivo y requisitos {#goal-and-prerequisites}

Aprende a convertir un mensaje de chat en una línea de registro local. Primero crearás la acción y después el evento que la dispara. Necesitas el host de escritorio abierto y acceso a **Automatizaciones**; no hacen falta cuenta de Cloud, servicio de audio ni destino externo.

<DocCallout title="Empieza con un efecto fácil de comprobar" tone="tip">

Utiliza únicamente la acción de registro durante esta primera prueba. Usa texto ficticio o no sensible: el registro conserva el mensaje resultante.

</DocCallout>

## Crear la acción {#create-the-action}

<DocSteps>

1. **Abre Automatizaciones → Acciones** en el perfil que quieras utilizar.
2. **Crea la acción.** Selecciona **Write to the log** (`core.log`, escribir en el registro) y llámala `Comprobar chat`.
3. **Configura el mensaje.** Introduce la siguiente plantilla en el campo del mensaje, pulsa **Guardar** y activa la acción.

</DocSteps>

```text
Comprobar chat: {{ event.user.uniqueId }} dijo {{ event.data.comment }}
```

Estas son las rutas normalizadas actuales del chat. `event.data.comment` contiene el mensaje; `event.user.uniqueId`, el identificador del espectador. El nombre visible utiliza otro campo.

<DemoFrame scenario="automation" locale="es" panel="automation" title="Explora las acciones y eventos de una automatización" />

## Crear el evento {#create-the-event}

<DocSteps>

1. **Crea el disparador.** Abre **Eventos**, crea `Evento comprobar chat` y selecciona `tiktok.chat`.
2. **Vincula tu acción.** Añade únicamente `Comprobar chat` y conserva el modo normal de ejecutar todas las acciones.
3. **Prepara la primera muestra.** Deja los filtros vacíos y configura una espera global de `10` segundos para limitar después el tráfico real.
4. **Guarda y prueba.** Revisa cualquier vista previa de cambios. Con solo la acción de registro vinculada, utiliza su prueba e inspecciona la ejecución y el texto resultante. El host puede aportar una muestra; ese resultado no demuestra conectividad real con TikTok.
5. **Añade una condición.** Usa el campo `event.data.comment`, operador igual (`eq`), valor `!hello`. Guarda otra vez y activa explícitamente el evento.
6. **Comprueba un mensaje real.** Conecta a un [LIVE](../live/connections.md) donde tengas autorización para probar. Observa un mensaje nuevo `!hello` en el [feed](../live/feed.md) y consulta el historial cuando la espera permita ejecutarlo.

</DocSteps>

## Resultado esperado y estado {#expected-result-and-status}

La acción local de registro y el disparador de chat están disponibles. La muestra debería producir una línea legible; un chat real coincidente, una ejecución con el identificador y mensaje recibidos. Por ejemplo, el resultado para `luna` sería `Comprobar chat: luna dijo !hello`. Otro texto no debería superar la condición exacta.

## Recuperación {#recovery}

Si la muestra deja de coincidir tras añadir el filtro, su comentario puede ser distinto de `!hello`. Comprueba los campos reales, el perfil activo, la espera y los interruptores antes de repetir.

<DocCallout title="Revisa el efecto antes de ejecutar" tone="warning">

Las pruebas de acciones y el disparo de un evento pueden ejecutar efectos. Esta regla solo escribe un registro, por lo que su efecto es previsible. Desactívala al terminar o continúa con las [recetas](recipes.md).

</DocCallout>
