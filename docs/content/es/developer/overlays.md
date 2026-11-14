---
title: "SDK de widgets, gateway y límites de overlays"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "99d3351859ddb3a3304133f062f68b8e3130416947fc40433dc9c6bffaf85f6a"
translationRevision: "c85583e3ca5d09b8e5e036553fd83606450727d27347a785793f4c35012b1e52"
order: 170
---

# SDK de widgets, gateway y límites de overlays {#widget-sdk-gateway-and-overlay-boundaries}

## Objetivo y requisitos {#goal-and-prerequisites}

Construye un widget que comparta renderizado entre vista previa local y salida OBS. Usa las herramientas Vue instaladas y conoce los [eventos](events.md). La vista previa no necesita gateway ni credenciales. La salida en vivo requiere el plugin event-gateway instalado por separado y su credencial específica para widgets.

## Añadir una plantilla {#add-a-template}

1. Crea el renderizador dentro de `WidgetStage`; dimensiona respecto al padre, no a toda la ventana.
2. Usa `defineWidget` de `src/widgets/sdk/template.ts`. Proporciona esquema, sobres de muestra nuevos y factoría `create(search, clock)`.
3. Devuelve un controlador con `handleEnvelope`, `clear` y `dispose`, además del renderizado. Emplea el reloj suministrado para mantener muestras y avanzar tiempo en pruebas.
4. Registra la plantilla en `sdk/templates.ts` y añade entrada independiente, compilación, ruta permitida del gateway y empaquetado. Registrarla no publica automáticamente una URL.
5. Usa `WidgetHost.vue` tanto en el editor como en el paquete independiente; no incrustes HTML del gateway en una vista previa local.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts test bun widgets
```

Conserva tokens, valores visuales y campos editables en el esquema. Los diseños se serializan como JSON y la URL copiada incluye una instantánea validada; tras guardarla de nuevo hay que copiar otra URL a OBS.

## Protocolo del gateway {#gateway-protocol}

`src/widgets/shared/gateway-client.ts` controla autenticación, suscripciones, latido, reconexión y huecos. Los widgets consumen sobres canónicos, sin conectarse directamente a TikTok. `/ws/widgets` es distinto de la superficie completa para clientes externos. Analiza los mensajes mediante el módulo compartido y conserva el límite de credenciales sin pasar un token completo por la WebView principal.

## Resultado esperado y estado {#expected-result-and-status}

Las vistas previas integradas y el SDK están implementados. Existen editor Cloud y código `cloud-overlay`, pero continúan los límites de validación OBS real, ilustraciones/fuentes en vivo y aplicación desde paquetes. Changes de Overlays es un seguimiento sin integrar. Rankings sigue oculto en navegación normal aunque existan widgets de clasificación.

## Recuperación {#recovery}

Ante una vista previa vacía, comprueba vida del controlador, valores del esquema y dimensiones antes de debilitar políticas de origen. En vivo, revisa salud, credencial de widget y huecos explícitos. Chromium sin interfaz no demuestra aceptación OBS. El [SDK](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk) y las pruebas de protocolo compartido son las referencias actuales.

## Detalles de operación {#operational-details}

La suite unitaria usa fixtures; Chromium con gateway real necesita widgets construidos y checkout externo elegido por `TIKTOOLS_PLUGINS_REPO`. Si falta fuente se omite: informa el skip. Ejecuta `bun x --no-install playwright test --config src/widgets/playwright.gateway.config.ts` para ese límite. Después verifica OBS real, credencial widget, entrega y recuperación con LIVE; Chromium headless no acredita aceptación OBS/nativa.
