import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationActions',
  version: 2,
  route: '/automations',
  scope: 'automations',
  tags: ['actions'],
  icon: 'rows',
  shape: 'custom',
  layout: [
    { id: 'actionsTab', target: 'automation-tab-actions', policy: 'cancel', prepare: ['[data-guide="automation-editor-close"]'], version: 2 },
    { id: 'actions', target: 'automation-actions', policy: 'skip', prepare: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-actions"]'], version: 2 },
    { id: 'editor', target: 'automation-editor', policy: 'skip', prepare: ['[data-guide="automation-new-record"]'], version: 2 },
    { id: 'templates', target: 'automation-templates', policy: 'skip' },
  ],
  steps: {
    actionsTab: {
      es: ['Abre la pestaña Acciones', 'Las acciones son efectos reutilizables compartidos por eventos. Abre la pestaña Acciones para revisarlas independientemente de cualquier disparador.'],
      en: ['Open the Actions tab', 'Actions are reusable effects shared by events. Open the Actions tab to review them independently of any trigger.'],
      pt: ['Abra a aba Ações', 'Ações são efeitos reutilizáveis compartilhados por eventos. Abra a aba Ações para revisá-las independentemente de qualquer gatilho.'],
      fr: ['Ouvrez l\'onglet Actions', 'Les actions sont des effets réutilisables partagés par les événements. Ouvrez l\'onglet Actions pour les examiner indépendamment de tout déclencheur.'],
      ja: ['アクションタブを開く', 'アクションはイベント間で共有される再利用可能な効果です。アクションタブを開いて、トリガーとは独立に確認できます。'],
      zh: ['打开操作选项卡', '操作是事件之间共享的可重用效果。打开操作选项卡,可以独立于任何触发器来审查它们。'],
    },
    actions: {
      es: ['Lee la tabla de acciones', 'Cada fila es una acción reutilizable con su tipo y destino. Editar una acción actualiza todos los eventos que la vinculan.'],
      en: ['Read the actions table', 'Each row is one reusable action with its type and target. Editing an action updates every event that links it.'],
      pt: ['Leia a tabela de ações', 'Cada linha é uma ação reutilizável com seu tipo e destino. Editar uma ação atualiza todos os eventos que a vinculam.'],
      fr: ['Lisez le tableau des actions', 'Chaque ligne est une action réutilisable avec son type et sa cible. Modifier une action met à jour tous les événements qui la lient.'],
      ja: ['アクションテーブルを読む', '各行はタイプと対象を持つ再利用可能なアクションです。アクションを編集すると、それをリンクするすべてのイベントが更新されます。'],
      zh: ['查看操作表', '每行是一个具有类型和目标的可重用操作。编辑一个操作会更新所有关联它的事件。'],
    },
    editor: {
      es: ['Configura la acción', 'El editor define qué hace el efecto: texto, sonido, overlay o puntos. Las acciones de prueba nunca se ejecutan durante la guía.'],
      en: ['Configure the action', 'The editor sets what the effect does: text, sound, overlay or points. Test actions never fire during the guide.'],
      pt: ['Configure a ação', 'O editor define o que o efeito faz: texto, som, overlay ou pontos. Ações de teste nunca disparam durante o guia.'],
      fr: ['Configurez l\'action', 'L\'éditeur définit ce que fait l\'effet : texte, son, overlay ou points. Les actions de test ne se déclenchent jamais pendant le guide.'],
      ja: ['アクションを設定', 'エディターは効果の内容を設定します:テキスト、サウンド、オーバーレイ、ポイント。テストアクションはガイド中には実行されません。'],
      zh: ['配置操作', '编辑器设置效果的内容:文本、声音、叠加层或积分。测试操作在指南期间绝不会触发。'],
    },
    templates: {
      es: ['Empieza con una plantilla', 'Las plantillas de reglas previsualizan una configuración reutilizable. Importar y activar requieren revisión propia; la guía no hace ninguna de las dos cosas.'],
      en: ['Start from a template', 'Rule templates preview a reusable configuration. Importing and enabling require their own review; the guide does neither.'],
      pt: ['Comece com um modelo', 'Os modelos de regras permitem visualizar uma configuração reutilizável. Importar e ativar exigem revisão própria; o guia não faz nenhuma das duas coisas.'],
      fr: ['Partez d\'un modèle', 'Les modèles de règles prévisualisent une configuration réutilisable. L\'importation et l\'activation exigent leur propre révision ; le guide ne fait ni l\'une ni l\'autre.'],
      ja: ['テンプレートから始める', 'ルールテンプレートは再利用可能な設定をプレビューします。インポートと有効化には独自のレビューが必要で、ガイドはどちらも行いません。'],
      zh: ['从模板开始', '规则模板可预览可重用的配置。导入和启用需要各自的审查;指南两者都不执行。'],
    },
  },
  card: {
      es: ['Automatizaciones: acciones', 'Efectos reutilizables, configuración del editor y plantillas.'],
      en: ['Automations: actions', 'Reusable effects, editor configuration and rule templates.'],
      pt: ['Automações: ações', 'Efeitos reutilizáveis, configuração do editor e modelos.'],
      fr: ['Automatisations : actions', 'Effets réutilisables, configuration de l\'éditeur et modèles.'],
      ja: ['オートメーション:アクション', '再利用可能な効果、エディター設定、ルールテンプレート。'],
      zh: ['自动化:操作', '可重用效果、编辑器配置和规则模板。'],
    },
});
