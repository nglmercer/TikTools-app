---
title: "Revise alterações por área"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "470449e93855a0308eeb03939acc2d6e929761dd28d5f7ff8930152aaf0d1fe7"
translationRevision: "470449e93855a0308eeb03939acc2d6e929761dd28d5f7ff8930152aaf0d1fe7"
order: 20
---

# Revise alterações por área {#review-changes-by-area}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Entenda o que um painel **Changes** compara antes de redefinir, enviar ou adotar dados. Você precisa de conteúdo local salvo. Operações do Cloud também precisam de um host compatível, do modo de sincronização relevante e de uma [conta autenticada](account.md). Painéis com o mesmo nome não realizam todos o mesmo transporte.

## Revise Events e Actions localmente {#review-events-and-actions-locally}

1. Abra **Automations**, selecione Events ou Actions e abra **Changes**.
2. Leia o aviso: essa área compara as definições atuais com uma referência mantida neste dispositivo. A primeira visita carregada inicializa essa referência, portanto pode legitimamente não mostrar alterações.
3. Selecione um registro adicionado, modificado ou excluído e inspecione os valores anteriores e posteriores.
4. Escolha **Mark as reviewed** somente quando quiser que a seção atual se torne a nova referência local. Isso não envia definições nem aprova execução delegada.
5. Use **Reset** apenas quando pretender restaurar a referência. Para um registro recém-adicionado, a redefinição o remove; para um registro modificado/excluído, ela restaura a definição mantida. Salve uma exportação antes, se necessário.

O adaptador de Events/Actions não tem coleção do Cloud para definições individuais nesta versão. Uma comparação sem alterações não é um backup no Cloud nem prova de que outro computador recebeu suas edições.

## Revise Variables e Plugins explicitamente {#review-variables-and-plugins-explicitly}

Variables tem seu próprio compartilhamento nativo revisado: salve localmente, busque candidatas, inspecione a comparação e explicitamente envie, aplique/adote ou retire. Valores secretos ou bloqueados continuam mascarados e não podem ser enviados por esse caminho. Salvar localmente nunca envia. Consulte [Variables](../automations/variables.md).

Changes de Plugins usa seu workspace de sincronização compatível para seleções de plugins e configurações preparadas. A publicação ocorre por seção, em vez de um envio separado para cada linha. As configurações mostram nomes das chaves alteradas sem expor valores; conflitos permitem escolher a versão local ou do Cloud, não mesclar campos arbitrariamente. Leia a ação da seção e o modo de sincronização selecionado antes de publicar.

Overlays do Cloud usam o fluxo de criação/revisão da [biblioteca/editor](../overlays/index.md). Um adaptador compartilhado Changes de overlays não está disponível nesta versão base.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

A comparação local de automações e o compartilhamento nativo condicionado às capacidades estão disponíveis nas respectivas áreas. Operações com efeitos remotos dependem do serviço; isso não implica aceitação em produção. Se uma revisão ficar desatualizada, atualize e compare novamente. Se o resultado não estiver confirmado, inspecione as informações de status/diário antes de criar outra operação. Nunca trate um tempo limite como prova de que nada mudou.

<DemoFrame scenario="cloud" locale="pt" tour="changes" title="Demonstração: Cloud" />
