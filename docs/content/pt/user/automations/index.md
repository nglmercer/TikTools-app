---
title: "Organize automações"
docId: "user.automations.index"
translationKey: "user.automations.index"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4b3afb8ff9602033e4ffeba505da980bc2a9d617796853f59645c0f40eaf53ad"
translationRevision: "86883982c8b04da84950756859c6bb51ee159858e85e082b4c78c37a6ae8c2f5"
order: 10
---

# Organize automações {#organize-automations}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Crie uma biblioteca pequena e compreensível que responda a eventos de LIVE. Você precisa de um host de desktop funcional. Uma [conexão real com uma LIVE](../live/connections.md) é necessária para validar os eventos recebidos do serviço; ações de plugins também exigem que o plugin e seu serviço estejam prontos.

## Entenda a biblioteca {#understand-the-library}

Uma **Action** é uma operação configurada reutilizável, como escrever uma linha no registro ou tocar um som. Um **Event** define um acionador, filtros, intervalo de espera e ações vinculadas. Um perfil agrupa definições de um fluxo de trabalho; apenas um pacote de perfil fica ativo por vez. As [Variables](variables.md) de execução fornecem valores que várias ações podem reutilizar.

1. Abra **Automations** e confirme o perfil ativo antes de editar. Pesquise Events e Actions separadamente para saber qual tipo de registro está selecionando.
2. Comece pela [primeira regra de registro](first-rule.md). Crie sua ação e depois um evento que a referencie.
3. Escolha um acionador aceito pelo host atual. Use o seletor de campos para adicionar filtros desse acionador. Várias condições restringem as correspondências; elas não alteram os dados recebidos.
4. Defina um intervalo de espera e seu escopo. Um intervalo global limita a regra entre todos os espectadores; um intervalo por usuário acompanha cada espectador separadamente.
5. Vincule as ações desejadas na ordem. O modo normal executa a lista escolhida; o modo aleatório seleciona uma ação dela. Uma ação de atraso adia as ações seguintes, sem reduzir a velocidade de ingestão da LIVE.
6. Salve, revise as alterações de definição exibidas e ative explicitamente os registros que pretende executar. Confira no histórico de execução o nome da ação, o resultado e o erro.

Toda comparação aceita uma lista de valores em vez de um único valor. A combinação padrão é “qualquer”: a condição é atendida quando pelo menos um valor corresponde. Mude o quantificador para “todos” quando todos os valores precisarem corresponder — com “não é”, isso significa “não é nenhum”. Uma condição também pode nomear um alias de captura: o valor vencedor fica disponível nos modelos de ação como <code v-pre>{{ match.&lt;alias&gt; }}</code> (ou <code v-pre>{{ match.&lt;posição&gt; }}</code> sem alias). Abaixo das condições sempre obrigatórias, você pode adicionar até quatro **alternativas**: grupos em que todas as condições precisam passar internamente, e basta um único grupo para o evento disparar.

Os testes podem executar os efeitos configurados. Leia a lista de ações selecionadas antes de pressionar **Test** ou disparar um evento, especialmente para áudio, HTTP, pontos e operações de plugins. Use registros enquanto aprende.

## Resultado esperado e status {#expected-result-and-status}

Edição local, execução, modelos, perfis e histórico estão disponíveis. Acionadores de plugins aparecem apenas enquanto o plugin correspondente está instalado e ativo. A ativação depende da superfície e dos valores enabled revisados; confira eventos e ações antes de confirmar. O painel **Changes** de Events/Actions é uma [comparação com referência local](../cloud/changes.md), não um envio automático ao Cloud.

## Recuperação {#recovery}

Quando não houver execução, verifique perfil ativo, estados de ativação, acionador, filtros, intervalo de espera e ações ausentes, nessa ordem. Para um conflito de revisão, leia a definição salva atual e revise novamente. Não sobrescreva a atualização de outra pessoa às cegas. Exporte um [perfil](templates.md) que funcione antes de fazer alterações amplas.

## Detalhes de operação {#operational-details}

Antes de trocar perfil/importar, leia [ativação por superfície](templates.md#operational-details). A troca pode ativar membros desativados individualmente. Cooldown fica em memória, limitado a 24 horas; sem uniqueId os espectadores compartilham escopo anônimo. A tabela esvazia ao atingir 8.192 chaves: sobrecarga ou reinício podem permitir execuções adicionais. Não é cota durável nem garantia contra replay.

Experimente interativamente antes de tocar na sua configuração real:

<DemoFrame scenario="automation" locale="pt" panel="automation" title="Experimente o espaço de automações" />

## Operações de linha de comando {#command-line-operations}

Cada operação deste guia também está disponível na CLI — offline, scriptável e apta para agentes. As linhas de uso são sintaxe do parser e permanecem em inglês em todos os idiomas.

<!--@include: ../../../../.generated/cli-domain-automation.pt.md-->

<!--@include: ../../../../.generated/cli-domain-workflow.pt.md-->

Consulte a [referência completa de comandos](../../developer/cli.md) para as convenções, a saída JSON e os marcadores de operações destrutivas.

<TourFrame id="automationFilters" locale="pt" title="Guia interativo: Filtros e condições" />

<TourFrame id="automationCooldowns" locale="pt" title="Guia interativo: Cooldowns e modos" />
