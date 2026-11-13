---
title: "Organizar automatizaciones"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d387d7bdb1836994210e926b3eb87a8db533292ec393afcba6adb22dedfff286"
translationRevision: "d387d7bdb1836994210e926b3eb87a8db533292ec393afcba6adb22dedfff286"
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

Las pruebas pueden ejecutar los efectos configurados. Lee la lista seleccionada antes de pulsar **Test** o disparar un evento, especialmente si incluye audio, HTTP, puntos o plugins. Aprende primero con registros.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles la edición local, la ejecución, las plantillas, los perfiles y el historial. Los disparadores de plugins aparecen mientras su plugin está instalado y activo. Las definiciones importadas permanecen desactivadas hasta su activación explícita. **Changes** en Eventos/Acciones es una [comparación local](../cloud/changes.md), sin subida automática a Cloud.

## Recuperación {#recovery}

Si no se ejecuta, comprueba perfil activo, interruptores, disparador, filtros, espera y acciones ausentes, en ese orden. Ante un conflicto de revisión, vuelve a leer la definición guardada y revísala. Evita sobrescribir a ciegas cambios ajenos. Exporta un [perfil](templates.md) conocido antes de modificaciones amplias.
