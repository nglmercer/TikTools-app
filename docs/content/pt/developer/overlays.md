---
title: "SDK de widgets, gateway e limites de overlays"
docId: "developer.overlays"
translationKey: "developer.overlays"
locale: "pt"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "99d3351859ddb3a3304133f062f68b8e3130416947fc40433dc9c6bffaf85f6a"
translationRevision: "172243dd57089b19a14a3aaae7bf9ab8f24a9e40d9c77e5ad26930a9519bbcd5"
order: 170
---

# SDK de widgets, gateway e limites de overlays {#widget-sdk-gateway-and-overlay-boundaries}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Crie um widget que compartilhe renderização entre prévia local e saída do OBS. Use as ferramentas Vue instaladas e entenda os [eventos](events.md). A prévia local não precisa de gateway nem credencial de conta. A saída ao vivo de widgets exige o plugin event-gateway instalado separadamente e sua credencial com escopo de widget.

## Adicione um modelo {#add-a-template}

1. Defina um renderizador dentro de `WidgetStage`; dimensione-o em relação ao elemento pai em vez da área de visualização do navegador.
2. Use `defineWidget` de `src/widgets/sdk/template.ts`. Forneça seu esquema, envelopes de amostra novos e a fábrica `create(search, clock)`.
3. Retorne um controlador que implemente `handleEnvelope`, `clear` e `dispose`, além da renderização. Use o relógio fornecido para que prévias possam manter amostras e testes possam avançar o tempo.
4. Registre-o em `sdk/templates.ts` e adicione a entrada independente, configuração de compilação, lista de rotas permitidas do gateway e entrada de empacotamento. O registro sozinho não expõe uma URL pública.
5. Use o mesmo `WidgetHost.vue` no editor e no bundle independente; não incorpore HTML do gateway para uma prévia local.

```sh
bun scripts/task.ts widgets
bun scripts/task.ts test bun widgets
```

Mantenha tokens de texto, padrões de design e campos editáveis no esquema do modelo. Designs são serializados como JSON e URLs copiadas de widgets carregam um instantâneo validado; um salvamento posterior exige copiar uma URL atualizada para o OBS.

## Protocolo do gateway {#gateway-protocol}

`src/widgets/shared/gateway-client.ts` é responsável por autenticação, assinaturas, heartbeat, reconexão e relato de lacunas. Widgets consomem envelopes canônicos de domínio, não o TikTok diretamente. O endpoint de widgets é `/ws/widgets`; ele difere da superfície completa de cliente externo do gateway. Analise envelopes pelo analisador compartilhado e preserve limites de credenciais em vez de passar um token completo do gateway pela WebView principal.

## Resultado esperado e status {#expected-result-and-status}

Prévias locais integradas e o SDK de widgets estão implementados. Edição de overlays do Cloud e código de `cloud-overlay` existem, mas validação real no OBS, cobertura de imagens/fontes ao vivo e aplicação de pacotes mantêm limites. Changes de Overlays é uma continuação ainda não mesclada. Rankings permanece oculto na navegação normal do aplicativo mesmo com código de widget de ranking existente.

## Recuperação {#recovery}

Para uma prévia local em branco, inspecione tempo de vida do controlador, padrões do esquema e dimensionamento do palco antes de enfraquecer a política de origem. Para saída ao vivo, inspecione integridade do gateway, credencial do widget e lacunas explícitas. Um teste Chromium sem interface gráfica não comprova aceitação da fonte de navegador do OBS. O [SDK de widgets](https://github.com/nglmercer/TikTools-app/tree/remake/src/widgets/sdk) e os testes de protocolo compartilhado são as referências atuais da implementação.

## Detalhes de operação {#operational-details}

A suíte unitária usa fixtures; Chromium com gateway real exige widgets construídos e checkout externo via `TIKTOOLS_PLUGINS_REPO`. Sem fonte ela é omitida: informe skip. Execute `bun x --no-install playwright test --config src/widgets/playwright.gateway.config.ts` para esse limite. Depois confira OBS real, credencial widget, entrega e recuperação com LIVE; Chromium headless não comprova aceitação OBS/nativa.
