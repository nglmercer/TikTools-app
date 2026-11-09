# Configuración y Conexiones — iteración OBS

Capturas reales del 2026-11-09. Código de interfaz: **9983db077779a2784739768183884a5345ef0db2**. El commit de esta galería solo añade imágenes/documentación. Frontend de producción compilado con `bun scripts/task.ts frontend-build`; host nativo local-web de la versión previa del PR (sin cambios de backend en esta iteración). Chromium real, API real de instancia temporal, sin mocks.

| Pantalla | Escritorio 1440 × 1000 | Móvil 390 × 844 |
| --- | --- | --- |
| Conexiones: paneles LIVE/explorador y servidores | [Captura](connections-desktop-es.png) | [Captura](connections-mobile-es.png) |
| General: idioma y aplicación | [Captura](general-desktop-es.png) | [Captura](general-mobile-es.png) |
| Variables desplegadas | [Captura](general-variables-desktop-es.png) | [Captura](general-variables-mobile-es.png) |
| Plugins: actualizaciones y fuentes | [Captura](plugins-desktop-es.png) | [Captura](plugins-mobile-es.png) |
| Formulario de nueva fuente desplegado | [Captura](plugins-add-source-desktop-es.png) | [Captura](plugins-add-source-mobile-es.png) |
| Cuenta / Cloud y diagnóstico | [Captura](account-desktop-es.png) | [Captura](account-mobile-es.png) |
| Colaboración: estado | [Captura](collaboration-desktop-es.png) | [Captura](collaboration-mobile-es.png) |
| Colaboración: red y seguridad | [Captura](collaboration-network-desktop-es.png) | [Captura](collaboration-network-mobile-es.png) |
| Colaboración: dispositivos y permisos | [Captura](collaboration-devices-desktop-es.png) | [Captura](collaboration-devices-mobile-es.png) |

## Qué cambió

Conexiones recupera paneles paralelos con cabeceras grises y el panel de servidores debajo. Ajustes se organiza en General, Plugins, Cuenta y Cloud, y Colaboración. Los controles booleanos visibles son switches propios; los desplegables son botones accesibles con regiones asociadas, no details/summary nativos. Las opciones extensas se despliegan sin desmontar borradores. Automatizaciones no se modificó ni se capturó.

Las imágenes muestran el viewport, con scroll real donde corresponde; no son composiciones. La captura del formulario de fuentes se desplaza hasta el formulario. Los valores de prueba de borradores se vaciaron/restauraron sin guardar configuración ni crear registros.

## Verificación y límites

Playwright comprobó el layout paralelo de escritorio y apilado móvil, switches con Espacio, desplegables con Enter/Espacio, borradores conservados, ausencia de details/summary y roles checkbox en Ajustes, cero errores de página y cero desbordamiento horizontal del documento. [Manifest de hashes](manifest.json).

Cuenta sin sesión, sin plugins instalados ni servidores configurados y compilación sin controladores remotos: los estados vacíos/advertencias son reales. No se simuló una conexión fallida ni una sincronización autenticada. No se crearon invitaciones, no se instalaron plugins ni se inició LIVE. Las filas de servidores pobladas y los formularios de plugins no se representan con datos inventados.
