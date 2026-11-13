---
title: "Use o TikTools"
docId: "user.overview"
translationKey: "user.overview"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
translationRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
order: 0
---

# Use o TikTools {#use-tiktools}

O TikTools conecta um desktop Windows ou Linux a uma LIVE do TikTok, exibe seus eventos, mantém pontos dos espectadores e executa regras no seu computador. A interface de desktop e o host opcional sem interface gráfica compartilham o mesmo núcleo Rust. Uma conta do Cloud é independente da conexão com o TikTok.

## Escolha sua primeira tarefa {#choose-your-first-task}

1. Confira os [requisitos](requirements.md) e [instale um binário publicado](installation.md). Você não precisa de Rust, Cargo ou Bun para usar a versão de desktop.
2. [Conecte-se a uma LIVE](first-live.md), começando sem cookie do TikTok quando a sala permitir acesso de visitante.
3. Use **Live feed** para inspecionar eventos. **Points** mantém os totais dos espectadores; **Analytics** mostra estatísticas. **Automations** conecta acionadores a ações configuradas.
4. Instale integrações opcionais em **Plugins**. Os pacotes de plugins externos são distribuídos separadamente do aplicativo.
5. Use **Overlays** para preparar uma fonte de navegador no OBS. A integração com o gateway é necessária para entregar widgets ao vivo.

## Recursos disponíveis e experimentais {#available-and-experimental-features}

A navegação principal expõe Live feed, Points, Analytics, Connections, Automations, Plugins, Overlays e Settings. Rankings e competições têm um mecanismo nativo e uma rota direta, mas sua página de gerenciamento ainda está oculta na navegação normal. Não presuma que todo protótipo descrito em um documento de projeto antigo seja um recurso lançado.

Alterações de plugins do Cloud e Variables revisadas usam controles explícitos por seção. Automations atualmente compara com uma cópia de referência local; esse botão não publica perfis no Cloud. Os transportes de colaboração e alguns encaminhamentos para instaladores continuam experimentais. Um controle desativado normalmente indica uma capacidade ausente no host ou um pré-requisito não atendido.

## Quando algo falha {#when-something-fails}

Abra **Diagnostics**, guarde a origem do erro e o identificador da solicitação e registre a versão instalada do aplicativo. Evite incluir cookies, tokens de convite ou valores de variáveis privadas. Comece pelas instruções de recuperação de cada guia e depois consulte os guias de suporte. A demonstração interativa usa dados fictícios e não pode determinar se sua conta do TikTok, o OBS ou a rede funcionam.

Para desenvolver o código-fonte, use a [documentação de desenvolvimento](../developer/index.md) separada.
