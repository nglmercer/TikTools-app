---
title: "Reutilizar y revisar variables de ejecución"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1c64b76271cba5b5bb23ef1f671be2bd5bf6c286afb710d2bf85ccf8cecc66b0"
translationRevision: "79e8f37c7cb401fcd92a9515d346c36a1119d58fa9e7084451e9c55993c618b8"
order: 40
---

# Reutilizar y revisar variables de ejecución {#reuse-and-review-runtime-variables}

## Objetivo y requisitos {#goal-and-prerequisites}

Cambia una vez un valor compartido y reutilízalo en varias acciones. Necesitas **Automations → Variables** y un host compatible con las operaciones correspondientes. La edición tipada y las operaciones revisadas de Cloud dependen de sus capacidades; guardar localmente no requiere subir datos.

## Guardar un valor local {#save-a-local-value}

1. Añade una variable llamada `showLabel`. Las claves comienzan con letra o guion bajo y admiten letras, números, puntos y guiones, hasta `64` caracteres. Evita claves repetidas.
2. Si está disponible la edición tipada, elige texto e introduce `Programa de la tarde`. También se admiten número, booleano, lista y secreto. Las listas utilizan un arreglo JSON de cadenas, como `["rojo", "azul"]`.
3. Pulsa **Save**. Un conflicto de revisión significa que la colección guardada cambió; vuelve a cargarla y compara con tu modificación prevista.
4. Utiliza el valor guardado en un campo de acción compatible con plantillas:

```text
{{ globals.showLabel }}: {{ event.user.uniqueId }}
```

5. Prueba una acción que solo escriba un registro para inspeccionar el texto. Cambia `showLabel`, guarda y repite. Las definiciones existentes pueden reutilizar el valor actualizado sin importar de nuevo una plantilla.

Los `params` se sustituyen al aplicar una plantilla; los `globals` se leen durante la ejecución. Una clave ausente produce texto vacío: verifica su escritura antes de usarla como destino o identificador. Las sugerencias del campo ofrecen rutas de evento apropiadas para el disparador.

Las condiciones también pueden publicar valores por coincidencia. Cuando un filtro declara un alias de captura, el valor que lo satisfizo queda disponible mientras se renderizan las acciones del evento como <code v-pre>{{ match.&lt;alias&gt; }}</code> (o <code v-pre>{{ match.&lt;posición&gt; }}</code> sin alias). Por ejemplo, una condición «empieza por» sobre el comentario con el alias `prefix` permite que una acción de registro imprima <code v-pre>{{ match.prefix }}</code>. Las capturas son texto plano acotado, solo las producen los filtros y las alternativas que realmente coincidieron, y no cambian las claves de cooldown.

Pulsa **+** junto a un campo compatible con plantillas para abrir **Insertar variable**: la ruta exacta aparece a la derecha y al elegir una fila se añade <code v-pre>{{ ruta }}</code> al campo. Las capturas de este evento también aparecen allí, pero solo se resuelven si su condición coincidió realmente. Asigna un alias a una condición en la sección Condiciones (el botón **Capturar como** del pie muestra los campos de alias) para escribir <code v-pre>{{ match.&lt;alias&gt; }}</code>; si no, usa la posición <code v-pre>{{ match.&lt;posición&gt; }}</code>.

## Revisar cambios de Cloud por separado {#review-cloud-changes-separately}

El control **Changes** de Variables ofrece operaciones nativas revisadas de envío, consulta, aplicación/adopción y retirada cuando el host las admite. Guarda primero la edición local, selecciona una comparación, inspecciona tipo y valor y elige explícitamente la dirección. Consultar una propuesta de Cloud es distinto de aplicarla localmente. Retirar un valor compartido también es distinto de borrar una fila local.

Los valores secretos o bloqueados se ocultan en las comparaciones y no pueden enviarse mediante esta ruta. No conviertas un secreto en texto ordinario para evitar el bloqueo.

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

Están disponibles las variables locales y el intercambio revisado según capacidades; guardar localmente nunca sube datos automáticamente. Las claves o valores inválidos y las revisiones antiguas bloquean el guardado. Corrige el campo señalado o vuelve a cargar antes de repetir. Para fallos de sesión, consulta [cuenta de Cloud](../cloud/account.md); para comparaciones, [Changes](../cloud/changes.md).

Pruébalo interactivamente antes de tocar tu configuración real:

<DemoFrame scenario="automation" locale="es" panel="automation" title="Prueba el espacio de automatizaciones" />

## Operaciones de línea de comandos {#command-line-operations}

Cada operación de esta guía también está disponible desde la CLI: sin conexión, scriptable y apta para agentes. Las líneas de uso son sintaxis del parser y permanecen en inglés en todos los idiomas.

<!--@include: ../../../../.generated/cli-domain-globals.es.md-->

Consulta la [referencia completa de comandos](../../developer/cli.md) para las convenciones, la salida JSON y los marcadores de operaciones destructivas.

<TourFrame id="automationVariables" locale="es" title="Guía interactiva: Automatizaciones: variables" />
