import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationEventRoom',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['events'],
  icon: 'zap',
  shape: 'event',
  steps: {
    eventsTab: {
      es: ['La sala como disparador', 'Entradas de espectadores, estadísticas de sala y cambios de conexión cubren el ciclo de vida del directo.'],
      en: ['The room as a trigger', 'Viewer joins, room stats and connection changes cover the stream lifecycle.'],
      pt: ['A sala como gatilho', 'Entradas de espectadores, estatísticas de sala e mudanças de conexão cobrem o ciclo de vida da live.'],
      fr: ['La salle comme déclencheur', 'Entrées de spectateurs, statistiques et changements de connexion couvrent le cycle du direct.'],
      ja: ['ルームをトリガーに', '視聴者の入室・ルーム統計・接続変化が配信のライフサイクルをカバーします。'],
      zh: ['房间作为触发器', '观众进入、房间统计和连接变化覆盖直播的生命周期。'],
    },
    events: {
      es: ['Encuentra tus reglas de sala', 'Los eventos de sala suelen ser pocos pero importantes: un mensaje de bienvenida o una alerta al conectar valen más que diez efectos.'],
      en: ['Find your room rules', 'Room events are few but important: one welcome message or a connect alert beats ten noisy effects.'],
      pt: ['Encontre suas regras de sala', 'Eventos de sala são poucos mas importantes: uma mensagem de boas-vindas vale mais que dez efeitos.'],
      fr: ['Trouvez vos règles de salle', 'Les événements de salle sont peu nombreux mais importants : une bienvenue vaut mieux que dix effets.'],
      ja: ['ルームのルールを確認', 'ルームイベントは少ないですが重要です: 歓迎メッセージ1件は10の演出に勝ります。'],
      zh: ['找到房间规则', '房间事件少而重要: 一条欢迎消息胜过十个效果。'],
    },
    trigger: {
      es: ['Elige join, stats o conexión', 'tiktok.join reacciona a cada entrada, room_stats a cambios de espectadores, y connected/disconnected al estado de la conexión LIVE.'],
      en: ['Pick join, stats or connection', 'tiktok.join reacts to each join, room_stats to viewer count changes, and connected/disconnected to the LIVE connection state.'],
      pt: ['Escolha join, stats ou conexão', 'tiktok.join reage a cada entrada, room_stats a mudanças de espectadores e connected/disconnected ao estado da conexão.'],
      fr: ['Choisissez join, stats ou connexion', 'tiktok.join réagit à chaque entrée, room_stats aux changements de spectateurs, connected/disconnected à l’état LIVE.'],
      ja: ['join・stats・接続を選択', 'tiktok.join は入室ごと、room_stats は視聴者数変化、connected/disconnected は接続状態に反応します。'],
      zh: ['选择 join、stats 或连接', 'tiktok.join 响应每次进入,room_stats 响应人数变化,connected/disconnected 响应连接状态。'],
    },
    conditions: {
      es: ['Filtra entradas relevantes', 'join llega muy seguido en salas grandes: filtra por rol o seguidor para que la bienvenida no se vuelva spam.'],
      en: ['Filter relevant joins', 'join fires constantly in big rooms: filter by role or follower status so the welcome does not become spam.'],
      pt: ['Filtre entradas relevantes', 'join dispara constantemente em salas grandes: filtre por papel ou seguidor para não virar spam.'],
      fr: ['Filtrez les entrées utiles', 'join se déclenche souvent dans les grandes salles : filtrez par rôle ou follower pour éviter le spam.'],
      ja: ['関連する入室を絞り込み', '大きなルームでは join が頻発します: ロールやフォロワーで絞り込みスパム化を防ぎます。'],
      zh: ['过滤相关进入', '大房间中 join 频繁触发: 按角色或粉丝过滤,避免刷屏。'],
    },
  },
  card: {
      es: ['Eventos de sala', 'Entradas, estadísticas y ciclo de vida de la conexión.'],
      en: ['Room events', 'Joins, stats and the connection lifecycle.'],
      pt: ['Eventos de sala', 'Entradas, estatísticas e ciclo de vida da conexão.'],
      fr: ['Événements de salle', 'Entrées, statistiques et cycle de la connexion.'],
      ja: ['ルームイベント', '入室、統計、接続のライフサイクル。'],
      zh: ['房间事件', '进入、统计和连接生命周期。'],
    },
});
