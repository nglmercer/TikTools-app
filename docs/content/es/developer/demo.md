---
title: "Arquitectura y límites de la demo web"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e2311798c9114ae8540db974017d7b5a571f626ce02f31a39fd6b6e64410491c"
translationRevision: "e2311798c9114ae8540db974017d7b5a571f626ce02f31a39fd6b6e64410491c"
order: 140
---

# Arquitectura y límites de la demo web

## Objetivo y requisitos

Explora la interfaz real con datos sintéticos en un navegador. Usa Bun 1.4.4 y la instalación bloqueada del repositorio. Esta guía describe la integración prevista de documentación/demo: el runtime y sus comprobaciones deben corresponder al arranque definitivo.

## Compilar e inspeccionar

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

Inicia desarrollo o previsualización únicamente cuando quieras trabajar en el navegador local. `docs` posee VitePress y `demo` el paquete web de la aplicación; `site build` los reúne en un sitio estático. El workspace documental conserva su dependencia VitePress separada de Vite 8 de la aplicación. Las órdenes no deben instalar dependencias silenciosamente ni invocar Cargo.

## Conservar el límite de aplicación

`src/web/app/mount-workspace.ts` proporciona la factoría compartida de aplicación/Pinia/router; importarla no monta ni selecciona transporte. El arranque demo explícito aporta `src/web/demo/runtime/transport.ts` al límite `ControlTransport`/cliente de control. Las muestras puras viven en `demo/fixtures/base-host.ts`, independientes de Playwright. `shared/lib/app-storage.ts` y `demo/storage.ts` aíslan almacenamiento para que reiniciar no borre preferencias nativas ni otra instancia demo. Páginas, widgets y funcionalidades deben utilizar sus almacenes y llamadas existentes en `src/web/shared/api`. No crees una segunda aplicación de imitación, no inyectes APIs nativas falsas en el arranque normal ni permitas que una solicitud demo termine en un host real.

Los escenarios deben sembrar registros explícitamente sintéticos y flujos deterministas. Reiniciar debe restaurar el estado inicial, detener temporizadores/suscripciones y limpiar borradores o errores transitorios. Las operaciones no admitidas necesitan un resultado simulado o no disponible, sin inventar éxito de acceso Cloud, ejecución de plugins, audio nativo o integraciones externas.

La entrada `startTour(id, {router, capabilities, taskState})` y el registro `tourRegistry` están en `src/web/features/guided-help/index.ts`. Siete recorridos versionados utilizan objetivos estables y catálogos separados EN/ES/PT/FR. Driver.js carga bajo demanda, limita objetivos al workspace, respeta movimiento reducido, restaura foco y cancela esperas acotadas al navegar. Cerrar una guía nunca completa tareas ni ejecuta acciones de la aplicación.

## Resultado esperado y estado

La demo muestra navegación, edición local, eventos y recuperación que admite su modelo. No valida sesiones TikTok, OBS Browser Source, Wry/comportamiento nativo, backend de producción ni varios equipos. Rankings sigue oculto; Changes de Automations es local, Variables revisadas tiene implementación nativa real, Changes de Overlays es un seguimiento sin integrar y las puertas de instalación automática siguen experimentales.

## Recuperación y verificación

Reinicia el escenario después de simular un hueco o fallo. Si contacta con un servicio externo, detén el recorrido y corrige el transporte. Comprueba contenido, paquete demo y ensamblado por separado; compilar documentación no demuestra corrección nativa. [Pruebas](testing.md) distingue evidencia de host falso, navegador, Rust y aceptación manual.
