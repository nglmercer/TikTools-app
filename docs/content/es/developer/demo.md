---
title: "Arquitectura y límites de la demo web"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "es"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e7f0ab271cce7a8c6e78d9d73c38ae0da022cefe2db104b1001a8f45cf8754c"
translationRevision: "b874d7bbdae72627d4394ee473ceb799d092b6b784a007e058c2774e5d21a97c"
order: 140
---

# Arquitectura y límites de la demo web {#browser-demo-architecture-and-limits}

## Objetivo y requisitos {#goal-and-prerequisites}

Explora la interfaz real con datos sintéticos en un navegador. Usa Bun 1.4.4 y la instalación bloqueada del repositorio. La entrada explícita de la demo funciona sin Rust, cuentas ni host activo. Su runtime y las pruebas de navegador forman parte del repositorio.

## Compilar e inspeccionar {#build-and-inspect}

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

Para previsualizar cambios sin commit usa `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build`. El manifiesto se marca como dirty y publicación lo rechaza. El ensamblado final exige el mismo SHA Git limpio y base en ambos paquetes; vuelve a compilar después del commit. Inicia desarrollo o previsualización únicamente cuando quieras trabajar en el navegador local. `docs` posee VitePress y `demo` el paquete web de la aplicación; `site build` los reúne en un sitio estático. El workspace documental conserva su dependencia VitePress separada de Vite 8 de la aplicación. Las órdenes no deben instalar dependencias silenciosamente ni invocar Cargo.

## Conservar el límite de aplicación {#preserve-the-application-boundary}

`src/web/app/mount-workspace.ts` proporciona la factoría compartida de aplicación/Pinia/router; importarla no monta ni selecciona transporte. El arranque demo explícito aporta `src/web/demo/runtime/transport.ts` al límite `ControlTransport`/cliente de control. Las muestras puras viven en `demo/fixtures/base-host.ts`, independientes de Playwright. `shared/lib/app-storage.ts` y `demo/storage.ts` aíslan almacenamiento para que reiniciar no borre preferencias nativas ni otra instancia demo. Páginas, widgets y funcionalidades deben utilizar sus almacenes y llamadas existentes en `src/web/shared/api`. No crees una segunda aplicación de imitación, no inyectes APIs nativas falsas en el arranque normal ni permitas que una solicitud demo termine en un host real.

`demo/scenarios.ts` valida 11 escenarios y las rutas, idiomas y recorridos admitidos. `runtime/host.ts` registra handlers explícitos: discovery anuncia esos handlers, los IDs se conservan en `rpc-response` y los efectos no admitidos devuelven error inmediatamente. El host prepara datos ficticios y avanza un reloj fijo al pulsar Siguiente evento. Reproducir y Siguiente evento conectan primero al creador ficticio mediante el RPC simulado si hace falta y generan un evento inmediato. Reproducir continúa cada dos segundos; Pausar detiene la producción. La barra muestra el estado y el contador de eventos. El flujo termina a los 300 pasos y conserva hasta 60 eventos y 60 ejecuciones. Las colas de respuestas y registros tienen límites independientes.

Reiniciar limpia únicamente el prefijo `tiktools.demo.<instance>.*` y recarga el iframe. Recargar reinicializa singletons, journals, i18n y diagnósticos. Cada iframe recibe un identificador distinto. Al salir se desmonta Vue, se liberan stores, cliente y transporte, y se detienen ambos temporizadores. La demo no instala `window.ipc` ni realiza HTTP arbitrario, shell, RCON, ejecución de código visitante o actualización nativa. Su CSP rechaza conexiones de red. Las plantillas de log y filtros simulados usan funciones puras compartidas: representan el contrato del frontend y no sustituyen la ejecución Rust.

El parámetro `embed=connection|feed|automation|points|overlays` selecciona una vista incrustada enfocada. Oculta la navegación general y conserva las páginas y el runtime reales; utiliza la ruta correspondiente al panel, sin iniciar un recorrido guiado. La vista de conexión permite practicar los controles sin ocupar el artículo con todo el espacio de trabajo. El modo incrustado mantiene el mismo límite de transporte simulado y el aislamiento de almacenamiento de la demo completa.

La entrada `startTour(id, {router, capabilities, taskState})` y el registro `tourRegistry` están en `src/web/features/guided-help/index.ts`. Siete recorridos versionados utilizan objetivos estables y catálogos separados para cada idioma admitido. Driver.js carga bajo demanda, limita objetivos al workspace, respeta movimiento reducido, restaura foco y cancela esperas acotadas al navegar. Cerrar una guía nunca completa tareas ni ejecuta acciones de la aplicación. El selector nombra la tarea completa, prepara su escenario y pausa la reproducción antes de iniciar. El cierre de Driver descarta la guía y la barra muestra el resultado. Reiniciar limpia el progreso versionado de esta instancia.

## Resultado esperado y estado {#expected-result-and-status}

La demo muestra navegación, edición local, eventos y recuperación que admite su modelo. No valida sesiones TikTok, OBS Browser Source, Wry/comportamiento nativo, backend de producción ni varios equipos. Rankings sigue oculto; Changes de Automations es local, Variables revisadas tiene implementación nativa real, Changes de Overlays es un seguimiento sin integrar y las puertas de instalación automática siguen experimentales.

## Componentes y Markdown {#components-and-markdown}

`docs/examples/*.vue` aporta el catálogo ejecutable y sus snippets. Los botones pequeños se montan directamente en Markdown sin estilos globales del escritorio. Los diálogos, layouts y páginas completas usan `DemoFrame`, que valida parámetros, respeta el base y carga un iframe solo bajo demanda. La opción pública `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` expone el catálogo sin activar todas las herramientas de desarrollo del escritorio.

Para insertar un panel real junto a una instrucción, declara el escenario, idioma, panel y un título descriptivo:

```vue
<DemoFrame scenario="connection" locale="es" panel="connection" title="Practica tu primera conexión LIVE" />
```

`panel` admite `connection`, `feed`, `automation`, `points` y `overlays` y selecciona el modo `embed` correspondiente. Antes de interactuar se muestra una vista previa sin cargar el iframe. El botón para probar el ejemplo carga su instancia aislada; reiniciar descarta únicamente el estado de esa instancia. **Abrir demo completa** sale del modo incrustado y recupera la navegación general. Coloca cada ejemplo junto al paso que explica y conserva el texto necesario para comprender la tarea sin cargarlo.

## Recuperación y verificación {#recovery-and-verification}

Reinicia el escenario después de simular un hueco o fallo. Si contacta con un servicio externo, detén el recorrido y corrige el transporte. Comprueba contenido, paquete demo y ensamblado por separado; compilar documentación no demuestra corrección nativa. [Pruebas](testing.md) distingue evidencia de host falso, navegador, Rust y aceptación manual.
