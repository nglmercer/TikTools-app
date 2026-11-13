---
title: "Primeira conexão com uma LIVE"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
translationRevision: "3dc7841a31f1b9717f9ffe0cc116c25b933b3470ab98a99b6c16399db922bfe0"
order: 30
---

# Primeira conexão com uma LIVE {#first-live-connection}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Receba eventos de um criador antes de ativar automações. Você precisa de uma instalação de desktop funcional, acesso à rede e um criador que esteja ao vivo. O login no Cloud é uma conexão separada e não autentica o TikTok.

## Conecte-se {#connect}

Comece pelo painel abaixo: use `demo_creator` para praticar e observe o estado depois de selecionar **Conectar**. Os controles são os mesmos da aplicação; os dados são fictícios.

<DemoFrame scenario="connection" locale="pt" panel="connection" title="Pratique sua primeira conexão" />

<DocSteps>

1. Abra **Conexões** ou a ação **Conectar** na barra de status inferior.
2. Digite o identificador do criador. O `@` inicial é opcional. Confira a grafia em vez de usar o nome de exibição.
3. Comece com **Usar um cookie de sessão** desativado quando o acesso de visitante for possível. Se a sala exigir sua sessão autorizada, ative esse controle e forneça o cookie apenas no campo mascarado.
4. Deixe o monitoramento desligado para esta primeira conexão direta. Pressione **Conectar** e aguarde o estado da conexão se estabilizar.
5. Abra **Atividade ao vivo**. Peça uma mensagem de chat inofensiva ou aguarde um evento real. Confira o criador e o horário.
6. Selecione um evento para inspecionar os detalhes. Selecione o nome ou avatar de um espectador para inspecionar seu perfil acompanhado.

</DocSteps>

## Resultado esperado {#expected-result}

O status muda para conectado e os eventos recebidos aparecem no feed. Uma sala sem atividade pode legitimamente não produzir novos eventos. Os pontos dependem das regras de pontuação ativas, e os resultados das automações dependem das ações e dos eventos configurados.


<DemoFrame scenario="feed" locale="pt" panel="feed" title="Inspecione eventos no feed de exemplo" />

<DocCallout title="Confirme o resultado" tone="tip">

Confira o criador, o horário e os detalhes de um evento. Uma conexão estabelecida sem atividade na sala pode deixar o feed vazio.

</DocCallout>

## Monitoramento e reconexão {#monitoring-and-reconnecting}

Após uma conexão direta bem-sucedida, **Monitorar e conectar automaticamente** pode aguardar o criador entrar ao vivo. Seu status distingue espera, verificação e atividade de conexão. Pare o monitoramento para editar um campo de criador bloqueado. Reconectar ao mesmo criador mantém o feed atual; trocar de criador inicia outro contexto de feed.

## Recuperação {#recovery}

Para um criador offline, confirme que a sala está ao vivo antes de tentar novamente. Para erros de autenticação ou limite de requisições, inspecione **Diagnósticos** e siga a condição informada para nova tentativa; cliques repetidos não corrigem uma sessão expirada no serviço. Se precisar de um cookie, obtenha uma sessão autorizada nova e mantenha seu valor fora de capturas de tela, registros e solicitações de suporte. Ele permanece na memória, não é uma credencial reutilizável do Cloud.

Desconecte antes de testar outro criador. A demonstração ensina esses controles com eventos fictícios, mas seu sucesso não valida uma sessão do TikTok.

<DocCallout title="Proteja sua sessão" tone="warning">

Use apenas uma sessão autorizada. Não cole cookies na demonstração nem em solicitações de suporte. O exemplo interativo simula a conexão sem autenticar no TikTok.

</DocCallout>
