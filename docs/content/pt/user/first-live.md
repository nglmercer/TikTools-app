---
title: "Primeira conexão com uma LIVE"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fb7101f2010cd57478f9f6b2ea9cbb8c0a6401975fe4b29b311c41b13c1a5185"
translationRevision: "843e1c80a343c25f0e22c75ce350b5cdb34e94827a0b98f307f2b7ab6474e620"
order: 30
---

# Primeira conexão com uma LIVE {#first-live-connection}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Receba eventos de um criador antes de ativar automações. Você precisa de uma instalação de desktop funcional, acesso à rede e um criador que esteja ao vivo. O login no Cloud é uma conexão separada e não autentica o TikTok.

## Conecte-se {#connect}

1. Abra **Connections** ou a ação **Connect** na barra de status inferior.
2. Digite o identificador do criador. O `@` inicial é opcional. Confira a grafia em vez de usar o nome de exibição.
3. Comece com **Use Cookie** desativado quando o acesso de visitante for possível. Se a sala exigir sua sessão autorizada, ative esse controle e forneça o cookie apenas no campo mascarado.
4. Deixe o monitoramento desligado para esta primeira conexão direta. Pressione **Connect** e aguarde o estado da conexão se estabilizar.
5. Abra **Live feed**. Peça uma mensagem de chat inofensiva ou aguarde um evento real. Confira o criador e o horário.
6. Selecione um evento para inspecionar os detalhes. Selecione o nome ou avatar de um espectador para inspecionar seu perfil acompanhado.

## Resultado esperado {#expected-result}

O status muda para conectado e os eventos recebidos aparecem no feed. Uma sala sem atividade pode legitimamente não produzir novos eventos. Os pontos dependem das regras de pontuação ativas, e os resultados das automações dependem das ações e dos eventos configurados.

## Monitoramento e reconexão {#monitoring-and-reconnecting}

Após uma conexão direta bem-sucedida, **Monitor and auto-connect** pode aguardar o criador entrar ao vivo. Seu status distingue espera, verificação e atividade de conexão. Pare o monitoramento para editar um campo de criador bloqueado. Reconectar ao mesmo criador mantém o feed atual; trocar de criador inicia outro contexto de feed.

## Recuperação {#recovery}

Para um criador offline, confirme que a sala está ao vivo antes de tentar novamente. Para erros de autenticação ou limite de requisições, inspecione **Diagnostics** e siga a condição informada para nova tentativa; cliques repetidos não corrigem uma sessão expirada no serviço. Se precisar de um cookie, obtenha uma sessão autorizada nova e mantenha seu valor fora de capturas de tela, registros e solicitações de suporte. Ele permanece na memória, não é uma credencial reutilizável do Cloud.

Desconecte antes de testar outro criador. A demonstração ensina esses controles com eventos fictícios, mas seu sucesso não valida uma sessão do TikTok.

<DemoFrame scenario="connection" locale="pt" tour="connection" title="Demonstração: conexão" />
