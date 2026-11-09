# Capturas actuales de configuración — PR #62

Capturadas el 2026-11-09 desde el código **162fd6533f0d76a50c75e7a3a7ff5129134f8f37**, con frontend de producción y host nativo recompilados desde ese mismo commit. El commit que añade esta galería solo cambia documentación e imágenes.

Chromium real contra una instancia temporal de TikTools; sin respuestas API simuladas ni datos de cuenta. Idioma: español. Cada imagen corresponde al viewport visible, no a una composición de toda la página: las secciones largas tienen scroll.

| Pantalla | Escritorio 1440 × 1000 | Móvil 390 × 844 |
| --- | --- | --- |
| General | [Abrir captura](general-desktop-es.png) | [Abrir captura](general-mobile-es.png) |
| Cuenta / Cloud (sesión cerrada) | [Abrir captura](account-desktop-es.png) | [Abrir captura](account-mobile-es.png) |
| Actualizaciones | [Abrir captura](updates-desktop-es.png) | [Abrir captura](updates-mobile-es.png) |
| Avanzado y diagnóstico Cloud | [Abrir captura](advanced-desktop-es.png) | [Abrir captura](advanced-mobile-es.png) |
| Colaboración: estado | [Abrir captura](collaboration-desktop-es.png) | [Abrir captura](collaboration-mobile-es.png) |
| Colaboración: red y seguridad | [Abrir captura](collaboration-network-desktop-es.png) | [Abrir captura](collaboration-network-mobile-es.png) |
| Colaboración: dispositivos y solicitudes | [Abrir captura](collaboration-devices-desktop-es.png) | [Abrir captura](collaboration-devices-mobile-es.png) |
| Conexiones (sin servidores) | [Abrir captura](connections-desktop-es.png) | [Abrir captura](connections-mobile-es.png) |
| Plugins (sin instalaciones) | [Abrir captura](plugins-desktop-es.png) | [Abrir captura](plugins-mobile-es.png) |

## Límites observados

- Cuenta sin iniciar sesión: no representa sincronización autenticada ni conflictos entre dispositivos.
- Host compilado sin controladores remotos: las advertencias y controles deshabilitados de Colaboración son reales.
- No hay plugins instalados ni conexiones de servidor; no se inventaron formularios de configuración de plugins poblados.
- La instalación automática de actualizaciones no está disponible en este host.
- No se inició un LIVE, se creó una invitación ni se cambiaron credenciales/permisos.
- Automatizaciones no se modificó ni se capturó en esta actualización.

## Comprobaciones

`bun scripts/task.ts frontend-build` y `cargo build -p tiktools-cli --features local-web --locked` completados. Playwright/Chromium verificó rutas de configuración, secciones de Colaboración, estados esperados y ausencia de errores de página/desbordamiento horizontal en ambos tamaños. Los hashes SHA-256 de las imágenes están en [manifest.json](manifest.json).

Las capturas de `../flat-ui` son históricas; no representan las pantallas actuales de configuración y Automatizaciones.
