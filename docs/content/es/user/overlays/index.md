---
title: "Preparar superposiciones para OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3141f778f3bf0e5c9e01415122e7863e9342c266a57bee9811d7a6b75b2c842b"
translationRevision: "6ee9433054bc94f58084d3f2788e5e813c322f156395359d2c0c633c6f3026ed"
order: 10
---

# Preparar superposiciones para OBS {#prepare-overlays-for-obs}

## Objetivo y requisitos {#goal-and-prerequisites}

Diseña un widget, comprueba cómo se verá en tu emisión y conecta su fuente de navegador a OBS. El ejemplo de esta página permite explorar el editor con datos ficticios antes de preparar la escena real.

<DocCallout title="Prepara la emisión local" tone="info">

Necesitas TikTools abierto, la capacidad de pasarela de eventos y OBS en el mismo equipo para la dirección local predeterminada. La salida en directo también requiere el evento pertinente o una fuente de ranking configurada.

</DocCallout>

## Diseñar y previsualizar {#design-and-preview}

<DocSteps>

1. **Elige el widget.** Abre **Overlays** y lee el estado de la **Pasarela de eventos**. Elige seguidores, regalos, chat, compartidos, suscripciones, likes o ranking.
2. **Ajusta el diseño.** Edita capas, texto visible, colores, espacios y avatar en **Propiedades**. Utiliza contraste suficiente y un tamaño legible a la resolución real de emisión.
3. **Comprueba la presentación.** Pulsa **Repetir vista previa** para inspeccionar el diseño. Su contenido ayuda a evaluar la presentación; no demuestra entrega LIVE.
4. **Guarda el diseño.** Puedes utilizar **Deshacer** y **Rehacer** antes de pulsar **Guardar diseño**. Cambiar de tipo con modificaciones pendientes solicita descartarlas.

</DocSteps>

<DemoFrame scenario="overlays" locale="es" panel="overlays" title="Prueba el editor y la vista previa de overlays" />

<DocSteps>

1. **Obtén la fuente.** En **URL de la fuente de navegador**, utiliza **Copiar URL** para obtener la dirección completa; la dirección visible oculta su credencial. Mantenla privada.
2. **Conecta OBS.** Añade una fuente de navegador y pega la URL copiada. Mantén TikTools abierto y verifica un evento pertinente antes de depender de ella durante la emisión.

</DocSteps>

La pasarela predeterminada es `127.0.0.1:17452`. Una dirección local se refiere al equipo que ejecuta la fuente de navegador; copiarla a otro equipo no permite acceder automáticamente a este host. Los widgets de ranking requieren además un ID seleccionado; consulta su [estado](../points/rankings.md).

## Utilizar la biblioteca Cloud {#use-the-cloud-library}

Cuando el host anuncia las capacidades correspondientes, abre la biblioteca desde **Overlays**. Consulta diseños locales o **Mis overlays**, crea una superposición en el editor Cloud o abre un ID existente y publica una revisión. El editor admite texto, campos en directo, ilustraciones y dimensiones del lienzo; las imágenes se suben mediante un flujo aparte. Guardar un diseño local y crear o revisar uno en Cloud son operaciones distintas.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles los diseños locales, la pasarela y los flujos de editor/biblioteca Cloud según capacidades. Esta versión no incluye un adaptador compartido de **Cambios** para superposiciones.

<DocCallout title="Comprueba el recorrido completo" tone="tip">

El resultado esperado es ver el evento elegido en la fuente de navegador de OBS. Una vista previa correcta en TikTools no prueba la recepción en OBS: verifica también el LIVE real y el sistema operativo en tu entorno.

</DocCallout>

## Recuperación {#recovery}

Si falta salida, revisa estado de pasarela, recursos, credenciales, URL y tipo de evento. Ante conflictos de revisión Cloud, carga la versión actual antes de editar. No publiques el token en capturas; informa del error mediante [diagnósticos](../preferences/troubleshooting.md).

## Detalles de operación {#operational-details}

Instala el plugin Event Gateway por separado, habilítalo y comprueba que se está ejecutando antes de añadir fuentes OBS para el LIVE. Cada estado orienta la recuperación:

| Estado | Qué hacer |
| --- | --- |
| `missing` | Instala el plugin. |
| `disabled` | Habilítalo y revisa su disponibilidad. |
| `stopped` | Inicia la pasarela. |
| `credentialUnavailable` | Reinicia la pasarela una vez. |
| `assetsMissing` | Repara o reconstruye sus recursos de widgets empaquetados. |
| `starting` | Espera brevemente y vuelve a comprobar. |
| `unreachable` | Revisa el puerto y el servicio de la dirección local. |

Usa la credencial del widget incluida en la URL generada. Un estado listo o una vista previa local no prueban el LIVE ni la recepción en OBS: realiza una prueba inocua en la fuente prevista.
