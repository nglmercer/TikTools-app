---
title: "Transportes actuales y restricciones de plataforma"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
translationRevision: "15e858773e746822b7f728503f4ecb7de0b01bff30aadc52426dbc9470470f02"
order: 200
---

# Transportes actuales y restricciones de plataforma {#current-transports-and-platform-restrictions}

## Objetivo y requisitos {#goal-and-prerequisites}

Selecciona el transporte que existe en la compilación y plataforma de destino. Lee [Control API](control-api.md) y [colaboración](collaboration.md). Un transporte lleva operaciones serializadas; no debe crear otro núcleo ni otorgar autoridad por sí mismo.

## Elegir el límite {#choose-the-boundary}

| Recorrido | Implementación actual | Límite importante |
| --- | --- | --- |
| Escritorio WebView | IPC de Wry y cliente de control compartido | El ciclo de GUI/hilos pertenece al escritorio |
| CLI local | Socket Unix o tubería con nombre de Windows; host stdio explícito | Misma carpeta y propietario de la aplicación |
| Navegador local | Adaptador loopback nativo más `browser-transport.ts` | Host nativo emparejado, no un servidor remoto público |
| Pasarela HTTPS/WSS autenticada | Transportes SDK/navegador con TLS, saludo, permisos y feature gates | Los comandos remotos necesitan un permiso explícito |
| Plugin de control remoto instalable | `tiktools.remote-control` sobre un relay WSS con `remote.controller.ticket` | Propiedad del plugin; el host no compila código remoto |

La interfaz TypeScript está en `src/web/shared/api/control-transport.ts`. El control remoto es un plugin instalable, no parte del binario del host: cada comando pasa por `HostCallPolicy` (`remote.command.dispatch`, `remote.pair.list`, `remote.grant.list`, `remote.grant.revoke`, `remote.controller.ticket`). Esta compilación no tiene transporte peer/RTC nativo ni enlace de host de Cloud Teams.

## Inspeccionar la ruta seleccionada {#inspect-the-selected-route}

Descubre los métodos y capacidades admitidos por el host conectado antes de emparejar o mostrar una acción de control:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
```

El emparejamiento y los permisos son propiedad del plugin: el ticket de controlador es de corta duración y un solo uso, y `remote.grant.list` / `remote.grant.revoke` son la vista autorizada. Una conexión de relay solo demuestra transporte, nunca autoridad RPC.

## Resultado esperado y estado {#expected-result-and-status}

El IPC local y el transporte de escritorio son rutas establecidas. El control remoto funciona mediante el plugin instalable y un relay autenticado; no es un segundo runtime de host. Los códigos de inicio de sesión se envían a los dispositivos conectados con una validez de cinco minutos, con OAuth como alternativa cuando no hay cuenta configurada. El IPC local en Android funciona con las herramientas instrumentadas `mobile` (`info` / `exec` / `tty`); los addons nativos y WebViews de escritorio siguen sin estar disponibles allí.

## Recuperación {#recovery}

Trata la desconexión como incertidumbre posible para operaciones aceptadas. La reconexión debe invalidar proyecciones privadas, renegociar la identidad y reanudar o capturar flujos limitados. Nunca debilites la verificación de certificados ni vuelvas silenciosamente a un propietario local tras un fallo remoto. Los tickets caducados y los permisos revocados deben fallar en cerrado.

## Detalles operativos {#operational-details}

Mantén los permisos con el mínimo privilegio y plazo limitado; vuelve a comprobarlos tras cada reconexión. Diagnostica por separado identidad, caducidad del ticket, alcance del permiso, alcance del relay y tamaño de trama. Las tramas grandes fallan en la validación de encuadre, no producen RPC parciales. Prueba sesiones modificadas o revocadas con fixtures antes de la aceptación real con dos dispositivos.
