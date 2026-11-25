import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'settings',
  version: 1,
  route: '/settings',
  scope: 'settings',
  tags: ['settings'],
  icon: 'gear',
  shape: 'custom',
  layout: [
    { id: 'sections', target: 'settings-sections', policy: 'cancel' },
    { id: 'language', target: 'settings-language', policy: 'skip' },
    { id: 'review', target: 'settings-review', policy: 'skip' },
    { id: 'updater', target: 'settings-updater', policy: 'skip' },
  ],
  steps: {
    sections: {
      es: ['Elige una sección', 'General contiene preferencias de la aplicación. Cuenta contiene perfil Cloud, plan y copias de seguridad.'],
      en: ['Choose a settings section', 'General contains application preferences. Account contains the cloud profile, plan and backup tools.'],
      pt: ['Escolha uma seção de configurações', 'Geral contém preferências do aplicativo. Conta contém perfil Cloud, plano e ferramentas de backup.'],
      fr: ['Choisissez une section de paramètres', 'General contient les préférences de l’application. Account contient le profil Cloud, le plan et les outils de sauvegarde.'],
      ja: ['設定セクションを選択', 'Generalにはアプリ設定、AccountにはCloudプロフィール、プラン、バックアップツールがあります。'],
      zh: ['选择设置分区', 'General 包含应用偏好设置。Account 包含 Cloud 个人资料、套餐和备份工具。'],
    },
    language: {
      es: ['Cambia el idioma', 'El idioma se aplica inmediatamente a la interfaz. No cambia datos del LIVE ni el comportamiento de automatizaciones.'],
      en: ['Change the language', 'The language applies immediately to the interface. It does not change LIVE data or automation behavior.'],
      pt: ['Mude o idioma', 'O idioma é aplicado imediatamente à interface. Ele não altera dados da LIVE nem o comportamento das automações.'],
      fr: ['Changez la langue', 'La langue s’applique immédiatement à l’interface. Elle ne change ni les données du LIVE ni le comportement des automatisations.'],
      ja: ['言語を変更', '言語はインターフェースにすぐ適用されます。LIVEデータや自動化の動作は変わりません。'],
      zh: ['更改语言', '语言会立即应用到界面。它不会更改 LIVE 数据或自动化行为。'],
    },
    review: {
      es: ['Elige el modo de revisión', 'El modo de revisión controla cuánta confirmación piden los flujos sensibles de sincronización. Los modos más seguros exigen revisión explícita.'],
      en: ['Choose the review mode', 'Review mode controls how much confirmation sensitive synchronization flows ask for. Safer modes require more explicit review.'],
      pt: ['Escolha o modo de revisão', 'O modo de revisão controla quanta confirmação os fluxos sensíveis de sincronização exigem. Modos mais seguros exigem revisão mais explícita.'],
      fr: ['Choisissez le mode de révision', 'Le mode de révision contrôle le niveau de confirmation demandé par les flux de synchronisation sensibles. Les modes plus sûrs exigent une révision plus explicite.'],
      ja: ['レビューモードを選択', 'レビューモードは、機密性の高い同期フローが求める確認の量を制御します。安全なモードほど明示的な確認が必要です。'],
      zh: ['选择审核模式', '审核模式控制敏感同步流程需要多少确认。更安全的模式需要更明确的审核。'],
    },
    updater: {
      es: ['Busca actualizaciones', 'Usa la tarjeta del actualizador para ver el canal y comprobar actualizaciones. Si no está disponible, usa la ruta de recuperación documentada.'],
      en: ['Check updates manually', 'Use the updater card to see the channel and check for updates. If updates are unavailable, download the release through the documented recovery path.'],
      pt: ['Verifique atualizações manualmente', 'Use o cartão do atualizador para ver o canal e procurar atualizações. Se as atualizações não estiverem disponíveis, baixe a versão pelo caminho de recuperação documentado.'],
      fr: ['Vérifiez les mises à jour manuellement', 'Utilisez la carte de mise à jour pour voir le canal et rechercher des mises à jour. Si elles sont indisponibles, téléchargez la version par le chemin de récupération documenté.'],
      ja: ['更新を手動で確認', 'アップデーターカードでチャンネル確認と更新チェックができます。更新を利用できない場合は、文書化された復旧手順でリリースをダウンロードします。'],
      zh: ['手动检查更新', '使用更新器卡片查看通道并检查更新。如果无法更新，请按照文档中的恢复路径下载版本。'],
    },
  },
  card: {
      es: ['Configuración', 'Idioma, modo de revisión y actualizaciones de la aplicación.'],
      en: ['Settings', 'Language, review mode and application updates.'],
      pt: ['Configurações', 'Idioma, modo de revisão e atualizações do aplicativo.'],
      fr: ['Paramètres', 'Langue, mode de révision et mises à jour de l’application.'],
      ja: ['設定', '言語、レビューモード、アプリの更新を管理します。'],
      zh: ['设置', '语言、审核模式和应用更新。'],
    },
});
