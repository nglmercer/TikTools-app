import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'backupRestore',
  version: 1,
  route: '/settings?section=account',
  scope: 'account',
  tags: ['account'],
  icon: 'history',
  shape: 'custom',
  layout: [
    { id: 'tools', target: 'backup-tools', policy: 'cancel' },
    { id: 'list', target: 'backup-list', policy: 'skip' },
    { id: 'plan', target: 'account-plan', policy: 'skip' },
  ],
  steps: {
    tools: {
      es: ['Elige la fuente', 'Las copias Cloud y los archivos locales son rutas de recuperación separadas. Elige la fuente antes de revisar qué cambiará.'],
      en: ['Choose the restore source', 'Cloud backups and local files are separate recovery paths. Pick the source before reviewing what will change.'],
      pt: ['Escolha a fonte de restauração', 'Backups Cloud e arquivos locais são caminhos separados de recuperação. Escolha a fonte antes de revisar o que vai mudar.'],
      fr: ['Choisissez la source de restauration', 'Les sauvegardes Cloud et les fichiers locaux sont des chemins de récupération séparés. Choisissez la source avant de vérifier ce qui changera.'],
      ja: ['復元元を選択', 'Cloudバックアップとローカルファイルは別の復旧経路です。変更内容を確認する前にソースを選びます。'],
      zh: ['选择恢复来源', 'Cloud 备份和本地文件是分开的恢复路径。检查将发生的变化前，先选择来源。'],
    },
    list: {
      es: ['Restaura deliberadamente', 'Cada copia puede descargarse, restaurarse o eliminarse. Restaurar siempre muestra una revisión antes de aplicarse.'],
      en: ['Restore deliberately', 'A backup row can download, restore or delete. Restore always shows a review of affected data before applying it.'],
      pt: ['Restaure deliberadamente', 'Uma linha de backup pode baixar, restaurar ou excluir. Restaurar sempre mostra uma revisão dos dados afetados antes de aplicar.'],
      fr: ['Restaurez délibérément', 'Une ligne de sauvegarde peut télécharger, restaurer ou supprimer. Restaurer montre toujours une révision des données affectées avant application.'],
      ja: ['慎重に復元', 'バックアップ行ではダウンロード、復元、削除ができます。復元は適用前に影響を受けるデータの確認画面を必ず表示します。'],
      zh: ['谨慎恢复', '备份行可以下载、恢复或删除。恢复前始终显示受影响数据的审核内容。'],
    },
    plan: {
      es: ['Verifica el resultado', 'Tras restaurar, revisa plan, perfiles, ajustes y automatizaciones clave. Una restauración correcta no revalida todas las credenciales externas.'],
      en: ['Verify the result', 'After restoration, check the plan, profiles, settings and key automations. A successful restore does not revalidate every external credential.'],
      pt: ['Verifique o resultado', 'Após a restauração, verifique plano, perfis, configurações e automações principais. Uma restauração bem-sucedida não revalida todas as credenciais externas.'],
      fr: ['Vérifiez le résultat', 'Après restauration, vérifiez plan, profils, paramètres et automatisations clés. Une restauration réussie ne revalide pas tous les identifiants externes.'],
      ja: ['結果を検証', '復元後はプラン、プロフィール、設定、主要な自動化を確認します。復元成功ですべての外部認証情報が再検証されるわけではありません。'],
      zh: ['验证结果', '恢复后检查套餐、配置文件、设置和关键自动化。成功恢复不会重新验证所有外部凭据。'],
    },
  },
  card: {
      es: ['Restaurar copias', 'Elige una fuente, revisa cambios y verifica la recuperación.'],
      en: ['Restore backups', 'Choose a backup source, review changes and verify recovery.'],
      pt: ['Restaurar backups', 'Escolha uma fonte de backup, revise alterações e verifique a recuperação.'],
      fr: ['Restaurer les sauvegardes', 'Choisissez une source, vérifiez les changements et confirmez la restauration.'],
      ja: ['バックアップを復元', '復元元を選び、変更を確認して復旧を検証します。'],
      zh: ['恢复备份', '选择备份来源、检查更改并验证恢复。'],
    },
});
