---
title: "Revisar cambios según el área"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "470449e93855a0308eeb03939acc2d6e929761dd28d5f7ff8930152aaf0d1fe7"
translationRevision: "470449e93855a0308eeb03939acc2d6e929761dd28d5f7ff8930152aaf0d1fe7"
order: 20
---

# Revisar cambios según el área {#review-changes-by-area}

## Objetivo y requisitos {#goal-and-prerequisites}

Comprende qué compara un panel **Changes** antes de restablecer, enviar o adoptar datos. Necesitas contenido local guardado. Las operaciones Cloud también requieren un host compatible, el modo de sincronización pertinente y una [cuenta autenticada](account.md). Los paneles con el mismo nombre no realizan necesariamente el mismo transporte.

## Revisar Eventos y Acciones localmente {#review-events-and-actions-locally}

1. Abre **Automations**, selecciona Events o Actions y entra en **Changes**.
2. Lee el aviso: esta área compara las definiciones actuales con una referencia conservada en este dispositivo. La primera visita con datos cargados crea esa referencia, por lo que puede no mostrar diferencias.
3. Selecciona un registro añadido, modificado o eliminado e inspecciona los valores anterior y posterior.
4. Usa **Mark as reviewed** únicamente para convertir la sección actual en la nueva referencia local. No sube definiciones ni aprueba ejecución delegada.
5. Usa **Reset** solo si deseas recuperar la referencia. Para un registro recién añadido, lo elimina; para uno modificado o borrado, restaura la definición conservada. Exporta antes si necesitas guardar el estado actual.

El adaptador de Eventos/Acciones no dispone de colección Cloud para definiciones individuales en esta versión. Una comparación limpia no constituye una copia en Cloud ni demuestra que otro equipo recibió tus cambios.

## Revisar Variables y Plugins explícitamente {#review-variables-and-plugins-explicitly}

Variables tiene intercambio nativo revisado: guarda localmente, consulta propuestas, inspecciona la comparación y envía, aplica/adopta o retira de forma explícita. Los valores secretos o bloqueados permanecen ocultos y no pueden enviarse por esa ruta. Guardar localmente nunca sube datos. Consulta [Variables](../automations/variables.md).

Changes de Plugins utiliza su espacio de sincronización compatible para selecciones y ajustes preparados. Publica por sección, sin un envío independiente por cada fila. Los ajustes muestran nombres de claves modificadas sin revelar valores; los conflictos permiten elegir la versión local o Cloud, sin mezclar campos arbitrariamente. Lee la acción de sección y el modo seleccionado antes de publicar.

Las superposiciones Cloud usan el flujo de creación/revisión de [biblioteca y editor](../overlays/index.md). Esta versión base no incluye un adaptador Changes compartido para ellas.

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

Están disponibles la comparación local y el intercambio nativo según capacidades en sus áreas respectivas. Las operaciones remotas dependen del servicio; no se implica aceptación en producción. Si una revisión caduca, actualiza y compara otra vez. Si el resultado no está confirmado, inspecciona estado e historial antes de crear otra operación. Un tiempo de espera agotado no demuestra que nada haya cambiado.

<DemoFrame scenario="cloud" locale="es" tour="changes" title="Demo de cloud" />
