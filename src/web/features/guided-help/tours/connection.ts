import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'connection',
  version: 1,
  route: '/connections',
  scope: 'connections',
  task: 'connection.connected',
  tags: ['connection'],
  icon: 'link',
  shape: 'custom',
  layout: [
    { id: 'form', target: 'connection-form', policy: 'cancel' },
    { id: 'explorer', target: 'connection-explorer', policy: 'skip' },
    { id: 'plugins', target: 'connection-plugins', policy: 'skip' },
  ],
  steps: {
    form: {
      es: ['Elige un creador', 'Introduce su nombre y revisa el estado. La guía nunca conecta ni lee tu cookie de sesión.'],
      en: ['Choose a creator', 'Enter the creator handle and review connection status. The guide never connects or reads your session cookie.'],
      pt: ['Escolha um criador', 'Informe o nome e confira o estado. O guia nunca conecta nem lê seu cookie de sessão.'],
      fr: ['Choisir un créateur', 'Saisissez son nom et vérifiez la connexion. Le guide ne connecte jamais le compte et ne lit aucun cookie de session.'],
      ja: ['クリエイターを選択', 'クリエイターのユーザー名を入力し、接続状態を確認してください。このガイドが接続したり、セッションCookieを読み取ったりすることはありません。'],
      zh: ['选择主播', '输入主播用户名并审查连接状态。指南绝不会进行连接，也不会读取你的会话 Cookie。'],
    },
    explorer: {
      es: ['Explora salas LIVE', 'Actualiza o elige una sala por tu cuenta. La conexión demo es simulada; el acceso nativo depende del host y de TikTok.'],
      en: ['Explore LIVE rooms', 'Refresh or choose a room yourself. A successful demo connection is simulated; native access depends on the host and TikTok.'],
      pt: ['Explore salas LIVE', 'Atualize ou escolha uma sala manualmente. A conexão da demonstração é simulada; o acesso nativo depende do host e do TikTok.'],
      fr: ['Explorer les salles LIVE', 'Actualisez ou choisissez une salle vous-même. La connexion de démonstration est simulée ; le mode natif dépend du serveur local et de TikTok.'],
      ja: ['LIVEルームを探す', 'ルームの更新や選択は、自分で行ってください。デモでの接続成功はシミュレーションです。ネイティブ環境でのアクセスは、ホストとTikTokに依存します。'],
      zh: ['探索直播间', '请自行刷新或选择直播间。演示中的成功连接是模拟的；原生访问取决于主机和 TikTok。'],
    },
    plugins: {
      es: ['Comprueba integraciones', 'Revisa cada conexión instalada y su salud antes de habilitar efectos. Abrir la guía no contacta servicios de plugins.'],
      en: ['Check integrations', 'Review each installed connection and its health before enabling effects. Opening this guide does not contact a plugin service.'],
      pt: ['Confira as integrações', 'Revise cada conexão instalada e sua saúde antes de ativar efeitos. Abrir o guia não contata serviços de plugins.'],
      fr: ['Vérifier les intégrations', 'Examinez chaque connexion installée avant les effets. Ouvrir le guide ne contacte aucun service de plugin.'],
      ja: ['連携を確認', '効果を有効にする前に、インストール済みの各接続と動作状態を確認してください。このガイドを開いても、プラグインのサービスには接続しません。'],
      zh: ['检查集成', '启用效果前，请审查每个已安装连接及其健康状态。打开本指南不会联系插件服务。'],
    },
  },
  card: {
      es: ['Conectar un creador', 'Comprueba el creador, las salas LIVE y las integraciones instaladas.'],
      en: ['Connect a creator', 'Check the creator, live rooms and installed integrations.'],
      pt: ['Conectar um criador', 'Verifique o criador, as salas LIVE e as integrações instaladas.'],
      fr: ['Connecter un créateur', 'Vérifiez le créateur, les salles LIVE et les intégrations installées.'],
      ja: ['クリエイターに接続', 'クリエイター、LIVE ルーム、インストール済み連携を確認します。'],
      zh: ['连接创作者', '检查创作者、LIVE 房间和已安装的集成。'],
    },
});
