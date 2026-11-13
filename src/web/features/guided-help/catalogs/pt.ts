import type { TourCatalog } from './index.ts';
export default {
  controls: { next: 'Próximo', previous: 'Anterior', done: 'Concluir guia', close: 'Fechar guia', progress: '{{current}} de {{total}}' },
  connection: {
    form: { title: 'Escolha um criador', description: 'Informe o nome e confira o estado. O guia nunca conecta nem lê seu cookie de sessão.' },
    explorer: { title: 'Explore salas LIVE', description: 'Atualize ou escolha uma sala manualmente. A conexão da demonstração é simulada; o acesso nativo depende do host e do TikTok.' },
    plugins: { title: 'Confira as integrações', description: 'Revise cada conexão instalada e sua saúde antes de ativar efeitos. Abrir o guia não contata serviços de plugins.' },
  },
  automation: {
    profiles: { title: 'Escolha o perfil de trabalho', description: 'Mantenha um perfil para as regras em revisão. Trocar ou importar perfis exige uma ação explícita no aplicativo.' },
    library: { title: 'Revise eventos e ações', description: 'Um evento vincula gatilho, filtros e intervalo a ações reutilizáveis. Mantenha efeitos novos desativados até revisar a definição.' },
    templates: { title: 'Comece com um modelo', description: 'Abra Rule templates para visualizar uma configuração. Importar e ativar exigem revisões próprias; o guia não executa essas ações.' },
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
  changes: {
    library: { title: 'Escolha os registros', description: 'Eventos, ações e variáveis têm revisões separadas. Preserve rascunhos não salvos antes de revisar outro estado.' },
    review: { title: 'Revise Changes', description: 'Changes de Automations é local. Variables revisadas usa verificações nativas de revisão; esta revisão não envia valores brutos.' },
    profiles: { title: 'Recupere explicitamente', description: 'Resolva confirmações incertas pelo diário original. Fechar o guia não conclui tarefas nem repete mutações.' },
  },
  diagnostics: {
    log: { title: 'Inspecione o histórico limitado', description: 'Filtre origem, nível ou texto e confira repetições. Revise exportações ocultadas antes de compartilhar; o histórico dura só a sessão.' },
    back: { title: 'Revise evidências e recupere', description: 'Copie ou exporte evidências revisadas e use o fluxo original para conflitos, sessões expiradas e comandos incertos. Limpar registros não corrige o erro.' },
  },
} satisfies TourCatalog;
