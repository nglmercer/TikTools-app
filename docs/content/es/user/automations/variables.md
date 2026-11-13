---
title: "Reutilizar y revisar variables de ejecución"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e8a5f268a9aa929d72578a9f6636533c2326215109c40f779375a8832e39fd70"
translationRevision: "e8a5f268a9aa929d72578a9f6636533c2326215109c40f779375a8832e39fd70"
order: 40
---

# Reutilizar y revisar variables de ejecución

## Objetivo y requisitos

Cambia una vez un valor compartido y reutilízalo en varias acciones. Necesitas **Automations → Variables** y un host compatible con las operaciones correspondientes. La edición tipada y las operaciones revisadas de Cloud dependen de sus capacidades; guardar localmente no requiere subir datos.

## Guardar un valor local

1. Añade una variable llamada `showLabel`. Las claves comienzan con letra o guion bajo y admiten letras, números, puntos y guiones, hasta `64` caracteres. Evita claves repetidas.
2. Si está disponible la edición tipada, elige texto e introduce `Programa de la tarde`. También se admiten número, booleano, lista y secreto. Las listas utilizan un arreglo JSON de cadenas, como `["rojo", "azul"]`.
3. Pulsa **Save**. Un conflicto de revisión significa que la colección guardada cambió; vuelve a cargarla y compara con tu modificación prevista.
4. Utiliza el valor guardado en un campo de acción compatible con plantillas:

```text
{{ globals.showLabel }}: {{ event.user.uniqueId }}
```

5. Prueba una acción que solo escriba un registro para inspeccionar el texto. Cambia `showLabel`, guarda y repite. Las definiciones existentes pueden reutilizar el valor actualizado sin importar de nuevo una plantilla.

Los `params` se sustituyen al aplicar una plantilla; los `globals` se leen durante la ejecución. Una clave ausente produce texto vacío: verifica su escritura antes de usarla como destino o identificador. Las sugerencias del campo ofrecen rutas de evento apropiadas para el disparador.

## Revisar cambios de Cloud por separado

El control **Changes** de Variables ofrece operaciones nativas revisadas de envío, consulta, aplicación/adopción y retirada cuando el host las admite. Guarda primero la edición local, selecciona una comparación, inspecciona tipo y valor y elige explícitamente la dirección. Consultar una propuesta de Cloud es distinto de aplicarla localmente. Retirar un valor compartido también es distinto de borrar una fila local.

Los valores secretos o bloqueados se ocultan en las comparaciones y no pueden enviarse mediante esta ruta. No conviertas un secreto en texto ordinario para evitar el bloqueo.

## Resultado esperado, estado y recuperación

Están disponibles las variables locales y el intercambio revisado según capacidades; guardar localmente nunca sube datos automáticamente. Las claves o valores inválidos y las revisiones antiguas bloquean el guardado. Corrige el campo señalado o vuelve a cargar antes de repetir. Para fallos de sesión, consulta [cuenta de Cloud](../cloud/account.md); para comparaciones, [Changes](../cloud/changes.md).
