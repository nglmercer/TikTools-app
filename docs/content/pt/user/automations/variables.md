---
title: "Reutilize e revise variáveis de execução"
docId: "user.automations.variables"
translationKey: "user.automations.variables"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "d60768cad1ee5373038b070f51a5ae8789e29ca37075d288ac60ac22e5ef1732"
translationRevision: "1e2596f78bb00a2dc39c3fd2dee5bb852d0164049616910a67feadd3aaee0d99"
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

As condições também podem publicar valores por correspondência. Quando um filtro declara um alias de captura, o valor que o satisfez fica disponível durante a renderização das ações do evento como <code v-pre>{{ match.&lt;alias&gt; }}</code> (ou <code v-pre>{{ match.&lt;posição&gt; }}</code> sem alias). Por exemplo, uma condição “começa com” no comentário com o alias `prefix` permite que uma ação de registro imprima <code v-pre>{{ match.prefix }}</code>. As capturas são texto simples limitado, só são produzidas pelos filtros e alternativas que realmente corresponderam e não alteram as chaves de espera.

Prima **+** junto a um campo compatível com modelos para abrir **Inserir variável**: o caminho exato aparece à direita e escolher uma linha acrescenta <code v-pre>{{ caminho }}</code> ao campo. As capturas deste evento também aparecem nessa lista, mas só são resolvidas se a sua condição correspondeu. Defina um alias na secção Condições (o botão **Capturar** no rodapé mostra os campos de alias) para escrever <code v-pre>{{ match.&lt;alias&gt; }}</code>; caso contrário, use a posição <code v-pre>{{ match.&lt;posição&gt; }}</code>.

## Revise alterações do Cloud separadamente {#review-cloud-changes-separately}

O controle **Changes** de Variables tem operações nativas revisadas de enviar, buscar, aplicar/adotar e retirar quando compatíveis. Salve as edições locais primeiro, selecione uma comparação, inspecione seu tipo e valor e escolha explicitamente a direção desejada. Buscar uma candidata do Cloud é separado de aplicá-la localmente. Retirar um valor compartilhado é separado de excluir uma linha local.

Valores secretos ou bloqueados ficam mascarados nas comparações e não podem ser enviados por esse caminho revisado. Nunca transforme um segredo em texto comum apenas para contornar esse bloqueio.

## Resultado esperado, status e recuperação {#expected-result-status-and-recovery}

Variáveis locais e compartilhamento revisado condicionado às capacidades estão disponíveis; salvar localmente nunca envia automaticamente. Chaves inválidas, valores tipados inválidos e revisões desatualizadas bloqueiam o salvamento. Corrija o campo destacado ou recarregue antes de tentar novamente. Para falhas de conta, consulte [conta do Cloud](../cloud/account.md); para o comportamento de comparação, consulte [Changes](../cloud/changes.md).

Experimente interativamente antes de tocar na sua configuração real:

<DemoFrame scenario="automation" locale="pt" panel="automation" title="Experimente o espaço de automações" />

