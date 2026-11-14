---
title: "Transportes actuales y restricciones de plataforma"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6696987a0db16b3b4d364c6136d0e552dbebd6fc68346e3ed031587a7b004468"
translationRevision: "1d16d732eaf401a9004105ac650774e0b60757e2ac74241077e6a6a1e39841c4"
order: 200
---

# Transportes actuales y restricciones de plataforma {#current-transports-and-platform-restrictions}

## Objetivo y requisitos {#goal-and-prerequisites}

Selecciona el transporte que existe en la compilación y plataforma de destino. Lee [Control API](control-api.md) y [colaboración](collaboration.md). Un transporte lleva operaciones serializadas; no debe crear otro núcleo ni otorgar autoridad por sí mismo.

## Elegir el límite {#choose-the-boundary}

| Recorrido | Implementación actual | Límite importante |
| --- | --- | --- |
| WebView de escritorio | IPC Wry y cliente compartido | Ventana e hilo pertenecen al escritorio |
| CLI local | Socket Unix o tubería Windows; host stdio explícito | Mismo directorio y propietario |
| Navegador local | Adaptador nativo y `browser-transport.ts` | Host emparejado, no simulación ni servidor remoto público |
| Remoto directo | Gateway HTTPS/WSS y transportes SDK/navegador | TLS, negociación, permisos y características |
| Par nativo | `tiktools-peer`, flujo iroh/QUIC cifrado | Identidad de par no equivale a autoridad RPC |
| WebRTC nativo | `tiktools-rtc`, señalización firmada y canales de datos | Exige negociación, permisos y ruta comprobada |

La interfaz TypeScript está en `src/web/shared/api/control-transport.ts`. El transporte de pares no activa implícitamente relays públicos ni descubrimiento de terceros. WebRTC es nativo Rust (`webrtc` 0.21.1), sin depender de `RTCPeerConnection` del navegador en la interfaz del escritorio.

## Inspeccionar la ruta seleccionada {#inspect-the-selected-route}

Descubre métodos y capacidades del host antes de emparejar o mostrar acciones del controlador:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` observa el par ICE seleccionado para clasificar conexión directa o relay. El estado listo no demuestra una ruta directa. La señalización/relay Cloud es independiente de la autorización y debe conservar la comprobación de permisos nativos.

## Resultado esperado y estado {#expected-result-and-status}

IPC local y transporte de escritorio son rutas establecidas. Pares nativos, WebRTC y controlador tienen código y pruebas dirigidas, pero despliegue y aceptación entre equipos siguen experimentales. Los textos que afirman ausencia de pares/relay son históricos. Android admite IPC local; los perfiles remotos persistidos carecen de almacén de credenciales seleccionado y no hay addons/WebView de escritorio.

## Recuperación {#recovery}

Una desconexión puede dejar operaciones aceptadas inciertas. Reconectar exige invalidar proyecciones privadas, negociar identidad y reanudar o recargar flujos acotados. No debilites certificados ni vuelvas silenciosamente al propietario local tras un fallo remoto. La [implementación RTC](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src) y el grafo real sustituyen planes antiguos.

## Detalles de operación {#operational-details}

Peer usa ALPN `tiktools/collaboration/1`, timeout actual de 15 s y hasta cuatro relays configurados explícitamente. Peer/RTC limitan tramas a 1 MiB; RTC fragmenta en paquetes de hasta 16 KiB. Una trama excesiva falla validación, no produce RPC parcial. Diagnostica por separado configuración/identidad, señalización, timeout, tamaño y autorización. Relay Cloud no es servidor TURN de ICE; una lista vacía no habilita discovery público. Comprueba host/dispositivo y permiso confirmado vigente antes de probar ruta, y registra si ICE elige directo o relay. Transporte ready/cifrado no demuestra autoridad ni acceso por todo NAT. Prueba sesión cambiada/revocada y tramas excesivas con fixtures antes de aceptar dos PC reales.
