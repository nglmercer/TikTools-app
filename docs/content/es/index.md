---
title: "Documentación de TikTools"
docId: site.home
translationKey: site.home
locale: es
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
translationRevision: "0b886028d9dcc8270a00598d321fa643840439d830ee50660707f8f87d630856"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / Para tu próximo LIVE

</template>


<template #intro>

# Tu LIVE.<br />Tus reglas. {#tiktools-documentation}

Conecta tu TikTok LIVE, automatiza las respuestas a tu audiencia y crea overlays para OBS. Prueba aquí la interfaz real con datos ficticios y prepara tu próxima transmisión.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Prueba TikTools</a>
[Descarga TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Tu primer LIVE, paso a paso {#start-here}

Explora la aplicación aquí mismo o instálala para conectar tu LIVE. No necesitas herramientas de desarrollo.

1. [**Revisa los requisitos** <span>Encuentra la configuración para tu sistema.</span>](./user/requirements.md)
2. [**Instala TikTools** <span>Descarga y abre un binario publicado.</span>](./user/installation.md)
3. [**Conecta un LIVE** <span>Ve los primeros eventos en el feed en vivo.</span>](./user/first-live.md)

</template>

<template #directory>

## Encuentra tu punto de partida {#explore-the-guides}

Guías para usar TikTools y referencias para ampliarlo.

</template>

<template #user>

### [Guías de usuario](./user/index.md) {#user-guides}

Prepara tu transmisión, responde a tu audiencia y simplifica las tareas diarias. Empieza aquí si utilizas la app.

- [Crea tu primera automatización](./user/automations/first-rule.md)
- [Configura OBS y overlays](./user/overlays/index.md)
- [Conoce los puntos de espectadores](./user/points/rules.md)

</template>

<template #developer>

### [Guías de desarrollo](./developer/index.md) {#developer-guides}

Trabaja con el host Rust, la interfaz Vue y la API de control tipada. Empieza aquí si desarrollas o amplías la app.

- [Prepara tu entorno de desarrollo](./developer/setup.md)
- [Explora la arquitectura y los contratos](./developer/architecture.md)
- [Crea tu primer plugin](./developer/plugins/quickstart.md)

</template>

<template #resources>

## Recursos para el camino {#resources}

- [**Referencia de componentes UI** <span>17 familias con ejemplos y código fuente.</span>](./developer/components/index.md)
- [**Solución de problemas** <span>Consulta diagnósticos y resuelve errores comunes.</span>](./user/preferences/troubleshooting.md)
- [**Notas de versiones** <span>Revisa los cambios antes de actualizar.</span>](./releases/index.md)

</template>

<template #footer>

Los ejemplos usan datos ficticios y guardan sus cambios dentro de la demo. Cada guía indica los requisitos y las funciones experimentales.

[Código en GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
