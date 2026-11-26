import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationActionHttp',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['actions'],
  icon: 'globe',
  shape: 'action',
  requirements: { automationActionType: { typeId: 'core.fetch' } },
  steps: {
    actionsTab: {
      es: ['Acciones webhook (HTTP)', 'Las acciones http.request llaman a servicios externos: Discord, hojas de cálculo o tus propias integraciones.'],
      en: ['Webhook actions (HTTP)', 'http.request actions call external services: Discord, spreadsheets or your own integrations.'],
      pt: ['Ações webhook (HTTP)', 'Ações http.request chamam serviços externos: Discord, planilhas ou suas próprias integrações.'],
      fr: ['Actions webhook (HTTP)', 'Les actions http.request appellent des services externes : Discord, feuilles de calcul ou vos intégrations.'],
      ja: ['Webhookアクション (HTTP)', 'http.request アクションは外部サービスを呼び出します: Discord、スプレッドシート、独自の連携。'],
      zh: ['webhook 操作 (HTTP)', 'http.request 调用外部服务: Discord、表格或你自己的集成。'],
    },
    actions: {
      es: ['Encuentra tus webhooks', 'Cada fila es una llamada HTTP reutilizable. Compartir una sola acción entre eventos evita duplicar URLs y cabeceras.'],
      en: ['Find your webhooks', 'Each row is a reusable HTTP call. Sharing one action across events avoids duplicating URLs and headers.'],
      pt: ['Encontre seus webhooks', 'Cada linha é uma chamada HTTP reutilizável. Compartilhar uma ação entre eventos evita duplicar URLs.'],
      fr: ['Trouvez vos webhooks', 'Chaque ligne est un appel HTTP réutilisable. Partager une action évite de dupliquer URL et en-têtes.'],
      ja: ['Webhookを確認', '各行は再利用可能なHTTP呼び出しです。共有すればURLやヘッダの重複を避けられます。'],
      zh: ['找到 webhook', '每行是可复用的 HTTP 调用,共享可避免重复 URL 和头。'],
    },
    settings: {
      es: ['Configura la llamada', 'Define método, URL, cabeceras y cuerpo. Guarda los tokens como variables secretas: nunca los escribas directamente en la acción.'],
      en: ['Configure the call', 'Set method, URL, headers and body. Store tokens as secret variables: never write them directly into the action.'],
      pt: ['Configure a chamada', 'Defina método, URL, cabeçalhos e corpo. Guarde tokens como variáveis secretas, nunca diretamente na ação.'],
      fr: ['Configurez l’appel', 'Définissez méthode, URL, en-têtes et corps. Stockez les jetons en variables secrètes, jamais directement dans l’action.'],
      ja: ['呼び出しを設定', 'メソッド・URL・ヘッダ・ボディを定義します。トークンは秘密変数に保存し、アクションに直接書かないでください。'],
      zh: ['配置调用', '定义方法、URL、头和请求体。令牌存为秘密变量,不要直接写进操作。'],
    },
    permissions: {
      es: ['Revisa los permisos de red', 'Las llamadas salientes necesitan permiso de red hacia su destino. La lista muestra exactamente a qué hosts accede la acción.'],
      en: ['Review network permissions', 'Outbound calls need network permission for their destination. The list shows exactly which hosts the action reaches.'],
      pt: ['Revise permissões de rede', 'Chamadas externas precisam de permissão de rede. A lista mostra exatamente a quais hosts a ação acessa.'],
      fr: ['Vérifiez les permissions réseau', 'Les appels sortants exigent une permission réseau. La liste montre exactement les hôtes contactés.'],
      ja: ['ネットワーク権限を確認', '外部呼び出しは宛先へのネットワーク権限が必要です。アクセス先ホストが正確に表示されます。'],
      zh: ['检查网络权限', '外部调用需要目标的网络权限,列表精确显示访问的主机。'],
    },
  },
  card: {
      es: ['Acciones webhook (HTTP)', 'Llamadas a servicios externos con secretos seguros.'],
      en: ['Webhook actions (HTTP)', 'Calls to external services with safe secrets.'],
      pt: ['Ações webhook (HTTP)', 'Chamadas a serviços externos com segredos seguros.'],
      fr: ['Actions webhook (HTTP)', 'Appels vers des services externes, secrets sécurisés.'],
      ja: ['Webhookアクション (HTTP)', '安全なシークレットで外部サービスを呼び出し。'],
      zh: ['webhook 操作 (HTTP)', '安全 secrets 调用外部服务。'],
    },
});
