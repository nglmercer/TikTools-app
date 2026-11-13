---
title: "Permisos e interfaz aislada de plugins"
docId: "developer.plugins.permissions-ui"
translationKey: "developer.plugins.permissions-ui"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
translationRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
order: 270
---

# Permisos e interfaz aislada de plugins {#plugin-permissions-and-isolated-ui}

## Objetivo y requisitos {#goal-and-prerequisites}

Expón configuración y efectos intencionales sin importar scripts de plugins a la aplicación principal. Usa manifiesto validado, paquetes SDK/UI compatibles y host privado. Lee [confianza del runtime](runtimes.md): una capacidad declarada no aísla del sistema a bibliotecas nativas ni procesos.

## Declarar una contribución mínima {#declare-a-narrow-contribution}

Describe campos de acciones/ajustes mediante esquemas JSON e indicaciones de interfaz. Las páginas declarativas sencillas renderizan componentes confiables del host. `events.publish` permite intenciones de evento; audio exige capacidad `audio.play` y permiso `audio.output`. Consulta el [esquema canónico](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json) y el analizador Rust, evitando copiar una lista exhaustiva de permisos.

Los procesadores requieren su contribución de enriquecimiento y devuelven anotaciones/vistas/registros sin efectos. Los observadores necesitan `events.subscribe` y reciben sobres estables mediante entrega acotada, sin acceder al enum interno Rust ni a manejadores arbitrarios de transporte.

## Construir una página aislada {#build-an-isolated-custom-page}

Un manifiesto `ui` con `apiVersion:1` y `mode:"webview"` apunta a recursos compilados `ui/dist/`. La aplicación aloja un marco de origen opaco con scripts; las ventanas separadas opcionales utilizan un intermediario nativo restringido. El código personalizado nunca ejecuta dentro del runtime Vue principal.

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
window.addEventListener('pagehide', stop, { once: true });
```

El host proporciona la identidad; el cliente no puede elegir otro plugin. El intermediario limita ajustes, acciones, opciones y eventos al plugin propio, además de idioma/tema. Una página sin intermediario falla explícitamente. Ejecutar una acción tiene efectos reales aunque el botón indique prueba.

## Resultado esperado y estado {#expected-result-and-status}

Renderizado declarativo, marcos y ventanas nativas tienen contratos y pruebas implementados. CORS/CSP, vínculo de ventana e intermediario nativo mantienen juntos el límite; Shadow DOM no lo hace. No es necesario borrar el IPC no configurable de WebKitGTK porque su controlador ya está restringido.

## Recuperación y verificación {#recover-and-verify}

Ante un marco vacío, revisa rutas, CSP y CORS de origen opaco antes de relajar políticas. Ante intermediario ausente, revisa el contexto anfitrión. Ejecuta `bun scripts/task.ts test bun plugin-ui` y las pruebas frontend correspondientes. El [paquete del intermediario](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-ui/src/broker.ts) define versiones, sobres y liberación.
