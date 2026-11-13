---
title: "Documentación de TikTools"
docId: site.home
translationKey: site.home
locale: es
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "24dadf5def54403406f3d877bac51a422608198ead6489003dbcbfe9b0e60e73"
translationRevision: "24dadf5def54403406f3d877bac51a422608198ead6489003dbcbfe9b0e60e73"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / Documentación

</template>

<template #language>

[English](../en/index.md)

</template>

<template #intro>

# Tu guía para<br />un mejor LIVE. {#tiktools-documentation}

Conecta tu TikTok LIVE, aprovecha los eventos de tu audiencia y crea con TikTools. Documentación práctica, desde tu primera conexión hasta tu próximo plugin.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Prueba la demo interactiva</a>
[Descarga TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Tu primer LIVE, paso a paso {#start-here}

Empieza con la app publicada. No necesitas herramientas de desarrollo.

1. [**Revisa los requisitos** <span>Encuentra la configuración para tu sistema.</span>](./user/requirements.md)
2. [**Instala TikTools** <span>Descarga y abre un binario publicado.</span>](./user/installation.md)
3. [**Conecta un LIVE** <span>Ve los primeros eventos en el feed en vivo.</span>](./user/first-live.md)

</template>

<template #directory>

## Encuentra tu punto de partida {#explore-the-guides}

Dos caminos. Las mismas herramientas.

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

La demo utiliza datos ficticios. Cada guía indica las funciones experimentales y sus requisitos.

[Código en GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
