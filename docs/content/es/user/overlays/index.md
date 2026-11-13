---
title: "Preparar superposiciones para OBS"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "732d9bfcadaba7b08c8a5c0504eea8f23dc7e9b2d63ed9f07287506ac2dad1e8"
translationRevision: "732d9bfcadaba7b08c8a5c0504eea8f23dc7e9b2d63ed9f07287506ac2dad1e8"
order: 10
---

# Preparar superposiciones para OBS {#prepare-overlays-for-obs}

## Objetivo y requisitos {#goal-and-prerequisites}

Diseña un widget legible y conecta su fuente de navegador local a OBS. Necesitas TikTools abierto, la capacidad de pasarela de superposiciones y OBS en el mismo equipo para la dirección local predeterminada. La salida en directo también requiere el evento pertinente o una fuente de ranking configurada.

## Diseñar y previsualizar {#design-and-preview}

1. Abre **Overlays** y lee el estado de la pasarela. Elige seguidores, regalos, chat, compartidos, suscripciones, likes o ranking.
2. Edita capas, texto visible, colores, espacios y avatar en el inspector. Utiliza contraste suficiente y un tamaño legible a la resolución real de emisión.
3. Repite la vista previa para inspeccionar el diseño. Su contenido ayuda a evaluar la presentación; no demuestra entrega LIVE.
4. Usa los controles de deshacer y rehacer y después **Save**. Cambiar de tipo con modificaciones pendientes solicita descartarlas.
5. Copia la URL desde el panel de vista previa/fuente. La URL visible oculta su credencial; usa el control de copia para obtener la dirección utilizable. Mantenla privada.
6. Añade una fuente de navegador en OBS y pega la URL copiada. Mantén TikTools abierto y verifica un evento pertinente antes de depender de ella durante la emisión.

La pasarela predeterminada es `127.0.0.1:17452`. Una dirección local se refiere al equipo que ejecuta la fuente de navegador; copiarla a otro equipo no permite acceder automáticamente a este host. Los widgets de ranking requieren además un ID seleccionado; consulta su [estado](../points/rankings.md).

## Utilizar la biblioteca Cloud {#use-the-cloud-library}

Cuando el host anuncia las capacidades correspondientes, abre la biblioteca desde Overlays. Consulta diseños locales o **My overlays**, crea una superposición en el editor Cloud o abre un ID existente y publica una revisión. El editor admite texto, campos en directo, ilustraciones y dimensiones del lienzo; las imágenes se suben mediante un flujo aparte. Guardar un diseño local y crear o revisar uno en Cloud son operaciones distintas.

## Resultado esperado y estado {#expected-result-and-status}

Están disponibles los diseños locales, la pasarela y los flujos de editor/biblioteca Cloud según capacidades. Esta versión no incluye un adaptador compartido **Changes** para superposiciones. Debes comprobar OBS, LIVE real y el sistema operativo en tu entorno.

## Recuperación {#recovery}

Si falta salida, revisa estado de pasarela, recursos, credenciales, URL y tipo de evento. Ante conflictos de revisión Cloud, carga la versión actual antes de editar. No publiques el token en capturas; informa del error mediante [diagnósticos](../preferences/troubleshooting.md).

<DemoFrame scenario="overlays" locale="es" tour="overlay" title="Demo de overlays" />
