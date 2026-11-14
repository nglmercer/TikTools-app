---
title: "Perguntas frequentes"
docId: "user.preferences.faq"
translationKey: "user.preferences.faq"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
translationRevision: "34accedc262f929fb125828a38ac205193d9a3241cff94302eadf2347ce608ad"
order: 40
---

# Perguntas frequentes {#frequently-asked-questions}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Resolva confusões comuns de fluxo antes de alterar dados ou credenciais. Você precisa do aplicativo de desktop em execução para as verificações abaixo; [requisitos](../requirements.md) descreve os pacotes compatíveis. Os recursos locais discutidos aqui estão disponíveis, salvo quando houver um limite experimental ou dependente de serviço declarado.

## O login no Cloud conecta o TikTok? {#does-cloud-sign-in-connect-tiktok}

Não. Abra [Connections](../live/connections.md), escolha um criador e inicie a LIVE separadamente. A identidade do Cloud dá suporte a serviços baseados em conta. Um cookie do TikTok pertence apenas ao campo mascarado de conexão destinado a ele.

## Por que um feed conectado está vazio? {#why-is-a-connected-feed-empty}

Uma sala silenciosa pode não ter eventos novos. Escolha todos os eventos, limpe a pesquisa e retome a rolagem, depois inspecione o status da conexão. Pausar a rolagem mantém ingestão e automações em execução. Limpar o feed não redefine pontos nem análises persistidas.

## As estatísticas são apenas desta sessão? {#are-statistics-only-for-this-session}

A [página Analytics](../points/statistics.md) atual consulta agregados persistidos do criador e intervalo de datas selecionados. Amplie as datas e atualize para inspecionar o histórico registrado. Não é uma transcrição completa do chat. Saldos de pontos usam regras diferentes e podem incluir ajustes manuais ou de automações.

## Onde ficam modelos, variáveis e rankings? {#where-are-templates-variables-and-rankings}

Modelos e perfis ficam em [Automations](../automations/templates.md); Explore de Plugins solicita plugins. Valores de execução ficam em Variables de Automations. A área experimental Rankings ainda tem `#/rankings`, mas fica oculta na navegação principal e na restauração inicial. Classificações comuns de pontos e painéis de ranking do feed continuam sendo visualizações separadas.

## Salvar sincroniza tudo? {#does-saving-synchronize-everything}

Não. Changes de Events/Actions compara uma referência local. Salvar Variables permanece local, com operações separadas e revisadas de enviar/adotar/retirar quando compatíveis. Comparações de segredos são mascaradas. Overlays do Cloud usam criação/revisão e operações de biblioteca; esta versão não tem adaptador compartilhado Changes de overlays. Leia [Changes](../cloud/changes.md) antes de escolher uma direção.

## Os testes são inofensivos e as importações executam imediatamente? {#are-tests-harmless-and-do-imports-run-immediately}

Testes podem executar efeitos, incluindo áudio, requisições de rede e alterações de pontos. Comece com [core.log](../automations/first-rule.md). Definições importadas permanecem desativadas; revise e ative explicitamente. Instalar um plugin também exige configurar seu serviço antes de suas ações funcionarem.

## Resultado esperado e recuperação {#expected-result-and-recovery}

Essas verificações devem identificar a página responsável sem uma redefinição. Verificações automáticas de atualização são opcionais; a instalação é experimental e está desativada pelos controles de lançamento atuais. Colaboração remota completa e busca de apelidos em produção ainda dependem de implantação e aceitação. Se a dúvida persistir, preserve seus dados por [backups](backups.md) e colete um [relatório de diagnóstico](troubleshooting.md) com versão, horário e reprodução.
