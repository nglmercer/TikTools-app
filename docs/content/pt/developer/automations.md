---
title: "Execução de automações e contratos de scripts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
translationRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
order: 150
---

# Execução de automações e contratos de scripts {#automation-execution-and-script-contracts}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Implemente uma automação usando descritores do host e contratos gerados de eventos. Trabalhe em um perfil privado com efeitos externos desativados até testá-los intencionalmente. Entenda a [normalização de eventos](events.md) e os [metadados de controle](control-api.md); o editor Vue configura a execução sob responsabilidade do Rust.

## Crie uma regra mínima {#build-a-minimal-rule}

Uma ação de comportamento armazena um tipo de ação configurado. Um evento de comportamento seleciona acionador, filtros, intervalo de espera e IDs de ações. Grafos de fluxo, por sua vez, compõem nós e portas versionados. Descritores integrados vêm do núcleo; descritores de plugins vêm dos manifestos instalados. Nunca fixe um segundo catálogo de ações no código do editor.

1. Selecione um acionador emitido de `src/automation/contracts/generated/event-registry.generated.ts`.
2. Use sua amostra para escolher um campo pontuado real, como `event.user.uniqueId` ou `event.data.giftName`.
3. Comece com registro ou uma ação desativada e depois teste filtro e espera com um evento sanitizado.
4. Salve pelo coordenador nativo responsável, mantendo a candidata do editor e as versões revisadas quando exigidas.

```sh
bun scripts/task.ts contracts
bun scripts/task.ts contracts --check
bun scripts/task.ts test bun automation
```

O [diretório de contratos gerados](https://github.com/nglmercer/TikTools-app/tree/remake/src/automation/contracts/generated) define tipos de eventos do frontend, metadados de registro e amostras. Amplie os contratos Rust e regenere junto ao expor outro campo.

## Limite dos scripts {#script-boundary}

`core.code` e nós de script de fluxos executam QuickJS com limites. Eles recebem JSON `event`, `inputs` e `data` (o alias de inputs). Não há carregador Node, sistema de arquivos, rede, WebView nem objeto de banco de dados. Mantenha transformações determinísticas e serializáveis em JSON; use ações configuradas do host para efeitos de HTTP, pontos, áudio ou plugins. Limites de código/resultado e orçamentos de execução pertencem ao host, não a um tempo limite do navegador.

## Resultado esperado e status {#expected-result-and-status}

Regras salvas executam pelo runtime do núcleo e seu intermediador de capacidades. Processadores podem adicionar evidências em `event.intel` sem substituir campos brutos. Changes de Automations é uma área de revisão local, não sincronização de configuração da conta inteira. Variables revisadas têm implementação nativa real; Changes de Overlays é uma continuação separada ainda não mesclada.

## Recuperação {#recovery}

Em um conflito, compare os registros atuais sem perder a candidata. Um rascunho remoto permanece desativado enquanto aguarda revisão do proprietário e transferência de autoridade; inspecionar prévia/manifesto sozinho não autoriza execução. Resultados desconhecidos de efeitos externos exigem o fluxo de inspeção/recuperação do efeito mantido. Reproduzir o histórico da interface nunca deve disparar a automação novamente. Use [modelos](templates.md) para importação controlada de biblioteca e [testes](testing.md) para evidências específicas de cada limite.
