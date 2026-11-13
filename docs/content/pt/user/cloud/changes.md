---
title: "Revise alterações por área"
docId: "user.cloud.changes"
translationKey: "user.cloud.changes"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f875a22e66dd822d7ea1344188d8b7b087a23a777e072240a4a3c7c57969f995"
translationRevision: "4227e2b6755c6c7032fc887c3b0772bd2d33364d120b710d415f8234e2444fb8"
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

## Detalhes de operação {#operational-details}

Antes de preparar/migrar v1 → v2, faça backup e coordene versões compatíveis em todos os dispositivos. No setup prepare, confira compatibilidade e visualize; revise contagens de seleção, pendências, conflitos e itens só-v1, sem presumir sincronização. Exigir cabeçalho de época é irreversível para escritas v2 de toda a conta e pode rejeitar dispositivos antigos. Timeout da barreira é ambíguo; repeti-la avança novamente. Atualize estado de época/migração e prévia antes de agir. Ler época não comprova escritas anteriores concluídas.

A recuperação depende do modo: Changes de Events/Actions compara referências locais, sem escrita Cloud. Para Variables/Plugins revisados, atualize comparações expiradas e confira revisão, conta e inventário. Se faltarem plugins, use revisão/reparo de inventário do setup e outra prévia. Para intenção incerta, preserve ID/evidência e consulte histórico/estado; recuperação técnica exige nova revisão das entradas congeladas e revisão Cloud. Retomar essa intenção não envia escrita remota por si só. Pare sem criar mutação substituta se não puder reconciliar.
