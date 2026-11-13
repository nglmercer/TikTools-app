---
title: "Importe modelos e gerencie perfis"
docId: "user.automations.templates"
translationKey: "user.automations.templates"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "49bb4d1eb5dff9f267b30e8d0ee693a6dfcb492ff4609d190370e40c72b094ab"
translationRevision: "49bb4d1eb5dff9f267b30e8d0ee693a6dfcb492ff4609d190370e40c72b094ab"
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
4. Aplique/importe explicitamente e aguarde o resultado. As definições importadas permanecem desativadas. Selecione-as depois e revise cada ação e evento antes de ativar explicitamente os desejados.
5. Comece com um teste apenas de registro ou inspecione as definições primeiro. Testes de HTTP, áudio, pontos ou plugins podem executar efeitos.
6. Confirme o perfil ativo antes de iniciar a LIVE. Trocar perfis muda o contexto da biblioteca selecionada; não é um comando de sincronização do Cloud.

Parâmetros como <code v-pre>{{ params.giftName }}</code> são resolvidos ao aplicar um modelo. Referências de execução como <code v-pre>{{ event.user.uniqueId }}</code> permanecem para cada evento, enquanto [globals](variables.md) fornece valores salvos compartilhados na execução.

## Resultado esperado e status {#expected-result-and-status}

As áreas locais de importação e exportação de modelos/perfis estão disponíveis. A navegação no Cloud exige a capacidade correspondente do host e a conta/serviço. Uma prévia de catálogo não comprova que o serviço externo de uma ação funciona. Plugins ausentes podem deixar as definições importadas inutilizáveis até serem instalados e configurados.

## Recuperação {#recovery}

Para JSON inválido, use os erros de análise informados e o tipo de documento esperado; renomear um JSON arbitrário não o converte. Para um conflito de revisão, atualize a biblioteca salva antes de aplicar novamente. Guarde o perfil exportado que funciona e consulte [backups](../preferences/backups.md) para os dados que a exportação de perfil não inclui.
