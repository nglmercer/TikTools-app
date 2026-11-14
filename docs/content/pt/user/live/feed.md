---
title: "Leia e inspecione o feed da LIVE"
docId: "user.live.feed"
translationKey: "user.live.feed"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "4830f0a02244f46bfefb9b37c09d96c969688d022a0df90359c96c3e359af147"
translationRevision: "695064c8829cdb35f26e51b5ea42e6c108544b66dc0fa07d3bceb05fc98130f3"
order: 20
---

# Leia e inspecione o feed da LIVE {#read-and-inspect-the-live-feed}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Encontre um evento, inspecione seus campos e diferencie uma exibição pausada de uma sala desconectada. Comece com uma [conexão com LIVE](connections.md) ou use eventos já presentes no feed atual. O feed é uma visualização de trabalho limitada, não uma gravação completa de uma transmissão.

## Inspecione a atividade {#inspect-activity}

1. Abra **Live feed** e confirme o criador ativo no status da conexão.
2. Escolha um filtro de eventos: todos, chat, presentes, curtidas, novos seguidores, entradas ou outras atividades sociais. Combine com pesquisa para encontrar um autor, mensagem, nome de presente ou ID exato de presente.
3. Selecione **Pause** para parar a rolagem automática enquanto lê. Os eventos podem continuar chegando e a contagem de não lidos pode aumentar. Esse controle não desconecta o TikTok nem suspende automações.
4. Selecione um evento para abrir seu inspetor. Leia o tipo de evento e os detalhes disponíveis antes de escolher campos de automação. Nem todo evento contém texto de chat, informações de presente ou identidade de espectador.
5. Selecione o nome ou avatar de um espectador para inspecionar seu perfil acompanhado. Use o controle do inspetor se o painel de detalhes estiver oculto.
6. Selecione **Resume** ou o botão de salto para eventos não lidos para voltar à atividade recebida. Limpe a pesquisa e selecione todos os eventos ao verificar se há atividade chegando.

Use os controles rotulados com foco de teclado quando possível. Pausar a rolagem ajuda quando o movimento dificulta a leitura. Um avatar ausente tem iniciais como alternativa; uma imagem não é necessária para identificar um espectador.

## Resultado esperado e status {#expected-result-and-status}

Filtragem e inspeção estão disponíveis. Durante tráfego intenso, a interface pode mostrar uma janela de renderização limitada e informar eventos descartados na interface. Esses indicadores descrevem a exibição; não use a quantidade de linhas visíveis como total contábil. [Analytics](../points/statistics.md) consulta agregados persistidos separadamente.

## Recuperação {#recovery}

Se o feed estiver vazio, remova os filtros, retome a rolagem e confira o status da conexão. Uma sala silenciosa pode estar funcionando normalmente. Reconectar ao mesmo criador mantém o contexto; trocar de criador o altera. **Clear** esvazia o feed de trabalho e não redefine pontos nem análises. Investigue indicadores de perda pelos [diagnósticos](../preferences/troubleshooting.md), sem presumir que todas as linhas ausentes da exibição sejam eventos perdidos na origem.

<DemoFrame scenario="feed" locale="pt" title="Demonstração: feed" />

## Detalhes de operação {#operational-details}

Esta versão mantém até 300 eventos visíveis e prepara até 900 enquanto a apresentação aguarda. Remove primeiro a linha mais antiga que não é presente; presentes só saem quando não restam outros tipos. Um presente antigo pode permanecer depois de um chat recente. Pausar a rolagem não interrompe a entrada nem garante retenção. Filtros não recuperam linhas removidas; um aviso de perda indica uma visualização incompleta, não autorização para repetir pontos ou ações.
