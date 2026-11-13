---
title: "Usar TikTools"
docId: "user.overview"
translationKey: "user.overview"
locale: "es"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6b17801afa3c9d70e222ab513770144e48b85c13ac26ccb82382b3c1ee8a12fa"
translationRevision: "6b17801afa3c9d70e222ab513770144e48b85c13ac26ccb82382b3c1ee8a12fa"
order: 0
---

# Usar TikTools

TikTools conecta un escritorio Windows o Linux a un TikTok LIVE, muestra sus eventos, conserva puntos de espectadores y ejecuta reglas en tu equipo. La interfaz de escritorio y el host opcional sin interfaz comparten el mismo núcleo Rust. La cuenta Cloud es independiente de la conexión con TikTok.

## Elige tu primera tarea

1. Revisa los [requisitos](requirements.md) e [instala un binario publicado](installation.md). Para usar el escritorio no necesitas Rust, Cargo ni Bun.
2. [Conecta con un LIVE](first-live.md), empezando sin cookie de TikTok cuando la sala permita acceso de invitado.
3. Usa **Live feed** para inspeccionar eventos. **Points** conserva totales por espectador; **Analytics** muestra estadísticas. **Automations** relaciona disparadores con acciones configuradas.
4. Instala integraciones opcionales desde **Plugins**. Los paquetes externos se distribuyen por separado de la aplicación.
5. Usa **Overlays** para preparar una Browser Source de OBS. La entrega de widgets en vivo requiere la integración de gateway.

## Funciones disponibles y experimentales

La navegación principal muestra Live feed, Points, Analytics, Connections, Automations, Plugins, Overlays y Settings. Rankings y competencias tienen un motor nativo y una ruta directa, pero su página de gestión sigue oculta en la navegación normal. Los diseños antiguos pueden describir funciones que todavía no están publicadas.

Los cambios de plugins en Cloud y las Variables revisadas usan controles explícitos por sección. Automations compara actualmente contra una referencia local; ese botón no publica perfiles en Cloud. Los transportes de colaboración y algunos procesos de instalación siguen siendo experimentales. Un control deshabilitado suele indicar una capacidad ausente del host o un requisito pendiente.

## Si algo falla

Abre **Diagnostics**, conserva el origen del error y su identificador de solicitud, y anota la versión instalada. Evita incluir cookies, tokens de invitación o valores privados de variables. Empieza por la recuperación de cada guía y después consulta soporte. La demo interactiva usa datos ficticios y no demuestra que funcionen tu cuenta TikTok, OBS o tu red.

Para trabajar con el código fuente, utiliza la [documentación de desarrollo](../developer/index.md).
