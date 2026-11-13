---
title: "Documentation TikTools"
docId: site.home
translationKey: site.home
locale: fr
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "cc1465f0b5da5cbbd2ff9782ed1163f9a7f793b3fedc837340d3896e3da9c990"
translationRevision: "2b499416d56fe7c0f57af76955293598365918abc54b09e3185de4b495c6e9d3"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / Documentation

</template>


<template #intro>

# Votre guide pour<br />un meilleur LIVE. {#tiktools-documentation}

Connectez votre LIVE, découvrez les événements de votre audience et transformez-les en actions. Apprenez avec des exemples de l’interface réelle, puis appliquez chaque parcours à votre prochaine diffusion.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Essayer la démo interactive</a>
[Télécharger TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Votre premier LIVE, étape par étape {#start-here}

Installez une version publiée, connectez un créateur et vérifiez votre premier événement. Essayez les commandes avec des données fictives à chaque étape.

1. [**Vérifier les prérequis** <span>Trouvez la configuration adaptée à votre système.</span>](./user/requirements.md)
2. [**Installer TikTools** <span>Téléchargez et ouvrez un binaire publié.</span>](./user/installation.md)
3. [**Se connecter à un LIVE** <span>Consultez vos premiers événements dans le flux en direct.</span>](./user/first-live.md)

</template>

<template #directory>

## Trouvez votre point de départ {#explore-the-guides}

Choisissez un parcours pour utiliser l’application, ou découvrez les outils qui permettent de l’étendre.

</template>

<template #user>

### [Guides d’utilisation](./user/index.md) {#user-guides}

Préparez votre diffusion, réagissez à votre audience et simplifiez les tâches quotidiennes. Commencez ici si vous utilisez l’application.

- [Créer votre première automatisation](./user/automations/first-rule.md)
- [Configurer OBS et les incrustations](./user/overlays/index.md)
- [Comprendre les points des spectateurs](./user/points/rules.md)

</template>

<template #developer>

### [Guides de développement](./developer/index.md) {#developer-guides}

Travaillez avec l’hôte Rust, l’interface Vue et l’API de contrôle typée. Commencez ici si vous développez ou étendez l’application.

- [Préparer votre environnement de développement](./developer/setup.md)
- [Explorer l’architecture et les contrats](./developer/architecture.md)
- [Créer votre premier plugin](./developer/plugins/quickstart.md)

</template>

<template #resources>

## Ressources utiles en chemin {#resources}

- [**Référence des composants d’interface** <span>17 familles avec exemples interactifs, états et code source.</span>](./developer/components/index.md)
- [**Dépannage** <span>Consultez les diagnostics et résolvez les problèmes courants.</span>](./user/preferences/troubleshooting.md)
- [**Notes de version** <span>Consultez les changements avant la mise à jour.</span>](./releases/index.md)

</template>

<template #footer>

La démo utilise des données fictives. Chaque guide précise les fonctionnalités expérimentales et les prérequis.

[Code source sur GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
