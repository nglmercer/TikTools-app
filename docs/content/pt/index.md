---
title: "Documentação do TikTools"
docId: site.home
translationKey: site.home
locale: "pt"
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: "en"
sourceRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
translationRevision: "10882fd5a2bfc5b46f0de2ac342d209b809b5a3561e1ae812ef93ec3c05e223d"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / Ferramentas para sua LIVE

</template>


<template #intro>

# Sua LIVE.<br />Suas regras. {#tiktools-documentation}

Conecte sua LIVE do TikTok, automatize ações e crie sobreposições para o OBS. Experimente os controles reais logo abaixo com dados fictícios, sem instalar o aplicativo nem criar uma conta.

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">Experimente o TikTools</a>
[Baixar TikTools](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## Sua primeira LIVE, passo a passo {#start-here}

Instale o aplicativo, conecte um criador e confira seu primeiro evento. Você pode praticar na demonstração antes de começar.

1. [**Confira os requisitos** <span>Encontre a configuração adequada para seu sistema.</span>](./user/requirements.md)
2. [**Instale o TikTools** <span>Baixe e abra um binário publicado.</span>](./user/installation.md)
3. [**Conecte-se a uma LIVE** <span>Veja seus primeiros eventos no feed ao vivo.</span>](./user/first-live.md)

</template>

<template #directory>

## Encontre seu ponto de partida {#explore-the-guides}

Use o aplicativo ou desenvolva suas próprias integrações.

</template>

<template #user>

### [Guias do usuário](./user/index.md) {#user-guides}

Configure sua transmissão, responda ao público e facilite as tarefas do dia a dia. Comece aqui se você usa o aplicativo.

- [Crie sua primeira automação](./user/automations/first-rule.md)
- [Configure OBS e overlays](./user/overlays/index.md)
- [Entenda os pontos dos espectadores](./user/points/rules.md)

</template>

<template #developer>

### [Guias de desenvolvimento](./developer/index.md) {#developer-guides}

Trabalhe com o host Rust, o frontend Vue e a API de controle tipada. Comece aqui se você desenvolve ou amplia o aplicativo.

- [Configure seu ambiente de desenvolvimento](./developer/setup.md)
- [Explore a arquitetura e os contratos](./developer/architecture.md)
- [Crie seu primeiro plugin](./developer/plugins/quickstart.md)

</template>

<template #resources>

## Recursos úteis pelo caminho {#resources}

- [**Referência de componentes de interface** <span>17 famílias com exemplos e código-fonte.</span>](./developer/components/index.md)
- [**Solução de problemas** <span>Encontre diagnósticos e resolva problemas comuns.</span>](./user/preferences/troubleshooting.md)
- [**Notas de versão** <span>Veja o que mudou antes de atualizar.</span>](./releases/index.md)

</template>

<template #footer>

A demonstração usa dados fictícios. Cada guia identifica os recursos experimentais e seus pré-requisitos.

[Código-fonte no GitHub](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
