---
title: "Documentation TikTools"
docId: site.home
translationKey: site.home
locale: fr
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
translationRevision: "430265d011c8fccab96c1de23a20513d0d13a0013f2859652221a5213a0e5b4e"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / Outils pour vos LIVE

</template>


<template #intro>

# Votre LIVE.<br />Vos règles. {#tiktools-documentation}

Connectez TikTok LIVE, transformez les événements de votre audience en automatisations et créez des incrustations pour OBS. Essayez l’interface réelle ci-dessous avec des données fictives, puis composez votre propre installation de diffusion.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Essayer TikTools</a>
[Télécharger TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Passez à votre prochain LIVE {#start-here}

Prêt à diffuser ? Installez TikTools pour Windows ou Linux, connectez un créateur et découvrez les premiers événements de votre audience.

1. [**Vérifier les prérequis** <span>Trouvez la configuration adaptée à votre système.</span>](./user/requirements.md)
2. [**Installer TikTools** <span>Téléchargez et ouvrez un binaire publié.</span>](./user/installation.md)
3. [**Se connecter à un LIVE** <span>Consultez vos premiers événements dans le flux en direct.</span>](./user/first-live.md)

</template>

<template #directory>

## Créez une configuration à votre image {#explore-the-guides}

Faites fonctionner votre installation ou enrichissez-la avec vos propres plugins.

</template>

<template #user>

### [Guides d’utilisation](./user/index.md) {#user-guides}

Connectez votre diffusion, récompensez vos spectateurs et automatisez les moments qui comptent. Suivez chaque parcours du premier clic jusqu’à un résultat que vous pouvez vérifier.

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

## Tout pour continuer {#resources}

- [**Référence des composants d’interface** <span>17 familles avec exemples interactifs, états et code source.</span>](./developer/components/index.md)
- [**Dépannage** <span>Consultez les diagnostics et résolvez les problèmes courants.</span>](./user/preferences/troubleshooting.md)
- [**Notes de version** <span>Consultez les changements avant la mise à jour.</span>](./releases/index.md)

</template>

<template #footer>

La démo dans le navigateur utilise la véritable interface TikTools avec des données fictives. Aucune installation ni aucun compte ne sont nécessaires pour l’essayer. Les guides couvrent les prérequis de votre installation réelle.

[Code source sur GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
