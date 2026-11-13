---
title: "SDK de widgets, gateway y límites de overlays"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "84e4b5a3347945462c691684f682f75e24cb4e72e31fa53592da4257b3e3d1b8"
translationRevision: "84e4b5a3347945462c691684f682f75e24cb4e72e31fa53592da4257b3e3d1b8"
order: 170
---

# SDK de widgets, gateway y límites de overlays

## Objetivo y requisitos

Construye un widget que comparta renderizado entre vista previa local y salida OBS. Usa las herramientas Vue instaladas y conoce los [eventos](events.md). La vista previa no necesita gateway ni credenciales. La salida en vivo requiere el plugin event-gateway instalado por separado y su credencial específica para widgets.

## Añadir una plantilla

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

## Protocolo del gateway

`src/widgets/shared/gateway-client.ts` controla autenticación, suscripciones, latido, reconexión y huecos. Los widgets consumen sobres canónicos, sin conectarse directamente a TikTok. `/ws/widgets` es distinto de la superficie completa para clientes externos. Analiza los mensajes mediante el módulo compartido y conserva el límite de credenciales sin pasar un token completo por la WebView principal.

## Resultado esperado y estado

Las vistas previas integradas y el SDK están implementados. Existen editor Cloud y código `cloud-overlay`, pero continúan los límites de validación OBS real, ilustraciones/fuentes en vivo y aplicación desde paquetes. Changes de Overlays es un seguimiento sin integrar. Rankings sigue oculto en navegación normal aunque existan widgets de clasificación.

## Recuperación

Ante una vista previa vacía, comprueba vida del controlador, valores del esquema y dimensiones antes de debilitar políticas de origen. En vivo, revisa salud, credencial de widget y huecos explícitos. Chromium sin interfaz no demuestra aceptación OBS. El [SDK](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk) y las pruebas de protocolo compartido son las referencias actuales.
