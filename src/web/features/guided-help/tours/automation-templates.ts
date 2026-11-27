import { automationSelector, defineTour } from '../model/define-tour.ts';
import { guideAction, guideScope } from '../model/guide-vocabulary.ts';

export default defineTour({
  id: 'automationTemplates',
  version: 1,
  route: '/automations',
  scope: guideScope.automations,
  tags: ['events', 'actions'],
  icon: 'copy',
  shape: 'custom',
  layout: [
    { id: 'gallery', target: 'automation-templates', policy: 'cancel', prepare: [automationSelector.editorClose] },
    { id: 'events', target: 'automation-events', policy: 'skip', prepareActions: [guideAction.automation.selectEvents], prepare: [automationSelector.editorClose, automationSelector.tabEvents] },
    { id: 'review', target: 'event-editor-trigger', policy: 'skip', prepareActions: [guideAction.automation.openEvent], prepare: [automationSelector.editorClose, automationSelector.tabEvents, automationSelector.firstEventRow] },
  ],
  steps: {
    gallery: {
      es: ['Empieza con una plantilla', 'Las plantillas son configuraciones listas para revisar: moderación, metas de directo y agradecimientos comunes.'],
      en: ['Start from a template', 'Templates are ready-to-review configurations: moderation, stream goals and common thank-yous.'],
      pt: ['Comece com um modelo', 'Modelos são configurações prontas para revisar: moderação, metas de live e agradecimentos comuns.'],
      fr: ['Partez d’un modèle', 'Les modèles sont des configurations prêtes à réviser : modération, objectifs de direct et remerciements courants.'],
      ja: ['テンプレートから開始', 'テンプレートは確認用の設定済み構成です: モデレーション、配信目標、一般的なお礼。'],
      zh: ['从模板开始', '模板是可检查的配置:  moderation、直播目标和常见感谢。'],
    },
    events: {
      es: ['Revisa lo importado', 'Importar una plantilla crea eventos y acciones desactivados. Nada se ejecuta hasta que los revises y actives por tu cuenta.'],
      en: ['Review what was imported', 'Importing a template creates disabled events and actions. Nothing runs until you review and enable them yourself.'],
      pt: ['Revise o importado', 'Importar um modelo cria eventos e ações desativados. Nada executa até você revisar e ativar.'],
      fr: ['Vérifiez l’import', 'Importer un modèle crée des événements et actions désactivés. Rien ne s’exécute avant votre révision.'],
      ja: ['インポート内容を確認', 'インポートは無効化されたイベントとアクションを作成します。確認して有効化するまで実行されません。'],
      zh: ['检查导入内容', '导入模板创建停用的事件和操作,检查并启用前不会执行。'],
    },
    review: {
      es: ['Ajusta antes de activar', 'Abre cada registro importado y revisa disparador, filtros y acciones. Las plantillas son un punto de partida, no una configuración final.'],
      en: ['Adjust before enabling', 'Open each imported record and review trigger, filters and actions. Templates are a starting point, not a final configuration.'],
      pt: ['Ajuste antes de ativar', 'Abra cada registro importado e revise gatilho, filtros e ações. Modelos são ponto de partida, não configuração final.'],
      fr: ['Ajustez avant d’activer', 'Ouvrez chaque enregistrement importé et vérifiez déclencheur, filtres et actions. Les modèles sont un point de départ.'],
      ja: ['有効化前に調整', 'インポートした各レコードを開き、トリガー・フィルタ・アクションを確認。テンプレートは出発点です。'],
      zh: ['启用前调整', '打开每条导入记录检查触发器、过滤器和操作。模板是起点,不是最终配置。'],
    },
  },
  card: {
      es: ['Plantillas de reglas', 'Configuraciones listas para revisar, importar y ajustar.'],
      en: ['Rule templates', 'Configurations ready to review, import and adjust.'],
      pt: ['Modelos de regras', 'Configurações prontas para revisar, importar e ajustar.'],
      fr: ['Modèles de règles', 'Configurations prêtes à réviser, importer et ajuster.'],
      ja: ['ルールテンプレート', '確認・インポート・調整できる構成。'],
      zh: ['规则模板', '可检查、导入和调整的配置。'],
    },
});
