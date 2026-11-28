---
title: "Organizar automatizaciones"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4b3afb8ff9602033e4ffeba505da980bc2a9d617796853f59645c0f40eaf53ad"
translationRevision: "9f81dd55bf99ad4139ffbbca008e2d860cfeaf1848ee2c519a7b2004457f86d2"
order: 10
---

# Organizar automatizaciones {#organize-automations}

## Objetivo y requisitos {#goal-and-prerequisites}

Construye una biblioteca pequeña y comprensible que responda a eventos LIVE. Necesitas el host de escritorio funcionando. Una [conexión LIVE](../live/connections.md) real permite validar eventos externos; las acciones de plugins también requieren su plugin y servicio preparados.

## Comprender la biblioteca {#understand-the-library}

Una **Action** es una operación reutilizable configurada, como escribir una línea o reproducir sonido. Un **Event** elige disparador, filtros, espera y acciones vinculadas. Un perfil agrupa definiciones para un flujo de trabajo; solo hay un paquete de perfil activo a la vez. Las [Variables](variables.md) proporcionan valores reutilizables por varias acciones.

1. Abre **Automations** y confirma el perfil activo antes de editar. Busca Eventos y Acciones por separado para saber qué tipo de registro seleccionas.
2. Comienza con la [primera regla de registro](first-rule.md). Crea la acción y después el evento que la referencia.
3. Elige un disparador admitido por el host. Añade filtros mediante el selector de campos correspondiente. Varias condiciones restringen la coincidencia; no modifican el contenido entrante.
4. Define el tiempo de espera y su ámbito. Una espera global limita la regla entre todos los espectadores; una espera por usuario los controla por separado.
5. Vincula las acciones deseadas en orden. El modo normal ejecuta la lista elegida; el aleatorio selecciona una acción. Una acción de demora pospone las posteriores sin frenar la recepción LIVE.
6. Guarda, revisa los cambios de definición que se muestren y activa explícitamente los registros deseados. Consulta el historial para comprobar nombre, resultado y error.

Toda comparación admite una lista de valores en lugar de un solo valor. La combinación predeterminada es «cualquiera»: la condición se cumple cuando al menos un valor coincide. Cambia el cuantificador a «todos» cuando todos los valores deban coincidir; con «no es» equivale a «no es ninguno». Una condición también puede nombrar un alias de captura: el valor ganador queda disponible para las plantillas de acción como <code v-pre>{{ match.&lt;alias&gt; }}</code> (o <code v-pre>{{ match.&lt;posición&gt; }}</code> sin alias). Debajo de las condiciones siempre obligatorias puedes añadir hasta cuatro **alternativas**: grupos cuyas condiciones deben cumplirse todas dentro del grupo, y basta con que un solo grupo se cumpla para que el evento se dispare.

Las pruebas pueden ejecutar los efectos configurados. Lee la lista seleccionada antes de pulsar **Test** o disparar un evento, especialmente si incluye audio, HTTP, puntos o plugins. Aprende primero con registros.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles la edición local, la ejecución, las plantillas, los perfiles y el historial. Los disparadores de plugins aparecen mientras su plugin está instalado y activo. La activación depende de la superficie y de los valores enabled revisados; comprueba eventos y acciones antes de confirmar. **Changes** en Eventos/Acciones es una [comparación local](../cloud/changes.md), sin subida automática a Cloud.

## Recuperación {#recovery}

Si no se ejecuta, comprueba perfil activo, interruptores, disparador, filtros, espera y acciones ausentes, en ese orden. Ante un conflicto de revisión, vuelve a leer la definición guardada y revísala. Evita sobrescribir a ciegas cambios ajenos. Exporta un [perfil](templates.md) conocido antes de modificaciones amplias.

## Detalles de operación {#operational-details}

Antes de cambiar perfil/importar, lee [activación por superficie](templates.md#operational-details). El cambio puede activar miembros deshabilitados individualmente. El cooldown vive en memoria, con máximo 24 horas; sin uniqueId los espectadores comparten ámbito anónimo. La tabla se vacía al alcanzar 8.192 claves: sobrecarga o reinicio pueden permitir ejecuciones adicionales. No es cuota durable ni garantía de replay.

Pruébalo interactivamente antes de tocar tu configuración real:

<DemoFrame scenario="automation" locale="es" panel="automation" title="Prueba el espacio de automatizaciones" />

## Operaciones de línea de comandos {#command-line-operations}

Cada operación de esta guía también está disponible desde la CLI: sin conexión, scriptable y apta para agentes. Las líneas de uso son sintaxis del parser y permanecen en inglés en todos los idiomas.

<!--@include: ../../../../.generated/cli-domain-automation.es.md-->

<!--@include: ../../../../.generated/cli-domain-workflow.es.md-->

Consulta la [referencia completa de comandos](../../developer/cli.md) para las convenciones, la salida JSON y los marcadores de operaciones destructivas.

<TourFrame id="automationFilters" locale="es" title="Guía interactiva: Filtros y condiciones" />

<TourFrame id="automationCooldowns" locale="es" title="Guía interactiva: Cooldowns y modos" />
