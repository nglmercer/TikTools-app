---
title: "Configure regras de pontos"
docId: "user.points.rules"
translationKey: "user.points.rules"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "0f07dcc0bfec1e9da49b2d29cb1cfdc4d98801ebd1f0cfeac0ca2bff26e92180"
translationRevision: "0f07dcc0bfec1e9da49b2d29cb1cfdc4d98801ebd1f0cfeac0ca2bff26e92180"
order: 10
---

# Configure regras de pontos {#configure-point-rules}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Escolha como a atividade recebida ganha sua moeda local e depois verifique uma atribuição simples. Você precisa do TikTools em execução e de um criador conhecido para uma verificação posterior em LIVE. Regras de pontos e ações de automação são separadas: uma regra pode atribuir pontos mesmo sem nenhuma automação criada.

## Defina a política de ganhos {#set-the-earning-policy}

1. Abra **Points** e localize o painel de regras. Escolha um nome de moeda reconhecível, como `Estrelas`.
2. Ative apenas as fontes desejadas: moedas de presentes, chat, curtidas, compartilhamentos, novos seguidores ou entradas. Cada fonte tem sua própria taxa e interruptor de ativação.
3. Para uma primeira verificação simples, ative chat com `1` ponto e desative as outras fontes. Deixe o bônus de assinante em `0` enquanto estabelece a referência.
4. Defina os pontos por nível, por exemplo `100`. O editor impõe um mínimo de `10`; as taxas não podem ser negativas.
5. Selecione **Save** e aguarde a confirmação. **Discard** restaura a configuração salva enquanto você ainda está editando.
6. Conecte-se por [Connections](../live/connections.md), observe um novo evento de chat no [feed](../live/feed.md), atualize a classificação e pesquise esse espectador.

As taxas de presentes e curtidas usam as quantidades de contribuição recebidas. Por exemplo, uma contribuição de `10` curtidas a `0.1` ponto por curtida produz `1` ponto antes de qualquer bônus aplicável. Uma linha exibida no feed não é necessariamente uma unidade de contribuição. Alterar as taxas define a política para atribuições seguintes; não espere um recálculo do passado.

## Resultado esperado e status {#expected-result-and-status}

Regras locais, saldos de espectadores e controles manuais de adicionar/deduzir estão disponíveis. Com a configuração isolada de chat, uma nova mensagem aceita deve aumentar em um o saldo do espectador. Os níveis seguem o valor configurado de pontos por nível. Esses são pontos locais do aplicativo, não moeda do TikTok nem promessa de pagamento.

## Recuperação {#recovery}

Verifique primeiro o interruptor da fonte, a taxa salva e o identificador do espectador. Uma automação que também atribui pontos pode adicionar uma segunda atribuição. Os controles de redefinição ficam bloqueados durante conexões LIVE ativas/em andamento; desconecte antes de uma redefinição planejada e leia sua confirmação. Mantenha um [backup com o aplicativo fechado](../preferences/backups.md) antes de limpar saldos acumulados.

<DemoFrame scenario="points" locale="pt" tour="points" title="Demonstração: pontos" />
