import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'pluginSetup',
  version: 1,
  route: '/plugins',
  scope: 'plugins',
  tags: ['plugins'],
  icon: 'download',
  shape: 'custom',
  layout: [
    { id: 'catalog', target: 'plugin-catalog', policy: 'cancel' },
    { id: 'details', target: 'plugin-details', policy: 'skip', prepare: ['[data-guide="plugin-tab-installed"]', '[data-plugin-key] .plugin-catalog__open'], prepareActions: ['plugins.openInstalled'] },
    { id: 'settings', target: 'plugin-settings', policy: 'skip', prepare: ['[data-guide="plugin-tab-settings"]'], prepareActions: ['plugins.showSettings'] },
    { id: 'connection', target: 'plugin-connection', policy: 'skip', prepare: ['[data-guide="plugin-tab-connection"]'], prepareActions: ['plugins.showConnection'] },
  ],
  steps: {
    catalog: {
      es: ['Abre el plugin instalado', 'Empieza en Instalados, no en Explorar, cuando configures un plugin ya confiable en esta máquina.'],
      en: ['Open the installed plugin', 'Start from Installed, not Explore, when configuring a plugin already trusted on this machine.'],
      pt: ['Abra o plugin instalado', 'Comece em Instalados, não em Explorar, ao configurar um plugin já confiável nesta máquina.'],
      fr: ['Ouvrez le plugin installé', 'Commencez dans Installed, pas Explore, pour configurer un plugin déjà approuvé sur cette machine.'],
      ja: ['インストール済みプラグインを開く', 'このマシンで信頼済みのプラグインを設定するときは、ExploreではなくInstalledから始めます。'],
      zh: ['打开已安装插件', '配置这台机器上已信任的插件时，从 Installed 而不是 Explore 开始。'],
    },
    details: {
      es: ['Revisa permisos y estado', 'La cabecera muestra activación, versión y acciones del runtime. Revisa permisos antes de activarlo.'],
      en: ['Review permissions and state', 'The details header shows enabled state, version and actions that affect the runtime. Review permissions before enabling.'],
      pt: ['Revise permissões e estado', 'O cabeçalho de detalhes mostra estado de ativação, versão e ações que afetam o runtime. Revise permissões antes de ativar.'],
      fr: ['Vérifiez permissions et état', 'L’en-tête des détails montre l’état d’activation, la version et les actions qui affectent le runtime. Vérifiez les permissions avant l’activation.'],
      ja: ['権限と状態を確認', '詳細ヘッダーには有効状態、バージョン、ランタイムに影響するアクションが表示されます。有効化前に権限を確認します。'],
      zh: ['检查权限和状态', '详情头部显示启用状态、版本以及影响运行时的操作。启用前先检查权限。'],
    },
    settings: {
      es: ['Configura ajustes', 'La pestaña Ajustes contiene valores específicos del plugin. Guarda primero cuando el plugin lo pida antes de probar.'],
      en: ['Configure settings', 'The Settings tab contains plugin-specific values. Save first when the plugin asks before testing.'],
      pt: ['Configure os ajustes', 'A aba Configurações contém valores específicos do plugin. Salve primeiro quando o plugin pedir antes de testar.'],
      fr: ['Configurez les réglages', 'L’onglet Settings contient les valeurs propres au plugin. Enregistrez d’abord lorsque le plugin le demande avant de tester.'],
      ja: ['設定を構成', 'Settingsタブにはプラグイン固有の値があります。プラグインが求める場合はテスト前に保存します。'],
      zh: ['配置设置', 'Settings 标签页包含插件专属值。如果插件要求，测试前先保存。'],
    },
    connection: {
      es: ['Conecta y prueba', 'La pestaña Conexión gestiona credenciales externas y ofrece una acción de prueba cuando el plugin la proporciona.'],
      en: ['Connect and test', 'The Connection tab manages external service credentials and offers a test action when the plugin provides one.'],
      pt: ['Conecte e teste', 'A aba Conexão gerencia credenciais de serviços externos e oferece uma ação de teste quando o plugin a fornece.'],
      fr: ['Connectez et testez', 'L’onglet Connection gère les identifiants de services externes et propose une action de test lorsque le plugin en fournit une.'],
      ja: ['接続してテスト', 'Connectionタブは外部サービス認証情報を管理し、プラグインが提供する場合はテストアクションを表示します。'],
      zh: ['连接并测试', 'Connection 标签页管理外部服务凭据，并在插件提供时显示测试操作。'],
    },
  },
  card: {
      es: ['Configurar un plugin', 'Permisos, ajustes, conexión y acciones de prueba.'],
      en: ['Configure a plugin', 'Permissions, settings, connection and test actions.'],
      pt: ['Configurar um plugin', 'Permissões, ajustes, conexão e ações de teste.'],
      fr: ['Configurer un plugin', 'Autorisations, réglages, connexion et actions de test.'],
      ja: ['プラグインを設定', '権限、設定、接続、テストアクションを確認します。'],
      zh: ['配置插件', '权限、设置、连接和测试操作。'],
    },
});
