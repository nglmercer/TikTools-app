---
title: "Desarrollo de plugins"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
translationRevision: "690689449dfe463405cff0ea6b3a33979249bbe68c598ee7e8ca862eedd704dc"
order: 240
---

# Desarrollo de plugins {#plugin-development-overview}

## Objetivo y requisitos {#goal-and-prerequisites}

Elige el límite más pequeño que proporcione tu integración. TikTools descubre paquetes en raíces integradas, de usuario y de desarrollo; añadir un paquete no exige compilar un plugin concreto dentro del host. Usa devkit/SDK compatibles y un perfil privado antes de probar efectos.

## Seguir el recorrido de autoría {#follow-the-authoring-path}

1. Empieza con [inicio rápido](quickstart.md) para crear, validar y compilar un paquete aislado.
2. Elige [runtime](runtimes.md) según dependencias nativas y confianza, no solo por extensión.
3. Declara [permisos e interfaz](permissions-ui.md) para que el host revise efectos y renderice configuración.
4. Usa [empaquetado y registro](packaging-registry.md) para integridad, confianza del editor y actualizaciones.

La autoridad Rust es `crates/tiktools-plugin-api`: validación de manifiestos, protocolo y contratos de interfaz. `crates/tiktools-plugin-sdk` proporciona la API Rust; `packages/plugin-sdk` contiene utilidades TypeScript y el [esquema del editor](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json). La carga corresponde a `crates/tiktools-plugin-loader`, no al frontend.

## Entender las contribuciones {#understand-contributions}

Una acción solicita efectos mediante resultados tipados. Las fuentes publican disparadores; los observadores reciben sobres estables `{topic,data}` con entrega acotada. Los procesadores añaden evidencia sin efectos en `event.intel`, con plazos, combinación determinista y aislamiento de salud. Ajustes/acciones declarativos permiten configuración habitual renderizada por el host. Una página compilada usa un intermediario restringido en su propio marco/ventana; su script nunca integra el runtime Vue principal.

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

Son comprobaciones dirigidas del host/SDK, no validación de cualquier paquete externo.

## Resultado esperado y estado {#expected-result-and-status}

Existen rutas nativa, proceso, declarativa y `napi-vm` sobre QuickJS. WASM es una ranura opcional sin motor general activo en compilaciones normales. Las integraciones de ejemplo viven en [plugins-example-tiktools](https://github.com/nglmercer/plugins-example-tiktools); aparecer en documentos antiguos no las convierte en funciones integradas.

## Recuperación {#recovery}

Si falla el descubrimiento, revisa compatibilidad, precedencia de raíces, destino y entrada. Si se deniega una intención, comprueba capacidades declaradas y concedidas sin eludir al host. Un inicio incierto exige reconciliación antes de sustituirlo. Los límites nativo/proceso mantienen confianza a nivel del sistema; el nombre del runtime no demuestra aislamiento.
