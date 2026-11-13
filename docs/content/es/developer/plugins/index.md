---
title: "Desarrollo de plugins"
docId: "developer.plugins.index"
translationKey: "developer.plugins.index"
locale: "es"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "556fd53e0dfac53e6fa1a4c39dc3896eafcb9fd67fe46e63b7b1da7c7396806b"
translationRevision: "556fd53e0dfac53e6fa1a4c39dc3896eafcb9fd67fe46e63b7b1da7c7396806b"
order: 240
---

# Desarrollo de plugins

## Objetivo y requisitos

Elige el límite más pequeño que proporcione tu integración. TikTools descubre paquetes en raíces integradas, de usuario y de desarrollo; añadir un paquete no exige compilar un plugin concreto dentro del host. Usa devkit/SDK compatibles y un perfil privado antes de probar efectos.

## Seguir el recorrido de autoría

1. Empieza con [inicio rápido](quickstart.md) para crear, validar y compilar un paquete aislado.
2. Elige [runtime](runtimes.md) según dependencias nativas y confianza, no solo por extensión.
3. Declara [permisos e interfaz](permissions-ui.md) para que el host revise efectos y renderice configuración.
4. Usa [empaquetado y registro](packaging-registry.md) para integridad, confianza del editor y actualizaciones.

La autoridad Rust es `crates/tiktools-plugin-api`: validación de manifiestos, protocolo y contratos de interfaz. `crates/tiktools-plugin-sdk` proporciona la API Rust; `packages/plugin-sdk` contiene utilidades TypeScript y el [esquema del editor](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json). La carga corresponde a `crates/tiktools-plugin-loader`, no al frontend.

## Entender las contribuciones

Una acción solicita efectos mediante resultados tipados. Las fuentes publican disparadores; los observadores reciben sobres estables `{topic,data}` con entrega acotada. Los procesadores añaden evidencia sin efectos en `event.intel`, con plazos, combinación determinista y aislamiento de salud. Ajustes/acciones declarativos permiten configuración habitual renderizada por el host. Una página compilada usa un intermediario restringido en su propio marco/ventana; su script nunca integra el runtime Vue principal.

```sh
bun scripts/task.ts test bun packages
bun scripts/task.ts rust test -p tiktools-plugin-api --locked
```

Son comprobaciones dirigidas del host/SDK, no validación de cualquier paquete externo.

## Resultado esperado y estado

Existen rutas nativa, proceso, declarativa y `napi-vm` sobre QuickJS. WASM es una ranura opcional sin motor general activo en compilaciones normales. Las integraciones de ejemplo viven en [plugins-example-tiktools](https://github.com/nglmercer/plugins-example-tiktools); aparecer en documentos antiguos no las convierte en funciones integradas.

## Recuperación

Si falla el descubrimiento, revisa compatibilidad, precedencia de raíces, destino y entrada. Si se deniega una intención, comprueba capacidades declaradas y concedidas sin eludir al host. Un inicio incierto exige reconciliación antes de sustituirlo. Los límites nativo/proceso mantienen confianza a nivel del sistema; el nombre del runtime no demuestra aislamiento.
