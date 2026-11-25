import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'diagnostics',
  version: 1,
  route: '/debug',
  scope: 'diagnostics',
  task: 'diagnostics.inspected',
  tags: ['diagnostics'],
  icon: 'chart',
  shape: 'custom',
  layout: [
    { id: 'log', target: 'diagnostics-log', policy: 'cancel' },
    { id: 'back', target: 'diagnostics-actions', policy: 'skip' },
  ],
  steps: {
    log: {
      es: ['Inspecciona el historial acotado', 'Filtra origen, nivel o texto y revisa repeticiones. Comprueba exportaciones ocultadas antes de compartir; el historial dura solo la sesión.'],
      en: ['Inspect bounded history', 'Filter by source, level or text and inspect repeated entries. Review redacted exports before sharing; the history is session-only.'],
      pt: ['Inspecione o histórico limitado', 'Filtre origem, nível ou texto e confira repetições. Revise exportações ocultadas antes de compartilhar; o histórico dura só a sessão.'],
      fr: ['Examiner l’historique limité', 'Filtrez source, niveau ou texte et examinez les répétitions. Vérifiez les exports expurgés avant partage ; l’historique dure seulement la session.'],
      ja: ['保持範囲内の履歴を確認', 'ソース、レベル、テキストで絞り込み、繰り返された記録を確認してください。共有前に、機密情報をマスクしたエクスポートを確認してください。履歴はこのセッション内のみです。'],
      zh: ['检查有限历史', '按来源、级别或文本筛选，并检查重复条目。分享前请审查经过敏感信息遮蔽的导出内容；历史仅限当前会话。'],
    },
    back: {
      es: ['Revisa evidencia y recupera', 'Copia o exporta evidencia revisada y vuelve al flujo original ante conflictos, sesiones vencidas u órdenes inciertas. Borrar registros no repara el error.'],
      en: ['Review evidence and recover', 'Copy or export reviewed evidence, then use the original flow for conflicts, expired sessions or uncertain commands. Clearing logs does not repair the error.'],
      pt: ['Revise evidências e recupere', 'Copie ou exporte evidências revisadas e use o fluxo original para conflitos, sessões expiradas e comandos incertos. Limpar registros não corrige o erro.'],
      fr: ['Examiner les preuves et récupérer', 'Copiez ou exportez les preuves examinées, puis utilisez la récupération d’origine pour conflits, sessions expirées ou commandes incertaines. Effacer les journaux ne répare pas leur cause.'],
      ja: ['証拠を確認して復旧', '確認済みの証拠をコピーまたはエクスポートし、競合、セッションの期限切れ、不確かなコマンドには元の手順で対処してください。ログを消去してもエラーは修復されません。'],
      zh: ['审查证据并恢复', '复制或导出已审查的证据，再使用原始流程处理冲突、会话过期或不确定的命令。清空日志不会修复错误。'],
    },
  },
  card: {
      es: ['Diagnóstico', 'Inspecciona el registro de la sesión y exporta evidencia.'],
      en: ['Diagnostics', 'Inspect the session log and export evidence.'],
      pt: ['Diagnóstico', 'Inspecione o registro da sessão e exporte evidências.'],
      fr: ['Diagnostic', 'Inspectez le journal de session et exportez des preuves.'],
      ja: ['診断', 'セッションログを検査し、証拠をエクスポートします。'],
      zh: ['诊断', '检查会话日志并导出证据。'],
    },
});
