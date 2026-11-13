---
title: "Gerencie conexões com LIVE"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "37f7274550d3010051ab456da2a2091b650e5e7aaaf30756b6695435bcfc08c7"
translationRevision: "85e930959fb8374b92c8dcb000a83d5b706c61651518b47567b2ee8b4ded7a1e"
order: 10
---

# Gerencie conexões com LIVE {#manage-live-connections}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Conecte-se a um criador específico, entenda a reconexão e configure servidores de plugins de forma independente. Você precisa de uma [instalação de desktop](../installation.md) funcional, acesso à rede e o identificador do criador. Confira os [requisitos](../requirements.md) antes de diagnosticar problemas do sistema operacional.

## Conecte-se de forma consciente {#connect-deliberately}

1. Abra **Connections**. Digite o identificador, com ou sem `@`; um nome de exibição não é o mesmo identificador.
2. Comece com **Use Cookie** desligado. Se o TikTok exigir uma sessão autorizada, ative-o e digite o cookie apenas no campo mascarado. Cookies permanecem na memória; não os cole em diagnósticos nem configurações do Cloud.
3. Deixe **Monitor and auto-connect** desligado para uma tentativa direta e selecione **Connect**. Leia o status e qualquer erro antes de tentar novamente.
4. Confira no [Live feed](feed.md) o criador correto e um evento novo. Uma sala conectada, mas silenciosa, pode não mostrar atividade.
5. Use **Reconnect** para o mesmo criador ou **Disconnect** antes de trocar de criador. Durante uma tentativa em andamento, **Cancel** interrompe essa tentativa.

Após uma conexão direta bem-sucedida, ative o monitoramento para aguardar um criador entrar ao vivo. Esperar e verificar são diferentes de estar conectado. Pare o monitoramento antes de editar um identificador bloqueado. Selecionar um criador recente inicia uma conexão; faz mais do que preencher o campo.

O explorador oferece salas descobertas, mas os resultados podem ficar desatualizados. Atualize antes de presumir que uma sala ainda está disponível. As conexões de servidores de plugins no painel inferior configuram seus próprios serviços; um plugin conectado não estabelece autenticação no TikTok ou no Cloud.

## Resultado esperado e status {#expected-result-and-status}

Conexões diretas, monitoramento e controles de conexão de plugins estão disponíveis. Reconectar ao mesmo criador preserva o contexto do feed atual; trocar de criador inicia outro contexto. O acesso real ao TikTok depende da sala e da sessão no serviço e não é garantido por um teste local bem-sucedido.

## Recuperação {#recovery}

Para salas offline, verifique se o criador está ao vivo. Para erros de sessão ou limite de requisições, siga a condição informada para tentar novamente. Cliques repetidos em reconectar não renovam a autenticação. Use a [solução de problemas](../preferences/troubleshooting.md) para coletar o componente com falha e o horário sem expor credenciais.
