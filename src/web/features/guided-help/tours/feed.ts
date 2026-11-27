import { defineTour } from '../model/define-tour.ts';
import { guideScope } from '../model/guide-vocabulary.ts';

export default defineTour({
  id: 'feed',
  version: 2,
  route: '/feed',
  scope: guideScope.feed,
  tags: ['feed'],
  icon: 'chat',
  shape: 'custom',
  layout: [
    { id: 'view', target: 'feed-view', policy: 'cancel' },
    { id: 'filters', target: 'feed-filters', policy: 'skip' },
    { id: 'stage', target: 'feed-stage', policy: 'skip', version: 2 },
    { id: 'controls', target: 'feed-controls', policy: 'skip' },
  ],
  steps: {
    view: {
      es: ['Elige la vista del feed', 'Feed en vivo muestra chat, regalos y seguidores como tarjetas; cambia a Lista para ver el registro compacto.'],
      en: ['Choose the feed view', 'Live feed shows chat, gifts and follows as cards; switch to List for the compact log.'],
      pt: ['Escolha a visualização do feed', 'Feed ao vivo mostra chat, presentes e seguidores como cartões; mude para Lista para ver o registro compacto.'],
      fr: ['Choisir la vue du flux', 'Le flux en direct affiche chat, cadeaux et abonnements sous forme de cartes ; passez en Liste pour le journal compact.'],
      ja: ['フィード表示を選ぶ', 'ライブフィードはチャット、ギフト、フォローをカードで表示します。「リスト」に切り替えると簡潔なログになります。'],
      zh: ['选择 Feed 视图', '实时 Feed 以卡片显示聊天、礼物和关注；切换到“列表”可查看紧凑日志。'],
    },
    filters: {
      es: ['Filtra la actividad', 'Filtra por tipo o busca por espectador y mensaje. Los filtros también afectan a las tarjetas del feed.'],
      en: ['Filter the activity', 'Narrow by kind or search by viewer and message. Filters also apply to the live feed cards.'],
      pt: ['Filtre a atividade', 'Filtre por tipo ou busque por espectador e mensagem. Os filtros também se aplicam aos cartões do feed.'],
      fr: ['Filtrer l’activité', 'Filtrez par type ou recherchez un spectateur ou un message. Les filtres s’appliquent aussi aux cartes du flux.'],
      ja: ['アクティビティを絞り込む', '種類で絞り込むか、視聴者やメッセージで検索します。フィルターはフィードのカードにも適用されます。'],
      zh: ['筛选活动', '按类型筛选或按观众和消息搜索。筛选同样适用于 Feed 卡片。'],
    },
    stage: {
      es: ['Lee el muro en vivo', 'Los mensajes importantes permanecen. Los me gusta y las entradas aparecen un momento como reacciones y luego se desvanecen.'],
      en: ['Read the live wall', 'Important messages stay on the wall. Likes and joins appear briefly as reactions, then fade out.'],
      pt: ['Leia o mural ao vivo', 'Mensagens importantes permanecem. Curtidas e entradas aparecem brevemente como reações e depois desaparecem.'],
      fr: ['Lire le mur en direct', 'Les messages importants restent affichés. Les j’aime et les arrivées apparaissent brièvement comme réactions, puis disparaissent.'],
      ja: ['ライブウォールを読む', '重要なメッセージは残ります。いいねや参加はリアクションとして一時的に表示され、その後消えます。'],
      zh: ['阅读实时墙', '重要消息会保留。点赞和加入会短暂显示为互动提示，然后淡出。'],
    },
    controls: {
      es: ['Pausar, limpiar e inspeccionar', 'Pausa el desplazamiento para leer con calma, limpia el historial guardado o abre el inspector para ver detalles.'],
      en: ['Pause, clear and inspect', 'Pause auto-scroll to read at your own pace, clear the saved history, or open the inspector for details.'],
      pt: ['Pausar, limpar e inspecionar', 'Pause a rolagem para ler com calma, limpe o histórico salvo ou abra o inspetor para ver detalhes.'],
      fr: ['Pause, effacement et inspection', 'Mettez le défilement en pause, effacez l’historique enregistré ou ouvrez l’inspecteur pour plus de détails.'],
      ja: ['一時停止・クリア・検査', '自動スクロールを一時停止して落ち着いて読む、保存済み履歴を消す、またはインスペクターで詳細を確認できます。'],
      zh: ['暂停、清空与查看', '暂停自动滚动以便阅读，清空已保存的历史记录，或打开检查器查看详情。'],
    },
  },
  card: {
      es: ['Feed en vivo', 'Chat, regalos y reacciones que llegan de la sala LIVE.'],
      en: ['Live feed', 'Chat, gifts and reactions arriving from the live room.'],
      pt: ['Feed ao vivo', 'Chat, presentes e reações recebidos da sala LIVE.'],
      fr: ['Flux en direct', 'Chat, cadeaux et réactions reçus de la salle LIVE.'],
      ja: ['ライブフィード', 'LIVE ルームから届くチャット、ギフト、リアクション。'],
      zh: ['实时 Feed', '来自 LIVE 房间的聊天、礼物和互动。'],
    },
});
