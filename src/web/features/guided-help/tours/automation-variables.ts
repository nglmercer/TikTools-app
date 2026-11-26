import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationVariables',
  version: 2,
  route: '/automations?view=variables',
  scope: 'automations',
  tags: ['variables'],
  icon: 'hash',
  shape: 'custom',
  layout: [
    { id: 'variablesTab', target: 'automation-tab-variables', policy: 'cancel', prepareActions: ['automation.selectVariables'] },
    { id: 'values', target: 'automation-variables', policy: 'skip', prepareActions: ['automation.selectVariables'], prepare: ['[data-guide="automation-editor-close"]', '[data-guide="automation-tab-variables"]'], version: 2 },
    { id: 'review', target: 'changes', policy: 'skip' },
  ],
  steps: {
    variablesTab: {
      es: ['Abre la pestaña Variables', 'Las variables viven con las automatizaciones. Abre la pestaña Variables para revisar valores compartidos por eventos y acciones.'],
      en: ['Open the Variables tab', 'Variables live with automations. Open the Variables tab to review values shared by events and actions.'],
      pt: ['Abra a aba Variáveis', 'As variáveis vivem com as automações. Abra a aba Variáveis para revisar valores compartilhados por eventos e ações.'],
      fr: ['Ouvrez l\'onglet Variables', 'Les variables vivent avec les automatisations. Ouvrez l\'onglet Variables pour examiner les valeurs partagées par les événements et les actions.'],
      ja: ['変数タブを開く', '変数はオートメーションと一緒に存在します。変数タブを開いて、イベントとアクションで共有される値を確認してください。'],
      zh: ['打开变量选项卡', '变量与自动化共存。打开变量选项卡,查看事件和操作共享的值。'],
    },
    values: {
      es: ['Revisa valores compartidos', 'Los secretos permanecen de solo escritura y nunca se muestran. Cambiar un valor afecta a cada evento y acción que lo referencia.'],
      en: ['Review shared values', 'Secrets stay write-only and never render. Changing a value affects every event and action that references it.'],
      pt: ['Revise valores compartilhados', 'Os segredos permanecem somente de escrita e nunca são exibidos. Alterar um valor afeta cada evento e ação que o referencia.'],
      fr: ['Vérifiez les valeurs partagées', 'Les secrets restent en écriture seule et ne s\'affichent jamais. Modifier une valeur affecte chaque événement et action qui la référence.'],
      ja: ['共有値を確認', 'シークレットは書き込み専用のままで、表示されることはありません。値を変更すると、それを参照するすべてのイベントとアクションに影響します。'],
      zh: ['检查共享值', '机密保持只写状态,永不显示。更改一个值会影响引用它的每个事件和操作。'],
    },
    review: {
      es: ['Revisa Cambios antes de sincronizar', 'Los valores locales y de Cloud se comparan en Cambios. Revisa las diferencias antes de aplicar cualquiera de las dos direcciones.'],
      en: ['Review Changes before syncing', 'Local and Cloud values are compared in Changes. Review the differences before applying either direction.'],
      pt: ['Revise Alterações antes de sincronizar', 'Os valores locais e da Cloud são comparados em Alterações. Revise as diferenças antes de aplicar qualquer direção.'],
      fr: ['Vérifiez Modifications avant de synchroniser', 'Les valeurs locales et Cloud sont comparées dans Modifications. Examinez les différences avant d\'appliquer l\'une ou l\'autre direction.'],
      ja: ['同期前に変更を確認', 'ローカルとCloudの値は「変更」で比較されます。どちらの方向も適用する前に差分を確認してください。'],
      zh: ['同步前检查变更', '本地和 Cloud 的值在“变更”中进行比较。在应用任一方向之前,先检查差异。'],
    },
  },
  card: {
      es: ['Automatizaciones: variables', 'Valores compartidos, secretos de solo escritura y revisión local/Cloud.'],
      en: ['Automations: variables', 'Shared values, write-only secrets and local versus Cloud review.'],
      pt: ['Automações: variáveis', 'Valores compartilhados, segredos somente de escrita e revisão local/Cloud.'],
      fr: ['Automatisations : variables', 'Valeurs partagées, secrets en écriture seule et révision local/Cloud.'],
      ja: ['オートメーション:変数', '共有値、書き込み専用シークレット、ローカルとCloudのレビュー。'],
      zh: ['自动化:变量', '共享值、只写机密和本地与 Cloud 审查。'],
    },
});
