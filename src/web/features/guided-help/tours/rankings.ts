import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'rankings',
  version: 1,
  route: '/rankings',
  scope: 'rankings',
  tags: ['rankings', 'points'],
  icon: 'trophy',
  shape: 'custom',
  layout: [
    { id: 'tabs', target: 'rankings-tabs', policy: 'cancel' },
    { id: 'list', target: 'rankings-list', policy: 'skip' },
    { id: 'detail', target: 'rankings-detail', policy: 'skip' },
    { id: 'competitions', target: 'rankings-competitions', policy: 'skip' },
  ],
  steps: {
    tabs: {
      es: ['Rankings o competiciones', 'Los rankings acumulan puntuaciones. Las competiciones comparan rankings seleccionados durante un evento limitado.'],
      en: ['Rankings or competitions', 'Rankings collect scores. Competitions compare selected rankings during a bounded event.'],
      pt: ['Rankings ou competições', 'Rankings acumulam pontuações. Competições comparam rankings selecionados durante um evento limitado.'],
      fr: ['Classements ou compétitions', 'Les classements accumulent des scores. Les compétitions comparent des classements sélectionnés pendant un événement limité.'],
      ja: ['ランキングまたは競技', 'ランキングはスコアを集計します。競技は期間限定イベントで選択したランキングを比較します。'],
      zh: ['排行榜或竞赛', '排行榜累计分数。竞赛在限定活动期间比较所选排行榜。'],
    },
    list: {
      es: ['Elige o crea un ranking', 'La lista muestra métrica, modo y estado. Crea un ranking solo después de decidir qué acción del espectador puntúa.'],
      en: ['Choose or create a ranking', 'The list shows metric, mode and status. Create a new ranking only after deciding what viewer action should score.'],
      pt: ['Escolha ou crie um ranking', 'A lista mostra métrica, modo e estado. Crie um novo ranking somente depois de decidir qual ação do espectador deve pontuar.'],
      fr: ['Choisissez ou créez un classement', 'La liste montre métrique, mode et état. Créez un classement seulement après avoir décidé quelle action du spectateur doit marquer des points.'],
      ja: ['ランキングを選択または作成', '一覧には指標、モード、状態が表示されます。どの視聴者アクションを得点にするか決めてから作成します。'],
      zh: ['选择或创建排行榜', '列表显示指标、模式和状态。只有决定哪种观众行为计分后才创建新排行榜。'],
    },
    detail: {
      es: ['Inspecciona el marcador', 'El detalle muestra puntuación, pausa e integridad. Pausar, reanudar y reiniciar son operaciones explícitas.'],
      en: ['Inspect the live scoreboard', 'A detail view shows score, pause state and integrity. Pause, resume and reset are explicit operations.'],
      pt: ['Inspecione o placar ao vivo', 'A visão de detalhe mostra pontuação, estado de pausa e integridade. Pausar, retomar e reiniciar são operações explícitas.'],
      fr: ['Inspectez le tableau des scores', 'La vue détaillée montre score, état de pause et intégrité. Pause, reprise et réinitialisation sont des opérations explicites.'],
      ja: ['ライブスコアボードを確認', '詳細ビューにはスコア、一時停止状態、整合性が表示されます。一時停止、再開、リセットは明示的な操作です。'],
      zh: ['检查实时记分板', '详情视图显示分数、暂停状态和完整性。暂停、恢复和重置都是明确操作。'],
    },
    competitions: {
      es: ['Configura competiciones', 'Las competiciones necesitan rankings compatibles y límites claros. Son experimentales; verifica el overlay antes de usarlas en directo.'],
      en: ['Configure competitions', 'Competitions need compatible rankings and clear boundaries. They are experimental, so verify the overlay before using them on air.'],
      pt: ['Configure competições', 'Competições precisam de rankings compatíveis e limites claros. Elas são experimentais; verifique o overlay antes de usá-las no ar.'],
      fr: ['Configurez les compétitions', 'Les compétitions exigent des classements compatibles et des limites claires. Elles sont expérimentales ; vérifiez l’overlay avant de les utiliser à l’antenne.'],
      ja: ['競技を設定', '競技には互換性のあるランキングと明確な境界が必要です。実験的機能なので、配信で使う前にオーバーレイを確認します。'],
      zh: ['配置竞赛', '竞赛需要兼容的排行榜和清晰边界。它们仍是实验性功能，直播使用前请验证悬浮层。'],
    },
  },
  card: {
      es: ['Rankings y competiciones', 'Crea rankings, inspecciona puntuaciones y configura competiciones.'],
      en: ['Rankings and competitions', 'Create rankings, inspect scores and configure competitions.'],
      pt: ['Rankings e competições', 'Crie rankings, inspecione pontuações e configure competições.'],
      fr: ['Classements et compétitions', 'Créez des classements, inspectez les scores et configurez les compétitions.'],
      ja: ['ランキングと競技', 'ランキング作成、スコア確認、競技設定を行います。'],
      zh: ['排行榜与竞赛', '创建排行榜、检查分数并配置竞赛。'],
    },
});
