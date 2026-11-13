---
title: "Adapte receitas práticas de automação"
docId: "user.automations.recipes"
translationKey: "user.automations.recipes"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "43a103ef68e43d691eec2214ffefb5e2e0e6b663afa9f4e63b5dac0ccec162ea"
translationRevision: "43a103ef68e43d691eec2214ffefb5e2e0e6b663afa9f4e63b5dac0ccec162ea"
order: 30
---

# Adapte receitas práticas de automação {#adapt-practical-automation-recipes}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Transforme uma [regra de registro](first-rule.md) funcional em reações com correspondência restrita. Você precisa de um perfil ativo e de campos de evento observados. Comece com ações `core.log`; substitua-as por outro efeito somente após confirmar a correspondência e a frequência.

## Reconheça um comando de chat {#match-a-chat-command}

Crie um evento `tiktok.chat` com `event.data.comment` igual (`eq`) a `!hello`. Registre:

```text
Olá {{ event.user.uniqueId }}
```

Use um intervalo de espera por usuário de `30` segundos para que um espectador não o acione continuamente, enquanto outro ainda tenha uma oportunidade independente. A igualdade exata é útil para comandos; a correspondência por conteúdo também aceita mensagens com texto ao redor. Escolha de forma consciente.

## Registre um presente que atenda aos critérios {#record-a-qualifying-gift}

Crie um evento `tiktok.gift` com `event.data.diamondCount` maior ou igual (`gte`) a `10`. Registre:

```text
Presente {{ event.data.giftName }} de {{ event.user.uniqueId }}
```

Inspecione primeiro os detalhes reais do presente. O valor em diamantes, a contagem de repetições e o indicador de conclusão são campos diferentes. Se quiser uma reação apenas na conclusão, verifique se os dados recebidos fornecem `event.data.repeatEnd` e use a condição booleana verdadeira. Não deduza uma execução por combo concluído apenas de um filtro de nome.

## Limite uma reação compartilhada {#bound-a-shared-reaction}

Crie um evento `tiktok.follow` com intervalo de espera global de `10` segundos. Vincule uma ação curta de registro. Se dois espectadores seguirem durante esse intervalo, a espera global pode suprimir a reação posterior. Use o escopo por usuário quando cada espectador precisar de uma permissão separada.

Para várias ações, ordene a lista e mantenha o modo de todas as ações quando cada etapa for necessária. O modo aleatório escolhe uma ação; não executa todas as etapas em ordem aleatória. Adicione `core.delay` apenas quando as etapas seguintes precisarem aguardar.

## Resultado esperado e status {#expected-result-and-status}

Esses acionadores, filtros de campo e controles de espera estão disponíveis no mecanismo local. Uma execução deve corresponder a um evento aceito que atenda aos critérios, respeitando a espera e o estado de ativação. A aceitação específica do provedor ainda exige uma verificação na sua LIVE real.

## Recuperação {#recovery}

Se uma receita falhar, volte a uma única ação de registro, inspecione os dados e simplifique temporariamente as condições. Não desative os limites de uma transmissão movimentada durante a investigação. Testes de áudio, pontos, HTTP e plugins podem produzir efeitos reais. Confira a [configuração de áudio](../plugins/audio.md), as [variáveis](variables.md) e os [diagnósticos de execução](../preferences/troubleshooting.md) antes de adicionar essas dependências.
