import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'changes',
  version: 2,
  route: '/automations?view=variables',
  scope: 'automations',
  task: 'changes.reviewed',
  tags: ['changes'],
  icon: 'git-merge',
  shape: 'custom',
  layout: [
    { id: 'library', target: 'automation-library', policy: 'cancel' },
    { id: 'review', target: 'changes', policy: 'cancel' },
    { id: 'profiles', target: 'automation-profiles', policy: 'skip' },
  ],
  steps: {
    library: {
      es: ['Elige los registros', 'Eventos, acciones y variables tienen límites de revisión separados. Conserva los candidatos sin guardar antes de revisar otro estado.'],
      en: ['Choose the records', 'Events, actions and variables have separate review boundaries. Unsaved editor candidates must be preserved before reviewing another state.'],
      pt: ['Escolha os registros', 'Eventos, ações e variáveis têm revisões separadas. Preserve rascunhos não salvos antes de revisar outro estado.'],
      fr: ['Choisir les enregistrements', 'Événements, actions et variables ont des examens distincts. Conservez les brouillons avant d’examiner un autre état.'],
      ja: ['記録を選択', 'イベント、アクション、変数はそれぞれ別に確認します。別の状態を確認する前に、エディター内の未保存の候補を保持してください。'],
      zh: ['选择记录', '事件、操作和变量有各自独立的审查边界。审查其他状态前，必须保留编辑器中未保存的候选内容。'],
    },
    review: {
      es: ['Revisa Changes', 'Changes de Automations es local. Variables revisadas utiliza comprobaciones nativas de revisión; esta revisión no sube valores crudos.'],
      en: ['Review Changes', 'Automations Changes is local-only. Reviewed Variables uses native revision checks; raw variable values are not uploaded by this review.'],
      pt: ['Revise Changes', 'Changes de Automations é local. Variables revisadas usa verificações nativas de revisão; esta revisão não envia valores brutos.'],
      fr: ['Examiner Changes', 'Changes des automatisations reste local. Variables révisées utilise des contrôles natifs de version ; cet examen ne téléverse pas les valeurs brutes.'],
      ja: ['変更を確認', '「自動化の変更」はローカル専用です。確認した変数にはネイティブのリビジョン確認を使用します。この確認で変数の生の値はアップロードされません。'],
      zh: ['审查更改', '“自动化更改”仅在本地进行。“已审查变量”使用原生修订版检查；本次审查不会上传原始变量值。'],
    },
    profiles: {
      es: ['Mantén la recuperación explícita', 'Resuelve una confirmación incierta mediante su diario original. Cerrar la guía no completa tareas ni reintenta mutaciones.'],
      en: ['Keep recovery explicit', 'An unknown acknowledgement must be resolved through its original journal. Dismissing this guide neither completes a task nor retries a mutation.'],
      pt: ['Recupere explicitamente', 'Resolva confirmações incertas pelo diário original. Fechar o guia não conclui tarefas nem repete mutações.'],
      fr: ['Récupérer explicitement', 'Résolvez une confirmation incertaine avec le journal d’origine. Fermer le guide ne termine aucune tâche et ne répète aucune mutation.'],
      ja: ['復旧を明示的に行う', '受領確認が不明な場合は、元の操作記録から解決する必要があります。このガイドを閉じても、タスクの完了や変更操作の再試行は行いません。'],
      zh: ['明确执行恢复', '未知确认结果必须通过其原始日志处理。关闭本指南既不会完成任务，也不会重试更改操作。'],
    },
  },
  card: {
      es: ['Cambios de biblioteca', 'Revisa los cambios y conflictos pendientes antes de aplicarlos.'],
      en: ['Library changes', 'Review pending changes and conflicts before applying them.'],
      pt: ['Alterações da biblioteca', 'Revise as alterações e conflitos pendentes antes de aplicá-los.'],
      fr: ['Modifications de la bibliothèque', 'Examinez les modifications et conflits en attente avant de les appliquer.'],
      ja: ['ライブラリの変更', '適用する前に保留中の変更と競合を確認します。'],
      zh: ['库更改', '应用前审查待处理的更改和冲突。'],
    },
});
