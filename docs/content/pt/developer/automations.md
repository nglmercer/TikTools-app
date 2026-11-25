---
title: "Execução de automações e contratos de scripts"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "pt"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3e39c947b170f92475e3ccbbcd579c67f16a5d241954285c740aaca66e196bd3"
translationRevision: "b4daeff977a79f94bea7e42c1588d7e00a52ff9bdec31aa73030b239de11f329"
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

## Verificação pela CLI {#cli-verification}

Com um host local em execução, a CLI comunica-se por IPC. Um agente pode verificar uma regra sem tocar no chat ao vivo.

```sh
target/debug/tiktools automation list --kind event
target/debug/tiktools automation get <id>
target/debug/tiktools automation context --event-type tiktok.chat
```

Um teste a seco avalia os filtros e indica o que cada ação faria, sem executar nada. A resposta inclui `eventSource` (`live`, `sample`, `custom`), `eventData` (o payload efetivamente usado) e `pinned` (valores copiados dos filtros para a amostra).

Os filtros são o contrato em `crates/tiktools-core/src/services/automation/filters.rs`, espelhado para testes por `src/automation/behavior/filter-simulation.ts` e fixado caso a caso por `tests/fixtures/automation-filters/filter-corpus.json`. Cada operador de valor aceita `values: string[]` além de `quantifier` (`any` por padrão, `all`; listas `neq` normalizam para `all` = «nenhum de»), e um alias opcional `as`. Os candidatos vencedores de um registro correspondente são publicados na renderização de ações como <code v-pre>{{ match.&lt;alias&gt; }}</code> / <code v-pre>{{ match.&lt;index&gt; }}</code> — limitados, sanitizados, injetados após a reserva de cooldown e nunca em projeções delegadas fora da lista explícita de campos `match.*`. Registros também podem carregar `anyOf`: até 4 grupos planos de até 12 filtros que fazem OR entre si enquanto continuam em AND com `filters`; somente o primeiro grupo vencedor contribui capturas.

```sh
target/debug/tiktools --json automation test --id <id>
target/debug/tiktools --json automation test --id <id> --event '{"type":"tiktok.chat","data":{"comment":"!play Daft Punk"}}'
```

Um disparo real publica o evento pelo pipeline ao vivo e **executa as ações**. Use-o apenas quando o operador pedir; verifique primeiro o teste a seco.

```sh
target/debug/tiktools --json automation fire --trigger tiktok.chat --event '{"type":"tiktok.chat","data":{"comment":"!play Daft Punk"}}'
target/debug/tiktools --json automation fire --trigger tiktok.chat --event '{"type":"tiktok.chat","data":{"comment":"!play Daft Punk"}}' --record '<editor draft json>'
```

`api call automation.fire '<params>' --dry-run` valida os parâmetros sem disparar; `api discover` e `api schema <method>` mostram o contrato atual.

## Limite dos scripts {#script-boundary}

`core.code` e nós de script de fluxos executam QuickJS com limites. Eles recebem JSON `event`, `inputs` e `data` (o alias de inputs). Não há carregador Node, sistema de arquivos, rede, WebView nem objeto de banco de dados. Mantenha transformações determinísticas e serializáveis em JSON; use ações configuradas do host para efeitos de HTTP, pontos, áudio ou plugins. Limites de código/resultado e orçamentos de execução pertencem ao host, não a um tempo limite do navegador.

## Resultado esperado e status {#expected-result-and-status}

Regras salvas executam pelo runtime do núcleo e seu intermediador de capacidades. Processadores podem adicionar evidências em `event.intel` sem substituir campos brutos. Changes de Automations é uma área de revisão local, não sincronização de configuração da conta inteira. Variables revisadas têm implementação nativa real; Changes de Overlays é uma continuação separada ainda não mesclada.

## Recuperação {#recovery}

Em um conflito, compare os registros atuais sem perder a candidata. Um rascunho remoto permanece desativado enquanto aguarda revisão do proprietário e transferência de autoridade; inspecionar prévia/manifesto sozinho não autoriza execução. Resultados desconhecidos de efeitos externos exigem o fluxo de inspeção/recuperação do efeito mantido. Reproduzir o histórico da interface nunca deve disparar a automação novamente. Use [modelos](templates.md) para importação controlada de biblioteca e [testes](testing.md) para evidências específicas de cada limite.

## Detalhes de operação {#operational-details}

O grafo abaixo usa portas v1 do catálogo trigger → transform → log e está desativado. Salve como `docs-graph.json` e confira versões/portas com `automation.nodes.list`. `workflow save --record` persiste; `workflow get` e enable/disable com metadados verificam estado salvo. Use revisões recém-lidas. Persistir/habilitar não prova executor completo ou editor visual em toda build: o runtime normal deste checkout executa Events/Actions de comportamento. Use o template de log de [templates](templates.md#operational-details), revise, habilite ambos intencionalmente e teste chat saneado para observar um efeito inofensivo. Salvar grafo não comprova execução LIVE.

Para enriquecimento, instale/habilite TextIntel com `events.enrich`, confira estado/métricas em Processors e teste sua amostra. Inspecione `event.intel.providers.textintel.comment.moderation.blocked` antes de criar filtro is-true para `core.log`; ausência do campo não significa evidência falsa. Circuito aberto ou saturação podem pular processadores. Amostra não comprova LIVE completo.

Para trabalho incerto consulte `automation.library.get`, `automation.definition.get`, `automation.adoption.get` ou `automation.approval.get`. Preserve pending.commandId, host e candidato. Resolva a operação original (`*.resolve`); se unknown, revise estado atual antes de `*.review_unknown` com ID original e confirmação explícita. Adoção exige nova revisão de autoridade/dependências, não só inspect/preview. `automation.effects.get {effectId}` inspeciona efeito externo retido: não autoriza despacho nem repetição cega. Preserve resultados desconhecidos para reconciliação operacional.

```json
{"schemaVersion":1,"id":"docs-example","name":"Docs","enabled":false,"nodes":[{"id":"start","type":"trigger.event","version":1,"position":{"x":0,"y":0},"config":{"eventType":"tiktok.chat"}},{"id":"text","type":"transform.template","version":1,"position":{"x":200,"y":0},"config":{"template":"docs-example"}},{"id":"log","type":"action.log","version":1,"position":{"x":400,"y":0},"config":{"message":"docs-example"}}],"edges":[{"id":"a","kind":"flow","source":"start","sourcePort":"flow","target":"text","targetPort":"flow"},{"id":"b","kind":"flow","source":"text","sourcePort":"flow","target":"log","targetPort":"flow"}]}
```

```sh
tiktools api call automation.nodes.list '{}' --json
tiktools workflow save --record "$(cat docs-graph.json)"
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.enable '{"id":"docs-example"}' --expected-revision REVISION --dry-run --json
tiktools api call automation.library.get '{}' --json
tiktools api call automation.adoption.get '{}' --json
```
