---
title: "Desarrollar TikTools"
docId: "developer.overview"
translationKey: "developer.overview"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1ee65a70172f8154eeaf60fb1a3069192769582a4ef264426d0aefaf346e8003"
translationRevision: "dce8233794e6aa3dcf805b91d9f881a553106387596b68b46a74270f571e9ff8"
order: 0
---

# Desarrollar TikTools {#develop-tiktools}

TikTools mantiene un único AppCore nativo detrás de operaciones de control tipadas. Vue representa las interfaces de escritorio, navegador local y controller; Winit/Wry administra las ventanas nativas. CLI y SDK utilizan las mismas operaciones. Empieza por la [preparación de desarrollo](setup.md) y elige el ciclo mínimo que cubra tu cambio.

## Elige una entrada {#choose-an-entry-point}

| Trabajo | Entrada | Qué comprueba |
| --- | --- | --- |
| Componentes Vue y stores | Vite, Vitest y ControlClient inyectado | Comportamiento de interfaz; no valida ventanas nativas. |
| Integración de escritorio | `bun run start` | WebView de desarrollo e integración IPC. |
| Servicios nativos | Tests Cargo enfocados | Contratos del núcleo, persistencia y servicios. |
| Acceso en navegador local | `bun run dev` | Host nativo, adaptador local y Vite con emparejamiento. |
| Documentación | `bun scripts/task.ts docs dev` | Markdown y build aislado de VitePress. |
| Demo pública | Entrada propia y transporte simulado | Escenarios ficticios deterministas, no servicios externos. |

## Responsabilidad del código {#source-ownership}

- `crates/tiktools-core`: servicios, operaciones y SQLite. No depende de una WebView.
- `crates/tiktools-control-api`: registro, JSON-RPC, IPC y descubrimiento.
- `crates/tiktools-desktop`: Winit/Wry, puente del hilo de interfaz, ciclo de vida y recursos empaquetados.
- `src/web/app`: arranque, router y shell. `pages` compone `widgets` y `features`; `entities` contiene stores y contratos de dominio; `shared` reúne interfaz e infraestructura reutilizables.
- `src/automation/contracts/generated`: contratos de eventos versionados para el frontend, generados desde Rust. No vuelvas a escribir esos schemas en la demo o las guías.
- `packages`: SDK y componentes TypeScript existentes para plugins. Sus implementaciones viven en el repositorio externo de plugins.

## Límites de contribución {#contribution-boundary}

Mantén explícitas las entradas nativa, de navegador local, controller remoto y demo. La ausencia del host nativo debe producir un error en una entrada real; nunca debe activar un mock silenciosamente. Libera listeners, suscripciones y temporizadores con su scope propietario. Conserva los lockfiles de Rust/Bun y los checks obligatorios de CI.

La instalación para usuarios pertenece a su [guía](../user/installation.md). Estos checks de desarrollo no prueban por sí solos conectividad real con TikTok, representación en OBS, compatibilidad de la WebView nativa o colaboración entre dos equipos.
