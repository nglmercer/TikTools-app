---
title: "Importe modelos e gerencie perfis"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e9847c93a8a75fd5a58cd115b49b8e0bcb091ceda1dbdf2c226b52667eb17e23"
translationRevision: "8b1eb642c045b49c1335ee05f654a44e666e5cf6ef855fb47579c0781c99a4ab"
order: 50
---

# Importe modelos e gerencie perfis {#import-templates-and-manage-profiles}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Reutilize uma regra revisada ou uma coleção de regras sem ativar efeitos por acidente. Você precisa de **Automations**, de um arquivo JSON ou item de catálogo confiável e dos plugins exigidos pelas ações. Exporte o perfil atual antes de substituir uma configuração estabelecida.

## Escolha a área correta {#choose-the-right-surface}

Um documento `.tiktemplate.json` descreve um evento e suas ações. Um documento `.tikprofile.json` agrupa vários modelos. Abra o diálogo de modelos em Automations para acessar modelos integrados/personalizados, sua biblioteca do Cloud ou a importação manual de modelos. Use a barra de ferramentas de perfis para criar, selecionar, exportar e importar perfis manualmente. **Plugins → Explore** navega pelos plugins; modelos e perfis pertencem a esta área.

## Revise e importe {#review-and-import}

1. Abra o diálogo de importação desejado e escolha um arquivo ou cole JSON compatível. Confira o nome e a origem preparados antes de continuar.
2. Para um modelo, inspecione o acionador, os tipos de ação e os parâmetros. Para um perfil, inspecione também a quantidade de itens e os parâmetros comuns.
3. Substitua destinos de exemplo, caminhos de mídia e identificadores de espectadores por valores adequados à sua configuração. Não cole credenciais em um arquivo destinado ao compartilhamento.
4. Aplique/importe explicitamente e aguarde o resultado. Não presuma que as definições ficam desativadas: confira os valores enabled. Selecione-as depois e revise cada ação e evento antes de ativar explicitamente os desejados.
5. Comece com um teste apenas de registro ou inspecione as definições primeiro. Testes de HTTP, áudio, pontos ou plugins podem executar efeitos.
6. Antes de confirmar perfil, revise ativação dos membros do destino e desativação dos demais, incluindo registros avulsos e membros antes desativados. Efeitos exigem gatilhos/condições; não é sync Cloud.

Parâmetros como <code v-pre>{{ params.giftName }}</code> são resolvidos ao aplicar um modelo. Referências de execução como <code v-pre>{{ event.user.uniqueId }}</code> permanecem para cada evento, enquanto [globals](variables.md) fornece valores salvos compartilhados na execução.

## Resultado esperado e status {#expected-result-and-status}

As áreas locais de importação e exportação de modelos/perfis estão disponíveis. A navegação no Cloud exige a capacidade correspondente do host e a conta/serviço. Uma prévia de catálogo não comprova que o serviço externo de uma ação funciona. Plugins ausentes podem deixar as definições importadas inutilizáveis até serem instalados e configurados.

## Recuperação {#recovery}

Para JSON inválido, use os erros de análise informados e o tipo de documento esperado; renomear um JSON arbitrário não o converte. Para um conflito de revisão, atualize a biblioteca salva antes de aplicar novamente. Guarde o perfil exportado que funciona e consulte [backups](../preferences/backups.md) para os dados que a exportação de perfil não inclui.

## Detalhes de operação {#operational-details}

Importar template manual primeiro salva um documento no catálogo; aplicá-lo cria ações/eventos vinculados. O parser usa true quando `enabled` falta e a aplicação preserva esses valores. Importar perfil manual prepara entradas para revisão do proprietário. O catálogo Cloud salva templates; um perfil Cloud prepara uma mutação com procedência. Origem ou prévia bem-sucedida não garantem desativação universal. Confira comparação final e estados salvos. Na CLI use explicitamente `tiktools template import file.tiktemplate.json --disabled` ou `tiktools template profile-import file.tikprofile.json --disabled`.

Trocar perfil muda ativação: confirmar habilita membros do destino e desabilita todas as ações/eventos externos, incluindo registros avulsos. Membros antes desativados podem ser ativados. Revise ambas as listas. Efeitos ainda exigem gatilhos, condições e runtime disponíveis; a troca não envia sincronização Cloud por si só.
