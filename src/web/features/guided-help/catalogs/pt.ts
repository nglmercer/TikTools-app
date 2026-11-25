import type { TourCatalog } from './index.ts';
export default {
  controls: { next: 'Próximo', previous: 'Anterior', done: 'Concluir guia', close: 'Fechar guia', progress: '{{current}} de {{total}}' },
  connection: {
    form: { title: 'Escolha um criador', description: 'Informe o nome e confira o estado. O guia nunca conecta nem lê seu cookie de sessão.' },
    explorer: { title: 'Explore salas LIVE', description: 'Atualize ou escolha uma sala manualmente. A conexão da demonstração é simulada; o acesso nativo depende do host e do TikTok.' },
    plugins: { title: 'Confira as integrações', description: 'Revise cada conexão instalada e sua saúde antes de ativar efeitos. Abrir o guia não contata serviços de plugins.' },
  },
  plugin: {
    catalog: { title: 'Instalados e Explore', description: 'Revise pacotes instalados ou explore itens compatíveis. Plugins nativos e de processo mantêm as permissões do usuário no sistema.' },
    search: { title: 'Encontre um pacote', description: 'Pesquise e confira editor, versão e requisitos. Catálogo vazio e backend indisponível são estados diferentes.' },
    install: { title: 'Revise a instalação', description: 'Instalar exige revisão de assinaturas e permissões. O guia não instala, aprova permissões nem executa ações de teste.' },
  },
  overlay: {
    gateway: { title: 'Confira o gateway', description: 'A saída ao vivo requer event-gateway e credencial de widget. A prévia local não comprova conexão com OBS.' },
    preview: { title: 'Visualize localmente', description: 'A prévia usa o mesmo renderizador com eventos de exemplo. Não precisa de LIVE real nem envia alertas ao OBS.' },
    design: { title: 'Revise o design', description: 'Edite e salve intencionalmente; depois copie uma URL atualizada para OBS. Changes de Overlays continua como trabalho não integrado.' },
  },
  points: {
    rules: { title: 'Revise as regras de pontos', description: 'Confira os valores antes de alterá-los. Alguns controles ficam bloqueados durante uma LIVE conectada ou ocupada.' },
    leaderboard: { title: 'Confira pontos confirmados', description: 'Pesquise ou ordene espectadores. Atualizar a exibição não ajusta nem redefine pontos e não repete eventos LIVE.' },
  },
  automationEvents: {
    profiles: { title: 'Escolha o perfil de trabalho', description: 'Eventos e ações pertencem a um perfil. Revise o perfil que você realmente usa antes de alterar qualquer regra.' },
    eventsTab: { title: 'Abra a aba Eventos', description: 'Eventos são as regras: cada um vincula um gatilho LIVE às ações que dispara. A aba Eventos é a visão padrão.' },
    events: { title: 'Leia a tabela de eventos', description: 'Cada linha mostra o gatilho, suas condições e as ações vinculadas. Mantenha novos eventos desativados até revisar sua definição.' },
    editor: { title: 'Edite um evento por vez', description: 'O editor define o gatilho, os filtros, o cooldown e as ações vinculadas. Salvar é explícito; nada dispara durante o guia.' },
    runs: { title: 'Verifique com o histórico', description: 'As execuções recentes mostram o que disparou, o que foi ignorado e por quê. Confirme cooldowns e filtros aqui antes de ativar um evento.' },
  },
  automationActions: {
    actionsTab: { title: 'Abra a aba Ações', description: 'Ações são efeitos reutilizáveis compartilhados por eventos. Abra a aba Ações para revisá-las independentemente de qualquer gatilho.' },
    actions: { title: 'Leia a tabela de ações', description: 'Cada linha é uma ação reutilizável com seu tipo e destino. Editar uma ação atualiza todos os eventos que a vinculam.' },
    editor: { title: 'Configure a ação', description: 'O editor define o que o efeito faz: texto, som, overlay ou pontos. Ações de teste nunca disparam durante o guia.' },
    templates: { title: 'Comece com um modelo', description: 'Os modelos de regras permitem visualizar uma configuração reutilizável. Importar e ativar exigem revisão própria; o guia não faz nenhuma das duas coisas.' },
  },
  automationVariables: {
    variablesTab: { title: 'Abra a aba Variáveis', description: 'As variáveis vivem com as automações. Abra a aba Variáveis para revisar valores compartilhados por eventos e ações.' },
    values: { title: 'Revise valores compartilhados', description: 'Os segredos permanecem somente de escrita e nunca são exibidos. Alterar um valor afeta cada evento e ação que o referencia.' },
    review: { title: 'Revise Alterações antes de sincronizar', description: 'Os valores locais e da Cloud são comparados em Alterações. Revise as diferenças antes de aplicar qualquer direção.' },
  },
  changes: {
    library: { title: 'Escolha os registros', description: 'Eventos, ações e variáveis têm revisões separadas. Preserve rascunhos não salvos antes de revisar outro estado.' },
    review: { title: 'Revise Changes', description: 'Changes de Automations é local. Variables revisadas usa verificações nativas de revisão; esta revisão não envia valores brutos.' },
    profiles: { title: 'Recupere explicitamente', description: 'Resolva confirmações incertas pelo diário original. Fechar o guia não conclui tarefas nem repete mutações.' },
  },
  diagnostics: {
    log: { title: 'Inspecione o histórico limitado', description: 'Filtre origem, nível ou texto e confira repetições. Revise exportações ocultadas antes de compartilhar; o histórico dura só a sessão.' },
    back: { title: 'Revise evidências e recupere', description: 'Copie ou exporte evidências revisadas e use o fluxo original para conflitos, sessões expiradas e comandos incertos. Limpar registros não corrige o erro.' },
  },
  feed: {
    view: { title: 'Escolha a visualização do feed', description: 'Feed ao vivo mostra chat, presentes e seguidores como cartões; mude para Lista para ver o registro compacto.' },
    filters: { title: 'Filtre a atividade', description: 'Filtre por tipo ou busque por espectador e mensagem. Os filtros também se aplicam aos cartões do feed.' },
    stage: { title: 'Leia o mural ao vivo', description: 'Mensagens importantes permanecem. Curtidas e entradas aparecem brevemente como reações e depois desaparecem.' },
    controls: { title: 'Pausar, limpar e inspecionar', description: 'Pause a rolagem para ler com calma, limpe o histórico salvo ou abra o inspetor para ver detalhes.' },
  },
  backup: {
    plan: { title: 'Conheça seus limites', description: 'Cada plano tem seus próprios limites de armazenamento e de backups. Estes medidores mostram quanto você usa.' },
    tools: { title: 'Nuvem ou este PC', description: 'Alterne entre salvar na sua conta na nuvem ou trabalhar com arquivos neste PC: exporte seus dados ou envie um arquivo que você já tem.' },
    create: { title: 'Salve um backup', description: 'Dê um nome ao backup e salve na sua conta. Inclui regras, perfis, ajustes e arquivos na nuvem, nunca segredos nem cookies.' },
    list: { title: 'Baixe, restaure ou exclua', description: 'Cada backup salvo pode ser baixado, restaurado em qualquer dispositivo com esta conta ou excluído. Restaurar sempre mostra antes o que vai mudar.' },
  },
  analytics: {
    range: { title: "Escolha o período", description: "Escolha uma predefinição ou datas personalizadas. O resumo é recarregado para o criador ativo; mudar o período nunca altera os eventos guardados." },
    metrics: { title: "Escolha a métrica", description: "Os cartões comparam chats, presentes, gostos e entradas. A métrica selecionada controla o gráfico de atividade sem modificar os dados de análise." },
    activity: { title: "Leia o gráfico de atividade", description: "Use o gráfico para identificar dias e horários ativos. Períodos vazios podem significar que não houve LIVE ou que não houve eventos correspondentes." },
    facts: { title: "Verifique os dados principais", description: "Totais e contagem de sessões oferecem uma visão compacta antes de comparar espectadores individuais." },
    contributors: { title: "Busque colaboradores", description: "Ordene ou pesquise a tabela para encontrar os espectadores mais ativos. Esta é uma classificação somente de leitura." },
  },
  settings: {
    sections: { title: "Escolha uma seção de configurações", description: "Geral contém preferências do aplicativo. Conta contém perfil Cloud, plano e ferramentas de backup." },
    language: { title: "Mude o idioma", description: "O idioma é aplicado imediatamente à interface. Ele não altera dados da LIVE nem o comportamento das automações." },
    review: { title: "Escolha o modo de revisão", description: "O modo de revisão controla quanta confirmação os fluxos sensíveis de sincronização exigem. Modos mais seguros exigem revisão mais explícita." },
    updater: { title: "Verifique atualizações manualmente", description: "Use o cartão do atualizador para ver o canal e procurar atualizações. Se as atualizações não estiverem disponíveis, baixe a versão pelo caminho de recuperação documentado." },
  },
  overlayObs: {
    gateway: { title: "Comece pelo gateway", description: "Os widgets do OBS recebem eventos pelo gateway local. Confirme que ele está ativo antes de copiar qualquer URL de fonte." },
    widget: { title: "Escolha o widget", description: "Escolha a fonte de follows, presentes, chat, likes ou ranking que você quer colocar no OBS." },
    preview: { title: "Teste com a pré-visualização", description: "Reproduza eventos de exemplo e ajuste o design localmente. A pré-visualização não prova que o OBS está conectado." },
    url: { title: "Copie a URL do OBS", description: "Copie a URL da fonte somente depois que o gateway estiver pronto e cole-a em uma Browser Source do OBS com o tamanho desejado." },
    design: { title: "Ajuste movimento e aparência", description: "O editor de design agora inclui estilo de animação, duração, easing e stagger. Salve antes de copiar uma URL atualizada." },
  },
  cloudOverlays: {
    open: { title: "Abra a biblioteca Cloud", description: "Quando os overlays Cloud estiverem disponíveis, explore documentos compartilhados pelo botão da biblioteca Cloud." },
    widgets: { title: "Selecione um destino local", description: "Escolha qual widget local deve receber o design Cloud selecionado antes de aplicar qualquer coisa." },
    preview: { title: "Pré-visualize antes de publicar", description: "Use a pré-visualização local para comparar o design selecionado. Publicar ou substituir uma cópia Cloud continua sendo uma ação explícita." },
    url: { title: "Copie a fonte correta", description: "URLs do gateway local e URLs de overlays Cloud resolvem casos de recuperação diferentes. Verifique qual delas sua cena do OBS deve usar." },
  },
  rankings: {
    tabs: { title: "Rankings ou competições", description: "Rankings acumulam pontuações. Competições comparam rankings selecionados durante um evento limitado." },
    list: { title: "Escolha ou crie um ranking", description: "A lista mostra métrica, modo e estado. Crie um novo ranking somente depois de decidir qual ação do espectador deve pontuar." },
    detail: { title: "Inspecione o placar ao vivo", description: "A visão de detalhe mostra pontuação, estado de pausa e integridade. Pausar, retomar e reiniciar são operações explícitas." },
    competitions: { title: "Configure competições", description: "Competições precisam de rankings compatíveis e limites claros. Elas são experimentais; verifique o overlay antes de usá-las no ar." },
  },
  pluginSetup: {
    catalog: { title: "Abra o plugin instalado", description: "Comece em Instalados, não em Explorar, ao configurar um plugin já confiável nesta máquina." },
    details: { title: "Revise permissões e estado", description: "O cabeçalho de detalhes mostra estado de ativação, versão e ações que afetam o runtime. Revise permissões antes de ativar." },
    settings: { title: "Configure os ajustes", description: "A aba Configurações contém valores específicos do plugin. Salve primeiro quando o plugin pedir antes de testar." },
    connection: { title: "Conecte e teste", description: "A aba Conexão gerencia credenciais de serviços externos e oferece uma ação de teste quando o plugin a fornece." },
  },
  connectionRecovery: {
    form: { title: "Verifique novamente criador e estado", description: "Comece com o handle exato do criador e o estado atual da conexão. Offline, autenticação e sala indisponível exigem recuperações diferentes." },
    explorer: { title: "Verifique a sala LIVE", description: "Atualize a lista de salas e confirme que a sala está acessível. Uma sala ausente pode estar offline, restrita ou simplesmente indisponível para convidados." },
    plugins: { title: "Verifique integrações após reconectar", description: "As integrações de conexão podem demorar após uma reconexão. Revise a saúde delas antes de assumir que automações ou overlays falharam." },
  },
  feedInspector: {
    controls: { title: "Abra o inspetor", description: "Use o controle do inspetor na barra do feed. Ele não pausa a LIVE nem altera eventos guardados." },
    panel: { title: "Entenda o painel", description: "O inspetor tem abas de espectadores, detalhes do evento e perfil. Seu conteúdo acompanha o evento ou espectador selecionado." },
    viewers: { title: "Compare espectadores ativos", description: "Alterne entre principais espectadores e likes para ver quem está impulsionando a atividade da sessão atual." },
    details: { title: "Inspecione um evento", description: "Os detalhes do evento incluem contagens, pontos, horário e evidências da mensagem úteis para diagnosticar automações." },
    profile: { title: "Abra um perfil de espectador", description: "A aba de perfil resume o espectador e suas mensagens recentes sem expor dados privados da conta." },
  },
  backupRestore: {
    tools: { title: "Escolha a fonte de restauração", description: "Backups Cloud e arquivos locais são caminhos separados de recuperação. Escolha a fonte antes de revisar o que vai mudar." },
    list: { title: "Restaure deliberadamente", description: "Uma linha de backup pode baixar, restaurar ou excluir. Restaurar sempre mostra uma revisão dos dados afetados antes de aplicar." },
    plan: { title: "Verifique o resultado", description: "Após a restauração, verifique plano, perfis, configurações e automações principais. Uma restauração bem-sucedida não revalida todas as credenciais externas." },
  },
} satisfies TourCatalog;
