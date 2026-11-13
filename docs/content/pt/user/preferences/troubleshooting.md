---
title: "Diagnostique uma operação com falha"
docId: "user.preferences.troubleshooting"
translationKey: "user.preferences.troubleshooting"
locale: "pt"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
translationRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
order: 30
---

# Diagnostique uma operação com falha {#diagnose-a-failed-operation}

## Objetivo e pré-requisitos {#goal-and-prerequisites}

Identifique o componente com falha e colete um relatório útil sem disparar efeitos repetidamente. Você precisa da versão do aplicativo, do horário aproximado da falha e da operação tentada. Mantenha um [backup](backups.md) antes de redefinir ou substituir dados.

## Inspecione a evidência mais próxima {#inspect-the-nearest-evidence}

1. Leia o erro na área onde a falha ocorreu. Use sua ação de status de diagnóstico ou abra **Diagnostics** para inspecionar entradas relacionadas.
2. Filtre por origem, nível ou texto. Origens úteis incluem LIVE, monitoramento, plugins, conta, sincronização, atualizações, transporte, catálogo, instalação e colaboração.
3. Selecione a entrada relevante e guarde horário, código, identificadores de solicitação/entidade e descrição curta. Expanda os detalhes quando disponíveis.
4. Copie os diagnósticos filtrados ou exporte o relatório de texto. Se a cópia/exportação falhar, use o texto alternativo exibido. Revise antes de compartilhar; remova tokens privados, cookies, links de convite e conteúdo pessoal.
5. Inclua uma reprodução curta: página, criador/perfil/plugin selecionado, ação realizada, resultado esperado e resultado real. Explique se aconteceu antes ou depois de uma atualização.

## Isole falhas comuns {#isolate-common-failures}

Para um feed vazio, limpe pesquisa e filtros, retome a rolagem e confirme a [conexão](../live/connections.md). Rolagem pausada não é desconexão. Um indicador de perda de exibição não comprova que pontos ou análises perderam o mesmo evento.

Para uma automação sem execução, verifique perfil ativo, ativação de evento/ação, acionador, caminhos de campos dos filtros e intervalo de espera. Inspecione o histórico antes de alterar efeitos. **Test** pode executar ações de áudio, HTTP, pontos e plugins.

Para uma falha de plugin, leia o erro da tarefa de instalação ou conexão. Para o Cloud, confira o estado da conta e a área afetada em vez de substituir um cookie do TikTok. Para overlays, inspecione o status do gateway e a configuração da fonte. Para atualizações, use as [instruções de atualização manual](../updates.md) quando a instalação estiver indisponível.

## Resultado esperado e status {#expected-result-and-status}

Filtragem, cópia e exportação de texto de diagnósticos estão disponíveis. Um aplicativo empacotado também precisa dos recursos web e do ambiente de execução da plataforma. No Linux, confira o ambiente GTK/WebKitGTK compatível e a sessão X11/XWayland; no Windows, confira o WebView2. Consulte [requisitos](../requirements.md) para o escopo de instalação.

## Recuperação {#recovery}

Tente novamente após corrigir a causa específica. Para conflitos de revisão, recarregue e revise o estado salvo atual. Para um comando remoto não confirmado, inspecione seu diário/status antes de enviar outro: um tempo limite pode ocorrer após um efeito bem-sucedido. Se a inicialização falhar, informe pacote/versão e registro de inicialização em vez de excluir o diretório de dados.

<DemoFrame scenario="diagnostics" locale="pt" tour="diagnostics" title="Demonstração: diagnósticos" />
