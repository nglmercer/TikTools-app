---
title: "Capas y flujo de datos del frontend"
docId: "developer.frontend"
translationKey: "developer.frontend"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
translationRevision: "289f2dc81c1eebfe9d93378ec7ca3dcf67413524a86f032651762e1f0b116a61"
order: 40
---

# Capas y flujo de datos del frontend {#frontend-layers-and-data-flow}

## Objetivo y requisitos {#goal-and-prerequisites}

Amplía la aplicación Vue sin acoplar una página al IPC nativo. El frontend utiliza Vue 3.5, Pinia 4, Vue Router 5 y vue-i18n 11 según los rangos del workspace. Conserva las dependencias bloqueadas y utiliza las órdenes de [preparación](setup.md) en lugar de crear otro proyecto frontend.

## Encontrar la capa propietaria {#find-the-owning-layer}

La organización actual de Feature-Sliced Design está en `src/web/`:

| Capa | Responsabilidad | Ejemplo existente |
| --- | --- | --- |
| `app` | Arranque, rutas, contenedor y vida del runtime | `app/model/use-app-runtime.ts` |
| `pages` | Composición de rutas | `pages/automations/ui/AutomationsPage.vue` |
| `widgets` | Paneles compuestos de mayor tamaño | `widgets/live-feed/ui/LiveFeedPanel.vue` |
| `features` | Operaciones del usuario y sus borradores | `features/apply-rule-template/` |
| `entities` | Almacenes y lógica reutilizable del dominio | `entities/rule-template/` |
| `shared` | API, componentes básicos, i18n y utilidades | `shared/api/control-client.ts` |

Compón las capas superiores con las inferiores. Las llamadas al host pasan por el cliente compartido; la selección del transporte pertenece al arranque. Las rutas `src/web/platform` y `src/web/views` de artículos antiguos son históricas.

## Implementar un cambio concreto {#implement-a-focused-change}

1. Localiza la ruta en `app/router/routes.ts` y sigue su página hasta la funcionalidad y el almacén de entidad.
2. Usa componentes compartidos existentes para campos, estados y diálogos. Conserva etiquetas, foco y contrato de eventos.
3. Separa los candidatos editables de los registros guardados. `shared/lib/request-scope.ts` permite ignorar respuestas obsoletas; invalidar no cancela una mutación aceptada por el host.
4. Añade claves de idioma mediante `shared/i18n` y libera las suscripciones al terminar su propietario.

```sh
bun scripts/task.ts test web rule-profiles
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-build
```

## Resultado esperado, estado y recuperación {#expected-result-status-and-recovery}

La ruta muestra datos actuales y conserva los cambios sin guardar ante conflictos. Rankings tiene código y ruta, pero sigue oculto en la navegación normal; la presencia de un componente no demuestra disponibilidad del producto. Los arranques nativo, de navegador local y de demostración pueden proporcionar transportes distintos al mismo código funcional.

Si una respuesta antigua reemplaza una consulta reciente, revisa su generación antes de cambiar el backend. Cuando desaparece una página de plugin, vuelve a los plugins instalados, como hace el runtime. El [runtime de la aplicación](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/app/model/use-app-runtime.ts) es la referencia para conexión, resincronización y liberación.
