---
title: "Revise estatísticas entre datas"
docId: "user.points.statistics"
translationKey: "user.points.statistics"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a7333d75baa9aec6e0c4afb69d1c773575a56ee8da982f0fa5a11a73b903ec2d"
translationRevision: "7c3770e4190c05abb3a91210f4474d18a8f26b6dc5e9ba104cb60cca8a635f0e"
order: 20
---

# Revise estatísticas entre datas {#review-statistics-across-dates}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Compare a atividade de um criador em um intervalo de datas útil e entenda por que os totais diferem do feed visível ou do saldo de pontos. Você precisa de atividade registrada na instalação local e do criador desejado selecionado por [Connections](../live/connections.md). O login no Cloud não é a origem dessas análises locais.

## Consulte o período registrado {#query-the-recorded-period}

1. Abra **Analytics** e confirme o criador exibido ao lado dos controles de data. O criador selecionado determina quais agregados armazenados são consultados.
2. Escolha **Today**, `7d`, `30d`, `90d` ou um intervalo personalizado. Para datas personalizadas, defina início e fim explicitamente e confira sua ordem.
3. Leia o intervalo exibido e a contagem de sessões antes de comparar resultados. Os limites de datas locais usam um deslocamento de fuso horário; mudar o fuso do computador pode alterar a interpretação de um período.
4. Selecione a métrica necessária: chats, presentes, curtidas, diamantes ou pico de espectadores. Use os gráficos de atividade para comparar períodos e a tabela de contribuidores para inspecionar os principais espectadores retornados.
5. Pesquise na tabela de contribuidores um espectador já presente nos dados retornados. Não é uma pesquisa irrestrita de todos os espectadores já observados.
6. Selecione **Refresh** após nova atividade ou ao retornar à página. Registre criador, datas e métrica junto de qualquer número compartilhado.

A página atual chama a operação nativa `analytics.summary` com um criador e intervalo de datas. Ela lê agregados persistidos, incluindo sessões registradas anteriormente; não se limita aos eventos ainda mantidos no feed atual. O banco de análises armazena agregados, não uma transcrição do chat.

## Resultado esperado e status {#expected-result-and-status}

Análises locais persistidas e consultas por intervalo de datas estão disponíveis. Datas vazias legitimamente não retornam atividade. Saldos de pontos podem diferir porque [taxas de ganho](rules.md), ajustes manuais e atribuições de automação expressam uma medida diferente. Diamantes, contagens de presentes e pontos não são unidades intercambiáveis.

## Recuperação {#recovery}

Se os dados parecerem ausentes, limpe a pesquisa de contribuidores, amplie o intervalo de datas, confirme o criador e atualize. Após um erro do host, guarde seu código de diagnóstico e tente novamente quando a conexão se recuperar. Limpar o feed não apaga as análises. Um diretório de dados novo não tem histórico anterior; use [backups](../preferences/backups.md) ao mover instalações.

<DemoFrame scenario="analytics" locale="pt" title="Demonstração: análises" />

## Detalhes de operação {#operational-details}

O Analytics solicita atualmente 10 principais contribuidores para o criador e período escolhidos. A busca filtra esse subconjunto, sem pesquisar todos os espectadores. Esses agregados persistidos têm período e unidade diferentes dos rankings de diamantes/likes da sessão e dos saldos de pontos. Preserve criador, datas e métrica ao exportar ou comparar.
