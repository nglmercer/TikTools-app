---
title: "Configure regras de pontos"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7a5005919428a41f471ef978adcefa865ab0ffb09f8d80cff8fee2b0464773ef"
translationRevision: "e4e36c516e8901c1535f5a1d6dfa54621174d0bdc59e380e3d2bd3af539f66d5"
order: 10
---

# Configure regras de pontos {#configure-point-rules}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Escolha como a atividade recebida ganha sua moeda local e depois verifique uma atribuição simples. Você precisa do TikTools em execução e de um criador conhecido para uma verificação posterior em LIVE. Regras de pontos e ações de automação são separadas: uma regra pode atribuir pontos mesmo sem nenhuma automação criada.

## Defina a política de ganhos {#set-the-earning-policy}

<DemoFrame scenario="points" locale="pt" panel="points" title="Explore as regras e os saldos de pontos" />

<DocSteps>

1. Abra **Pontos** e localize o painel de regras. Escolha um nome de moeda reconhecível, como `Estrelas`.
2. Ative apenas as fontes desejadas: moedas de presentes, chat, curtidas, compartilhamentos, novos seguidores ou entradas. Cada fonte tem sua própria taxa e interruptor de ativação.
3. Para uma primeira verificação simples, ative chat com `1` ponto e desative as outras fontes. Deixe o bônus de assinante em `0` enquanto estabelece a referência.
4. Defina os pontos por nível, por exemplo `100`. O editor impõe um mínimo de `10`; as taxas não podem ser negativas.
5. Selecione **Salvar regras** e aguarde a confirmação. **Descartar** restaura a configuração salva enquanto você ainda está editando.
6. Conecte-se por [Conexões](../live/connections.md), observe um novo evento de chat no [feed](../live/feed.md), atualize a classificação e pesquise esse espectador.

</DocSteps>

As taxas de presentes e curtidas usam as quantidades de contribuição recebidas. Por exemplo, uma contribuição de `10` curtidas a `0.1` ponto por curtida produz `1` ponto antes de qualquer bônus aplicável. Uma linha exibida no feed não é necessariamente uma unidade de contribuição. Alterar as taxas define a política para atribuições seguintes; não espere um recálculo do passado.

## Resultado esperado e status {#expected-result-and-status}

Regras locais, saldos de espectadores e controles manuais de adicionar/deduzir estão disponíveis. Com a configuração isolada de chat, uma nova mensagem aceita deve aumentar em um o saldo do espectador. Os níveis seguem o valor configurado de pontos por nível. Esses são pontos locais do aplicativo, não moeda do TikTok nem promessa de pagamento.

## Recuperação {#recovery}

Verifique primeiro o interruptor da fonte, a taxa salva e o identificador do espectador. Uma automação que também atribui pontos pode adicionar uma segunda atribuição. Os controles de redefinição ficam bloqueados durante conexões LIVE ativas/em andamento; desconecte antes de uma redefinição planejada e leia sua confirmação. Mantenha um [backup com o aplicativo fechado](../preferences/backups.md) antes de limpar saldos acumulados.

<DocCallout title="Compare um evento com um saldo" tone="tip">

Salve apenas a regra de chat a 1 ponto e confira o mesmo espectador antes e depois de uma nova mensagem. Verifique também se alguma automação adiciona pontos para esse evento.

</DocCallout>

## Detalhes de operação {#operational-details}

O menu da linha zera os pontos de um espectador, define nível 1 e acrescenta um marcador. **Zerar todos** afeta todos os espectadores salvos nesta instalação, entre criadores: zera saldos, define nível 1 e apaga o histórico de transações de pontos. Mantém os registros e contadores acumulados. Não limpa o feed nem o Analytics. Desconecte a LIVE, faça uma cópia com o aplicativo fechado e confira o alcance antes de confirmar.

Ajustes manuais positivos recebem o bônus de assinante salvo: +100 com bônus de 50% concede +150 a um assinante. Descontar 100 não recebe esse bônus; o saldo mínimo é zero. Confira o saldo final em vez de presumir um delta exato. A tabela busca apenas na sua captura dos 100 primeiros; um nome ausente não comprova ausência de registro.

## Operações de linha de comando {#command-line-operations}

Cada operação deste guia também está disponível na CLI — offline, scriptável e apta para agentes. As linhas de uso são sintaxe do parser e permanecem em inglês em todos os idiomas.

<!--@include: ../../../../.generated/cli-domain-points.pt.md-->

Consulte a [referência completa de comandos](../../developer/cli.md) para as convenções, a saída JSON e os marcadores de operações destrutivas.

<TourFrame id="points" locale="pt" title="Guia interativo: Pontos" />
