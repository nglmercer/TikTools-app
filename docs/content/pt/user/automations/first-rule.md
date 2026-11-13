---
title: "Crie sua primeira regra de registro"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
translationRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
order: 20
---

# Crie sua primeira regra de registro {#create-your-first-logging-rule}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Verifique o caminho de evento a ação com uma linha de registro local. Você precisa de um host de desktop em execução e acesso a **Automations**. Não é necessária conta do Cloud, serviço de áudio nem destino externo. Use texto de chat fictício ou não sensível: o registro armazena a mensagem renderizada.

## Crie a ação {#create-the-action}

1. Abra **Automations → Actions** no perfil que deseja usar.
2. Crie uma ação do tipo **Write to the log** (`core.log`) chamada `Verificação do chat`.
3. Defina o campo Message com o modelo abaixo, depois salve e ative a ação:

```text
Verificação do chat: {{ event.user.uniqueId }} disse {{ event.data.comment }}
```

Esses são os caminhos normalizados atuais do chat. `event.data.comment` é a mensagem; `event.user.uniqueId` é o identificador do espectador. O nome de exibição usa um campo diferente.

## Crie o evento {#create-the-event}

1. Abra **Events**, crie `Evento de verificação do chat` e selecione `tiktok.chat` como acionador.
2. Vincule apenas `Verificação do chat`. Mantenha o modo normal de todas as ações.
3. Deixe os filtros vazios para o primeiro teste com amostra. Defina um intervalo de espera global de `10` segundos para limitar o tráfego real depois.
4. Salve e revise qualquer prévia de alteração. Com apenas a ação de registro vinculada, use **Test** e inspecione a execução retornada e o texto do registro. O host pode fornecer uma amostra; um resultado de amostra não comprova conectividade real com o TikTok.
5. Adicione uma condição sobre `event.data.comment`, operador igual (`eq`), valor `!hello`. Salve novamente e ative explicitamente o evento.
6. Conecte-se a uma [LIVE](../live/connections.md) em que tenha autorização para testar. Observe uma nova mensagem `!hello` no [feed](../live/feed.md) e depois confira o histórico da automação quando o intervalo permitir.

## Resultado esperado e status {#expected-result-and-status}

A ação de registro local e o acionador de chat estão disponíveis. A amostra deve produzir um registro legível; um chat real correspondente deve produzir uma execução com o identificador e a mensagem reais. Outros textos de chat devem falhar na condição de correspondência exata.

## Recuperação {#recovery}

Se uma amostra deixar de corresponder após adicionar o filtro, seu comentário fornecido pode ser diferente de `!hello`. Verifique os campos do evento real, o perfil ativo e os estados de ativação em vez de disparar repetidamente. Testes podem executar efeitos; manter esta regra apenas com registro torna seu efeito previsível. Desative o evento ao terminar ou continue com as [receitas](recipes.md).

<DemoFrame scenario="automation" locale="pt" tour="automation" title="Demonstração: automação" />
