import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'automationEventSocial',
  version: 1,
  route: '/automations',
  scope: 'automations',
  tags: ['events'],
  icon: 'heart',
  shape: 'event',
  steps: {
    eventsTab: {
      es: ['Interacción social como disparador', 'Follows, likes y compartidos son eventos de alto volumen: ideales para agradecimientos ligeros y metas de engagement.'],
      en: ['Social interaction as a trigger', 'Follows, likes and shares are high-volume events: ideal for light thank-yous and engagement goals.'],
      pt: ['Interação social como gatilho', 'Follows, likes e compartilhamentos são eventos de alto volume: ideais para agradecimentos leves.'],
      fr: ['L’interaction sociale', 'Follows, likes et partages sont des événements à haut volume : parfaits pour des remerciements légers.'],
      ja: ['ソーシャルをトリガーに', 'フォロー・いいね・シェアは高ボリュームのイベント: 軽いお礼やエンゲージメント目標に最適です。'],
      zh: ['社交互动触发器', '关注、点赞和分享是高频率事件: 适合轻量感谢和互动目标。'],
    },
    events: {
      es: ['Encuentra tus reglas sociales', 'Las filas sociales suelen compartir acciones de overlay. Editar la acción actualiza todos los eventos que la vinculan.'],
      en: ['Find your social rules', 'Social rows often share overlay actions. Editing the action updates every event that links it.'],
      pt: ['Encontre suas regras sociais', 'Linhas sociais costumam compartilhar ações de overlay. Editar a ação atualiza todos os eventos vinculados.'],
      fr: ['Trouvez vos règles sociales', 'Les lignes sociales partagent souvent des actions d’overlay. Modifier l’action met à jour tous les événements liés.'],
      ja: ['ソーシャルのルールを確認', 'ソーシャルの行は overlay アクションを共有することが多く、アクションの編集は全イベントに反映されます。'],
      zh: ['找到社交规则', '社交行常共享 overlay 操作,编辑操作会更新所有关联事件。'],
    },
    trigger: {
      es: ['Elige follow, like o share', 'tiktok.follow, tiktok.like y tiktok.share tienen datos distintos: los likes acumulan conteo y los follows identifican al espectador.'],
      en: ['Pick follow, like or share', 'tiktok.follow, tiktok.like and tiktok.share carry different data: likes accumulate count and follows identify the viewer.'],
      pt: ['Escolha follow, like ou share', 'tiktok.follow, tiktok.like e tiktok.share têm dados diferentes: likes acumulam contagem e follows identificam o espectador.'],
      fr: ['Choisissez follow, like ou share', 'tiktok.follow, tiktok.like et tiktok.share portent des données différentes : les likes cumulent un compte.'],
      ja: ['follow・like・share を選択', 'tiktok.follow/like/share はデータが異なります: いいねはカウントを蓄積し、フォローは視聴者を特定します。'],
      zh: ['选择 follow、like 或 share', '三者的数据不同: 点赞累积计数,关注标识观众。'],
    },
    conditions: {
      es: ['Umbrales para alto volumen', 'Los likes llegan en ráfagas: filtra por conteo mínimo (por ejemplo cada 100) para no saturar el directo con efectos.'],
      en: ['Thresholds for high volume', 'Likes arrive in bursts: filter by minimum count (say every 100) so the stream is not flooded with effects.'],
      pt: ['Limiares para alto volume', 'Likes chegam em rajadas: filtre por contagem mínima (por exemplo cada 100) para não saturar a live.'],
      fr: ['Seuils pour haut volume', 'Les likes arrivent en rafales : filtrez par compte minimal (par exemple tous les 100) pour ne pas saturer le direct.'],
      ja: ['高ボリューム用のしきい値', 'いいねはバーストで届きます: 最小カウント(例: 100ごと)で絞り込み、配信の過剰な演出を防ぎます。'],
      zh: ['高频率的阈值', '点赞成群到来: 按最小计数过滤(如每100),避免效果刷屏。'],
    },
  },
  card: {
      es: ['Eventos sociales', 'Follows, likes y shares con umbrales para alto volumen.'],
      en: ['Social events', 'Follows, likes and shares with high-volume thresholds.'],
      pt: ['Eventos sociais', 'Follows, likes e shares com limiares de alto volume.'],
      fr: ['Événements sociaux', 'Follows, likes et partages avec seuils de volume.'],
      ja: ['ソーシャルイベント', 'フォロー・いいね・シェアと高ボリュームのしきい値。'],
      zh: ['社交事件', '关注、点赞、分享及高频率阈值。'],
    },
});
