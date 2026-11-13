---
title: "TikTools documentation"
docId: site.home
translationKey: site.home
locale: en
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "33094c0616b62ed788f107ae8287e95a8bab6ec7d37f26613f19dee55c5316d0"
translationRevision: "33094c0616b62ed788f107ae8287e95a8bab6ec7d37f26613f19dee55c5316d0"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / Documentation

</template>

<template #language>

[Español](../es/index.md) · [Português](../pt/index.md) · [Français](../fr/index.md)

</template>

<template #intro>

# Your guide to<br />a better LIVE. {#tiktools-documentation}

Connect your TikTok LIVE, make viewer events work for you, and build on TikTools. Practical documentation, from your first connection to your next plugin.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Try the interactive demo</a>
[Download TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Your first LIVE, step by step {#start-here}

Start with a released app. No development tools needed.

1. [**Check the requirements** <span>Find the right setup for your system.</span>](./user/requirements.md)
2. [**Install TikTools** <span>Download and open a released binary.</span>](./user/installation.md)
3. [**Connect to a LIVE** <span>See your first events in the live feed.</span>](./user/first-live.md)

</template>

<template #directory>

## Find your starting point {#explore-the-guides}

Two paths. One shared toolkit.

</template>

<template #user>

### [User guides](./user/index.md) {#user-guides}

Set up your stream, respond to your audience, and make everyday tasks easier. Start here if you use the app.

- [Create your first automation](./user/automations/first-rule.md)
- [Set up OBS and overlays](./user/overlays/index.md)
- [Understand viewer points](./user/points/rules.md)

</template>

<template #developer>

### [Developer guides](./developer/index.md) {#developer-guides}

Work with the Rust host, Vue frontend, and typed control API. Start here if you build or extend the app.

- [Set up your development environment](./developer/setup.md)
- [Explore the architecture and contracts](./developer/architecture.md)
- [Build your first plugin](./developer/plugins/quickstart.md)

</template>

<template #resources>

## Useful along the way {#resources}

- [**UI component reference** <span>17 families with examples and source.</span>](./developer/components/index.md)
- [**Troubleshooting** <span>Find diagnostics and recover from common issues.</span>](./user/preferences/troubleshooting.md)
- [**Release notes** <span>See what changed before you update.</span>](./releases/index.md)

</template>

<template #footer>

The demo uses fictional data. Each guide identifies experimental features and prerequisites.

[Source on GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
