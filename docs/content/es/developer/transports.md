---
title: "Transportes actuales y restricciones de plataforma"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "008f1f03ea7ac38072efb2584c11a75601c1c69c142a2fb5ae8953afba876b02"
translationRevision: "008f1f03ea7ac38072efb2584c11a75601c1c69c142a2fb5ae8953afba876b02"
order: 200
---

# Transportes actuales y restricciones de plataforma

## Objetivo y requisitos

Selecciona el transporte que existe en la compilación y plataforma de destino. Lee [Control API](control-api.md) y [colaboración](collaboration.md). Un transporte lleva operaciones serializadas; no debe crear otro núcleo ni otorgar autoridad por sí mismo.

## Elegir el límite

| Recorrido | Implementación actual | Límite importante |
| --- | --- | --- |
| WebView de escritorio | IPC Wry y cliente compartido | Ventana e hilo pertenecen al escritorio |
| CLI local | Socket Unix o tubería Windows; host stdio explícito | Mismo directorio y propietario |
| Navegador local | Adaptador nativo y `browser-transport.ts` | Host emparejado, no simulación ni servidor remoto público |
| Remoto directo | Gateway HTTPS/WSS y transportes SDK/navegador | TLS, negociación, permisos y características |
| Par nativo | `tiktools-peer`, flujo iroh/QUIC cifrado | Identidad de par no equivale a autoridad RPC |
| WebRTC nativo | `tiktools-rtc`, señalización firmada y canales de datos | Exige negociación, permisos y ruta comprobada |

La interfaz TypeScript está en `src/web/shared/api/control-transport.ts`. El transporte de pares no activa implícitamente relays públicos ni descubrimiento de terceros. WebRTC es nativo Rust (`webrtc` 0.21.1), sin depender de `RTCPeerConnection` del navegador en la interfaz del escritorio.

## Inspeccionar la ruta seleccionada

Descubre métodos y capacidades del host antes de emparejar o mostrar acciones del controlador:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` observa el par ICE seleccionado para clasificar conexión directa o relay. El estado listo no demuestra una ruta directa. La señalización/relay Cloud es independiente de la autorización y debe conservar la comprobación de permisos nativos.

## Resultado esperado y estado

IPC local y transporte de escritorio son rutas establecidas. Pares nativos, WebRTC y controlador tienen código y pruebas dirigidas, pero despliegue y aceptación entre equipos siguen experimentales. Los textos que afirman ausencia de pares/relay son históricos. Android admite IPC local; los perfiles remotos persistidos carecen de almacén de credenciales seleccionado y no hay addons/WebView de escritorio.

## Recuperación

Una desconexión puede dejar operaciones aceptadas inciertas. Reconectar exige invalidar proyecciones privadas, negociar identidad y reanudar o recargar flujos acotados. No debilites certificados ni vuelvas silenciosamente al propietario local tras un fallo remoto. La [implementación RTC](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src) y el grafo real sustituyen planes antiguos.
