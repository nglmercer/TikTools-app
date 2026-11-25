import { defineTour } from '../model/define-tour.ts';

export default defineTour({
  id: 'points',
  version: 1,
  route: '/points',
  scope: 'points',
  task: 'points.reviewed',
  tags: ['points'],
  icon: 'coins',
  shape: 'custom',
  layout: [
    { id: 'rules', target: 'points-rules', policy: 'cancel' },
    { id: 'leaderboard', target: 'points-leaderboard', policy: 'cancel' },
  ],
  steps: {
    rules: {
      es: ['Revisa reglas de puntos', 'Inspecciona los valores antes de modificarlos. Algunos controles se bloquean durante una sesión LIVE conectada u ocupada.'],
      en: ['Review earning rules', 'Inspect the configured values before changing them. Some controls are locked during a connected or busy LIVE session.'],
      pt: ['Revise as regras de pontos', 'Confira os valores antes de alterá-los. Alguns controles ficam bloqueados durante uma LIVE conectada ou ocupada.'],
      fr: ['Examiner les règles de points', 'Vérifiez les valeurs avant modification. Certains contrôles sont verrouillés pendant une session LIVE connectée ou occupée.'],
      ja: ['獲得ルールを確認', '変更する前に、設定値を確認してください。LIVEに接続中、または処理中は、一部の操作がロックされます。'],
      zh: ['审查积分获取规则', '更改前请检查已配置的值。直播会话已连接或繁忙时，部分控件会锁定。'],
    },
    leaderboard: {
      es: ['Inspecciona puntos confirmados', 'Busca u ordena espectadores. Actualizar la vista no ajusta ni reinicia puntos, tampoco repite eventos LIVE.'],
      en: ['Inspect committed points', 'Search or sort viewers. A display refresh is not a points adjustment, reset or replay of LIVE events.'],
      pt: ['Confira pontos confirmados', 'Pesquise ou ordene espectadores. Atualizar a exibição não ajusta nem redefine pontos e não repete eventos LIVE.'],
      fr: ['Examiner les points confirmés', 'Recherchez ou triez les spectateurs. Actualiser l’affichage ne modifie pas les points et ne rejoue aucun événement LIVE.'],
      ja: ['確定済みポイントを確認', '視聴者を検索・並べ替えできます。表示を更新しても、ポイントの調整やリセット、LIVEイベントの再実行は行いません。'],
      zh: ['检查已提交的积分', '搜索或排序观众。刷新显示不会调整或重置积分，也不会重放直播事件。'],
    },
  },
  card: {
      es: ['Puntos', 'Revisa las reglas de puntos y la clasificación confirmada.'],
      en: ['Points', 'Review earning rules and the confirmed leaderboard.'],
      pt: ['Pontos', 'Revise as regras de pontos e a classificação confirmada.'],
      fr: ['Points', 'Examinez les règles de points et le classement confirmé.'],
      ja: ['ポイント', '獲得ルールと確定したランキングを確認します。'],
      zh: ['积分', '审查积分规则和已确认的排行榜。'],
    },
});
