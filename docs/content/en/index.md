---
title: "TikTools documentation"
docId: site.home
translationKey: site.home
locale: en
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: en
sourceRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
translationRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / LIVE toolkit

</template>


<template #intro>

# Your LIVE.<br />Your rules. {#tiktools-documentation}

Connect TikTok LIVE, turn audience events into automations, and design overlays for OBS. Try the real interface below with fictional data, then build your own broadcast setup.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Try TikTools</a>
[Download TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Take it to your next LIVE {#start-here}

Ready for your broadcast? Install TikTools for Windows or Linux, connect a creator, and watch your first audience events arrive.

1. [**Check the requirements** <span>Find the right setup for your system.</span>](./user/requirements.md)
2. [**Install TikTools** <span>Download and open a released binary.</span>](./user/installation.md)
3. [**Connect to a LIVE** <span>See your first events in the live feed.</span>](./user/first-live.md)

</template>

<template #directory>

## Make it yours {#explore-the-guides}

Get your setup working, or extend it with your own plugins.

</template>

<template #user>

### [User guides](./user/index.md) {#user-guides}

Connect your stream, reward your viewers, and automate the moments that matter. Follow a workflow from its first click to a result you can check.

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

## Everything you need to keep going {#resources}

- [**UI component reference** <span>17 families with interactive examples, states and source.</span>](./developer/components/index.md)
- [**Troubleshooting** <span>Find diagnostics and recover from common issues.</span>](./user/preferences/troubleshooting.md)
- [**Release notes** <span>See what changed before you update.</span>](./releases/index.md)

</template>

<template #footer>

The browser demo runs the real TikTools interface with fictional data. No installation or account is needed to try it. Guides cover the requirements for your real setup.

[Source on GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
