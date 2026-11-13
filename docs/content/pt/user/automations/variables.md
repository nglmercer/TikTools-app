---
title: "Reutilize e revise variáveis de execução"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
translationRevision: "20ba9e681b63ffa9aff2ff5d868738d242fd4eab4d520d9e08656760ee8361ba"
order: 40
---

# Reutilize e revise variáveis de execução {#reuse-and-review-runtime-variables}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Altere um valor compartilhado uma vez e reutilize-o em várias ações. Você precisa de **Automations → Variables** e de um host compatível com as operações de variáveis relevantes. A edição tipada e as operações revisadas do Cloud aparecem conforme as capacidades do host; salvar localmente não exige envio.

## Salve um valor local {#save-a-local-value}

1. Adicione uma variável chamada `showLabel`. As chaves começam com uma letra ou sublinhado e podem conter letras, dígitos, pontos, sublinhados e hifens, até `64` caracteres. Evite chaves duplicadas.
2. Quando a edição tipada estiver disponível, escolha texto e digite `Programa da noite`. Outros tipos aceitos são número, booleano, lista e segredo. Listas usam um array JSON de strings, como `["vermelho", "azul"]`.
3. Selecione **Save**. Um conflito de revisão significa que a coleção salva mudou; recarregue e revise a edição pretendida em relação a ela.
4. Use o valor salvo em um campo de ação com modelo:

```text
{{ globals.showLabel }}: {{ event.user.uniqueId }}
```

5. Teste uma ação apenas de registro para inspecionar o texto renderizado. Altere `showLabel`, salve novamente e execute um novo teste. As definições de ações existentes podem reutilizar o valor atualizado sem reimportar um modelo.

Os `params` do modelo são substituídos na aplicação do modelo; `globals` são lidos na execução. Uma chave ausente em tempo de execução é renderizada como texto vazio, portanto confira a grafia antes de usá-la em um destino ou identificador. Use as sugestões de variáveis do campo para caminhos de evento adequados ao seu acionador.

## Revise alterações do Cloud separadamente {#review-cloud-changes-separately}

O controle **Changes** de Variables tem operações nativas revisadas de enviar, buscar, aplicar/adotar e retirar quando compatíveis. Salve as edições locais primeiro, selecione uma comparação, inspecione seu tipo e valor e escolha explicitamente a direção desejada. Buscar uma candidata do Cloud é separado de aplicá-la localmente. Retirar um valor compartilhado é separado de excluir uma linha local.

Valores secretos ou bloqueados ficam mascarados nas comparações e não podem ser enviados por esse caminho revisado. Nunca transforme um segredo em texto comum apenas para contornar esse bloqueio.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

Variáveis locais e compartilhamento revisado condicionado às capacidades estão disponíveis; salvar localmente nunca envia automaticamente. Chaves inválidas, valores tipados inválidos e revisões desatualizadas bloqueiam o salvamento. Corrija o campo destacado ou recarregue antes de tentar novamente. Para falhas de conta, consulte [conta do Cloud](../cloud/account.md); para o comportamento de comparação, consulte [Changes](../cloud/changes.md).
